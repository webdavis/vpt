use super::*;
use vpt_application::ports::SinkWarning;

#[test]
fn empty_hook_configuration_still_exports_each_saved_record() {
    let fixture = Fixture::new();
    let receipt = fixture.saved_event(EventKind::BriefWritten, EventState::Done);
    let outcome = sink(&fixture, &[], &[]).after_commit(&receipt);

    assert_eq!(
        outcome,
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    let documents = fixture.documents();
    assert_eq!(documents.len(), 2);
    assert_eq!(documents[0]["fact"]["tag"], "run_started");
    assert_eq!(documents[1]["fact"]["tag"], "event");
    assert_eq!(documents[1]["event_id"], receipt.records[0].id.render());
    assert_eq!(documents[1]["occurred_at"], "1970-01-01T00:00:11Z");
    assert_eq!(documents[1]["appended_at"], "1970-01-01T00:00:20Z");
    assert!(fixture.history.pending_exports().unwrap().is_empty());
}

#[test]
fn literal_argv_and_single_json_stdin_reach_only_the_selected_hook() {
    let fixture = Fixture::new();
    let script = fixture.path("hook-fixture");
    write_hook(&script);
    let report_capture = fixture.path("report-capture");
    let alert_capture = fixture.path("alert-capture");
    let canary = fixture.path("interpolation-canary");
    let metacharacter = format!("$(touch {})", canary.display());
    let report = command(
        &script,
        &report_capture,
        &fixture.log_path(),
        "ok",
        &[
            "space containing argument",
            "{literal}",
            "雪",
            &metacharacter,
        ],
    );
    let alert = command(
        &script,
        &alert_capture,
        &fixture.log_path(),
        "ok",
        &["alert-only"],
    );
    let mut sink = sink(&fixture, &report, &alert);

    let report_receipt = fixture.saved_event(EventKind::BriefWritten, EventState::Done);
    assert_eq!(
        sink.after_commit(&report_receipt),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    assert!(
        report_capture.with_extension("calls").exists(),
        "report hook was not invoked"
    );
    assert!(!alert_capture.with_extension("calls").exists());
    let args = fs::read(report_capture.with_extension("argv")).unwrap();
    assert_eq!(
        args,
        [
            "space containing argument",
            "{literal}",
            "雪",
            &metacharacter
        ]
        .join("\0")
        .as_bytes()
        .iter()
        .copied()
        .chain([0])
        .collect::<Vec<_>>()
    );
    let input: Value =
        serde_json::from_slice(&fs::read(report_capture.with_extension("stdin")).unwrap()).unwrap();
    assert_eq!(input["schema"], "vpt.event/1");
    assert_eq!(input["channel"], "report");
    assert_eq!(input["event"], "brief_written");
    assert!(!canary.exists(), "hook arguments were interpolated");

    let alert_receipt = fixture.saved_event(EventKind::ReviewNeeded, EventState::NeedsAttention);
    assert_eq!(
        sink.after_commit(&alert_receipt),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    assert_eq!(
        fs::read(report_capture.with_extension("calls")).unwrap(),
        b"x"
    );
    assert_eq!(
        fs::read(alert_capture.with_extension("calls")).unwrap(),
        b"x"
    );
    let input: Value =
        serde_json::from_slice(&fs::read(alert_capture.with_extension("stdin")).unwrap()).unwrap();
    assert_eq!(input["channel"], "alert");
    assert_eq!(input["event"], "review_needed");
}

#[test]
fn confirmed_history_precedes_export_and_original_hook() {
    let fixture = Fixture::new();
    let script = fixture.path("ordered-hook");
    write_hook(&script);
    let capture = fixture.path("ordered-capture");
    let report = command(&script, &capture, &fixture.log_path(), "ok", &[]);
    let receipt = fixture.saved_event(EventKind::BriefWritten, EventState::Done);

    assert_eq!(
        sink(&fixture, &report, &[]).after_commit(&receipt),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    assert!(
        capture.with_extension("seenlog").exists(),
        "original hook ran before export"
    );
    let seen = fs::read_to_string(capture.with_extension("seenlog")).unwrap();
    assert!(seen.contains(&receipt.records[0].id.render()));
    let view = fixture.history.recovery_view().unwrap();
    assert!(
        view.records
            .iter()
            .all(|record| record.export_position.is_some())
    );
    assert_eq!(fixture.documents().len(), view.records.len());
}

#[test]
fn log_only_failure_preserves_work_and_runs_original_hook_once() {
    let fixture = Fixture::new();
    let script = fixture.path("failure-hook");
    write_hook(&script);
    let capture = fixture.path("failure-capture");
    let report = command(&script, &capture, &fixture.log_path(), "ok", &[]);
    fs::write(fixture.log_path(), b"retained\n").unwrap();
    fs::set_permissions(fixture.log_path(), fs::Permissions::from_mode(0o644)).unwrap();
    let receipt = fixture.saved_event(EventKind::BriefWritten, EventState::Done);

    let outcome = sink(&fixture, &report, &[]).after_commit(&receipt);
    assert_eq!(
        outcome,
        SinkOutcome::Continue {
            warnings: BTreeSet::from([SinkWarning::ExportUnavailable])
        }
    );
    assert_eq!(fs::read(capture.with_extension("calls")).unwrap(), b"x");
    assert_eq!(fs::read(fixture.log_path()).unwrap(), b"retained\n");
    assert_eq!(
        fs::metadata(fixture.log_path())
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o644
    );
    assert_eq!(fixture.history.pending_exports().unwrap().len(), 2);
    let view = fixture.history.recovery_view().unwrap();
    assert_eq!(view.records.last().unwrap().id, receipt.records[0].id);
}

#[test]
fn hook_failure_fact_exports_without_recursive_hook() {
    let fixture = Fixture::new();
    let script = fixture.path("failing-hook");
    write_hook(&script);
    let capture = fixture.path("failing-capture");
    let report = command(&script, &capture, &fixture.log_path(), "exit7", &[]);
    let receipt = fixture.saved_event(EventKind::BriefWritten, EventState::Done);

    let outcome = sink(&fixture, &report, &[]).after_commit(&receipt);
    assert_eq!(
        outcome,
        SinkOutcome::Continue {
            warnings: BTreeSet::from([SinkWarning::HookFailed])
        }
    );
    assert_eq!(fs::read(capture.with_extension("calls")).unwrap(), b"x");
    let documents = fixture.documents();
    assert_eq!(documents.len(), 3);
    assert_eq!(documents[2]["fact"]["tag"], "hook_failed");
    assert_eq!(
        documents[2]["fact"]["event"],
        receipt.records[0].id.render()
    );
    assert_eq!(
        documents[2]["fact"]["failure"],
        serde_json::json!({"tag":"exited","code":7})
    );
    assert!(
        !serde_json::to_string(&documents)
            .unwrap()
            .contains("child-private-sentinel")
    );
    assert!(fixture.history.pending_exports().unwrap().is_empty());

    let fixture = Fixture::new();
    let script = fixture.path("outage-hook");
    write_hook(&script);
    let capture = fixture.path("outage-capture");
    let report = command(&script, &capture, &fixture.log_path(), "exit7", &[]);
    fs::write(fixture.log_path(), b"retained\n").unwrap();
    fs::set_permissions(fixture.log_path(), fs::Permissions::from_mode(0o644)).unwrap();
    let receipt = fixture.saved_event(EventKind::BriefWritten, EventState::Done);

    let outcome = sink(&fixture, &report, &[]).after_commit(&receipt);
    assert_eq!(
        outcome,
        SinkOutcome::Continue {
            warnings: BTreeSet::from([SinkWarning::ExportUnavailable, SinkWarning::HookFailed])
        }
    );
    assert_eq!(fs::read(capture.with_extension("calls")).unwrap(), b"x");
    assert_eq!(fs::read(fixture.log_path()).unwrap(), b"retained\n");
    let view = fixture.history.recovery_view().unwrap();
    assert_eq!(view.failures.len(), 1, "blocked prefix was retried inline");
    assert_eq!(view.records.len(), 3);
    assert!(
        matches!(&view.records[2].fact, HistoryFact::HookFailed { event, .. } if *event == receipt.records[0].id)
    );
    assert_eq!(fixture.history.pending_exports().unwrap().len(), 3);
}
