use super::*;
use crate::spawn::{Outcome, Status};
use std::{
    cell::Cell,
    time::{Duration, Instant},
};
use vpt_domain::brief::ContextStatus;

fn output(stdout: Vec<u8>, stderr: Vec<u8>) -> Outcome {
    Outcome {
        status: Status::Exited(0),
        stdout,
        stderr,
        group: 0,
    }
}
fn request<'a>(calendars: &'a [String], projects: &'a [String]) -> ContextRequest<'a> {
    collection_tests::request(calendars, projects)
}

#[test]
fn all_queries_share_one_deadline_and_keep_a_completed_half() {
    let clock = Cell::new(Instant::now());
    let mut budgets = Vec::new();
    let snapshot = collect::run(
        &["dam".into()],
        &request(&["calendar/".into()], &["one/".into(), "two/".into()]),
        |_, budget| {
            budgets.push(budget.as_secs());
            clock.set(clock.get() + Duration::from_secs(5));
            Ok(output(b"{\"objects\":[]}".to_vec(), vec![]))
        },
        || clock.get(),
    )
    .unwrap();
    assert_eq!(budgets, vec![30, 25, 20, 15, 10, 5]);
    assert_eq!(snapshot.status.calendar, ContextStatus::Present);
    assert!(
        matches!(snapshot.status.tasks,ContextStatus::Failed {ref detail} if detail == Failure::Deadline.detail())
    );
}

#[test]
fn pretty_printed_error_stderr_and_provider_scope_escape_fail_the_half() {
    let mut row = tests::task();
    row["path"] = serde_json::json!("outside/");
    for (stdout, stderr) in [
        (
            b"{\"objects\":[]}".to_vec(),
            b"{\n  \"error\": {\"message\": \"PRIVATE\"}\n}".to_vec(),
        ),
        (tests::bytes(vec![row]), vec![]),
    ] {
        let snapshot = collect::run(
            &["dam".into()],
            &ContextRequest {
                window: None,
                tasks: true,
                calendars: &[],
                projects: &["work/".into()],
            },
            |_, _| Ok(output(stdout.clone(), stderr.clone())),
            Instant::now,
        )
        .unwrap();
        assert!(matches!(
            snapshot.status.tasks,
            ContextStatus::Failed { .. }
        ));
        assert!(snapshot.tasks.is_empty());
        assert!(!format!("{snapshot:?}").contains("PRIVATE"));
    }
}

#[test]
fn overlapping_scopes_deduplicate_equal_rows_and_refuse_conflicting_versions() {
    for conflict in [false, true] {
        let mut calls = 0;
        let snapshot = collect::run(
            &["dam".into()],
            &ContextRequest {
                window: None,
                tasks: true,
                calendars: &[],
                projects: &["work/".into(), "work/team/".into()],
            },
            |_, _| {
                let mut row = tests::task();
                if calls > 0 && conflict {
                    row["subject"] = serde_json::json!("changed");
                }
                calls += 1;
                Ok(output(tests::bytes(vec![row]), vec![]))
            },
            Instant::now,
        )
        .unwrap();
        assert_eq!(snapshot.tasks.len(), if conflict { 0 } else { 1 });
        assert_eq!(snapshot.status.tasks == ContextStatus::Present, !conflict);
    }
}

#[test]
fn calendar_results_use_exact_inclusive_start_and_exclusive_end() {
    let mut rows = Vec::new();
    for (id, at) in [
        ("early", "2026-10-01T22:59:59.999Z"),
        ("start", "2026-10-01T23:00:00Z"),
        ("end", "2026-10-02T01:00:00Z"),
    ] {
        let mut row = tests::event();
        row["oid"] = serde_json::json!(id);
        row["event"]["start"] = serde_json::json!(at);
        row["event"]["end"] = serde_json::json!("2026-10-02T02:00:00Z");
        rows.push(row);
    }
    let mut request = request(&[], &[]);
    let calendars = ["calendar/".into()];
    request.calendars = &calendars;
    request.tasks = false;
    let snapshot = collect::run(
        &["dam".into()],
        &request,
        |_, _| Ok(output(tests::bytes(rows.clone()), vec![])),
        Instant::now,
    )
    .unwrap();
    assert_eq!(snapshot.occasions.len(), 1);
    assert_eq!(snapshot.occasions[0].id, "start");
}

#[test]
fn aggregate_overflow_still_returns_a_valid_bounded_context_document() {
    let mut event = tests::event();
    event["event"]["start"] = serde_json::json!("2026-10-01T23:00:00Z");
    event["event"]["end"] = serde_json::json!("2026-10-02T00:00:00Z");
    let (mut low, mut high) = (1usize, 65_536usize);
    while low < high {
        let size = (low + high).div_ceil(2);
        event["subject"] = serde_json::json!("x".repeat(size));
        if decode::read(&tests::bytes(vec![event.clone()]), true, "calendar/").is_ok() {
            low = size;
        } else {
            high = size - 1;
        }
    }
    event["subject"] = serde_json::json!("x".repeat(low));
    let snapshot = collect::run(
        &["dam".into()],
        &request(&["calendar/".into()], &["work/".into()]),
        |argv, _| {
            Ok(output(
                tests::bytes(vec![if argv[2].starts_with("kind:event") {
                    event.clone()
                } else {
                    tests::task()
                }]),
                vec![],
            ))
        },
        Instant::now,
    )
    .unwrap();
    assert!(vpt_protocol::context::encode(&snapshot).is_ok());
    assert!(matches!(
        snapshot.status.tasks,
        ContextStatus::Failed { .. }
    ));
}

#[test]
fn interruption_aborts_collection_instead_of_becoming_publishable_absence() {
    for elapsed in [Duration::ZERO, Duration::from_secs(31)] {
        let clock = Cell::new(Instant::now());
        let result = collect::run(
            &["dam".into()],
            &request(&["calendar/".into()], &["work/".into()]),
            |_, _| {
                clock.set(clock.get() + elapsed);
                Ok(Outcome {
                    status: Status::Interrupted,
                    ..output(vec![], vec![])
                })
            },
            || clock.get(),
        );
        assert_eq!(result, Err(ContextError::Interrupted));
    }
}

#[test]
fn excessive_query_plan_is_failed_without_spawning_or_retaining_unbounded_text() {
    let large = format!("{}/", "x".repeat(60_000));
    let calendars = [large];
    let mut request = request(&calendars, &[]);
    request.tasks = false;
    request.window.as_mut().unwrap().end.secs += 100 * 86_400;
    let mut calls = 0;
    let snapshot = collect::run(
        &["dam".into()],
        &request,
        |_, _| {
            calls += 1;
            Ok(output(b"{\"objects\":[]}".to_vec(), vec![]))
        },
        Instant::now,
    )
    .unwrap();
    assert_eq!(calls, 0);
    assert!(matches!(
        snapshot.status.calendar,
        ContextStatus::Failed { .. }
    ));
}
