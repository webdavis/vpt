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
fn answer(argv: &[String], events: &[serde_json::Value]) -> Outcome {
    match argv[1].as_str() {
        "agenda" => output(tests::agenda(events.to_vec()), vec![]),
        "show" => output(serde_json::to_vec(&tests::shown(&argv[2])).unwrap(), vec![]),
        _ => output(tests::bytes(vec![tests::task()]), vec![]),
    }
}
fn request<'a>(calendars: &'a [String], projects: &'a [String]) -> ContextRequest<'a> {
    collection_tests::request(calendars, projects)
}

#[test]
fn agenda_shows_and_task_queries_share_one_deadline_and_keep_a_completed_half() {
    let clock = Cell::new(Instant::now());
    let mut budgets = Vec::new();
    let mut events = Vec::new();
    for (id, at) in [
        ("one", "2026-10-01T23:00:00Z"),
        ("two", "2026-10-02T00:00:00Z"),
    ] {
        let mut row = tests::event();
        row["oid"] = serde_json::json!(id);
        row["start"] = serde_json::json!(at);
        row["end"] = serde_json::json!("2026-10-02T01:00:00Z");
        events.push(row);
    }
    let snapshot = collect::run(
        &["dam".into()],
        &[],
        &request(&["calendar/".into()], &["one/".into(), "two/".into()]),
        |argv, budget| {
            budgets.push(budget.as_secs());
            clock.set(clock.get() + Duration::from_secs(9));
            Ok(answer(argv, &events))
        },
        || clock.get(),
    )
    .unwrap();
    assert_eq!(budgets, vec![30, 21, 12, 3]);
    assert_eq!(snapshot.status.calendar, ContextStatus::Present);
    assert_eq!(snapshot.occasions.len(), 2);
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
            &[],
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
            &[],
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
        ("early", "2026-10-01T16:59:59.999-06:00"),
        ("start", "2026-10-01T17:00:00-06:00"),
        ("end", "2026-10-02T03:00:00+02:00"),
    ] {
        let mut row = tests::event();
        row["oid"] = serde_json::json!(id);
        row["start"] = serde_json::json!(at);
        row["end"] = serde_json::json!("2026-10-02T02:00:00Z");
        rows.push(row);
    }
    let mut shows = Vec::new();
    let mut request = request(&[], &[]);
    let calendars = ["calendar/".into()];
    request.calendars = &calendars;
    request.tasks = false;
    let snapshot = collect::run(
        &["dam".into()],
        &[],
        &request,
        |argv, _| {
            if argv[1] == "show" {
                shows.push(argv[2].clone());
            }
            Ok(answer(argv, &rows))
        },
        Instant::now,
    )
    .unwrap();
    assert_eq!(snapshot.occasions.len(), 1);
    assert_eq!(snapshot.occasions[0].id, "start");
    assert_eq!(shows, ["start"]);
}

#[test]
fn aggregate_overflow_still_returns_a_valid_bounded_context_document() {
    let mut event = tests::event();
    event["start"] = serde_json::json!("2026-10-01T23:00:00Z");
    event["end"] = serde_json::json!("2026-10-02T00:00:00Z");
    let collect = |event: &serde_json::Value, projects: &[String]| {
        let calendars = ["calendar/".into()];
        let request = request(&calendars, projects);
        let events = [event.clone()];
        collect::run(
            &["dam".into()],
            &[],
            &request,
            |argv, _| Ok(answer(argv, &events)),
            Instant::now,
        )
        .unwrap()
    };
    let (mut low, mut high) = (1usize, 65_536usize);
    while low < high {
        let size = (low + high).div_ceil(2);
        event["subject"] = serde_json::json!("x".repeat(size));
        if collect(&event, &["work/".into()]).status.calendar == ContextStatus::Present {
            low = size;
        } else {
            high = size - 1;
        }
    }
    event["subject"] = serde_json::json!("x".repeat(low));
    let snapshot = collect(&event, &["work/".into()]);
    assert!(vpt_protocol::context::encode(&snapshot).is_ok());
    assert_eq!(snapshot.status.calendar, ContextStatus::Present);
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
            &[],
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
fn an_empty_window_fails_the_calendar_without_spawning() {
    let calendars = ["calendar/".into()];
    let mut request = request(&calendars, &[]);
    request.tasks = false;
    let window = request.window.as_mut().unwrap();
    window.end = window.start;
    let mut calls = 0;
    let snapshot = collect::run(
        &["dam".into()],
        &[],
        &request,
        |_, _| {
            calls += 1;
            Ok(output(tests::agenda(vec![]), vec![]))
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
