use super::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use vpt_adapters::SqliteLedger;
use vpt_application::ports::{RecordingLedger, RunHistory};
use vpt_domain::history::{EventDetail, EventKind, EventState, SafeFailureClass};
use vpt_domain::run::{HistoryFact, RunOutcome};

#[test]
fn refused_store_creation_records_safe_history_and_alert_after_state_validation() {
    assert_refused_store(false);
}

#[test]
fn config_refusal_preserves_exit_status_when_export_and_alert_fail() {
    assert_refused_store(true);
}

fn assert_refused_store(fail_delivery: bool) {
    let sandbox = configured();
    fixture::seed(&sandbox, vec![]);
    add(&sandbox, "untouched.m4a");
    let parent = sandbox.path().join("external");
    fs::create_dir(&parent).unwrap();
    let hook = sandbox.path().join("bin/alert");
    let captured = sandbox.path().join("alert.json");
    fs::write(
        &hook,
        format!(
            "#!/bin/sh\n/bin/cat > \"$1\"\nprintf 'private hook bytes' >&2\nexit {}\n",
            if fail_delivery { 7 } else { 0 }
        ),
    )
    .unwrap();
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o700)).unwrap();
    let config = fs::read_to_string(sandbox.config_path()).unwrap();
    fs::write(sandbox.config_path(), format!("{config}\n[stores]\naudio = {:?}\n[alert]\ncommand = [{hook:?}, {captured:?}]\n[report]\ncommand = ['/usr/bin/touch', {:?}]\n", parent.join("audio"), sandbox.path().join("report-called"))).unwrap();
    let log_path = sandbox.path().join("state/vpt/events.jsonl");
    if fail_delivery {
        fs::create_dir(&log_path).unwrap();
    }
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o500)).unwrap();
    let before = snapshot(&sandbox);
    let dry = run(sandbox.vpt().args(["ingest", "--dry-run", "--json"]));
    assert_eq!(dry.status.code(), Some(0), "{}", stderr(&dry));
    assert_eq!(snapshot(&sandbox), before);
    let output = run(sandbox.vpt().args(["ingest", "--json"]));
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
    assert!(output.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(error["error"]["kind"], "config");
    assert_eq!(error["error"]["completed"], serde_json::json!([]));
    assert!(!parent.join("audio").exists());
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    assert!(ledger.recordings().unwrap().is_empty());
    let history = ledger.recovery_view().unwrap();
    let refusals: Vec<_> = history
        .records
        .iter()
        .filter_map(|record| match &record.fact {
            HistoryFact::Event(event) if event.kind == EventKind::ConfigRefused => Some(event),
            _ => None,
        })
        .collect();
    assert_eq!(refusals.len(), 1);
    assert_eq!(refusals[0].state, EventState::Failed);
    assert_eq!(
        refusals[0].detail,
        EventDetail::Failure(SafeFailureClass::ConfigurationRefused)
    );
    assert!(refusals[0].paths.is_empty());
    assert!(
        history
            .records
            .iter()
            .any(|record| matches!(record.fact, HistoryFact::RunFinished(RunOutcome::Failed)))
    );
    assert_eq!(ledger.pending_exports().unwrap().is_empty(), !fail_delivery);
    let alert: serde_json::Value = serde_json::from_slice(&fs::read(captured).unwrap()).unwrap();
    assert_eq!(alert["schema"], "vpt.event/1");
    assert_eq!(alert["channel"], "alert");
    assert_eq!(alert["event"], "config_refused");
    assert!(!sandbox.path().join("report-called").exists());
    if fail_delivery {
        let text = stderr(&output);
        assert!(text.contains("event log export is unavailable"));
        assert!(text.contains("configured notification command failed"));
        assert!(!text.contains("private hook bytes"));
        assert_eq!(
            history
                .records
                .iter()
                .filter(|record| matches!(record.fact, HistoryFact::HookFailed { .. }))
                .count(),
            1
        );
    } else {
        let log = fs::read_to_string(log_path).unwrap();
        assert_eq!(
            log.lines()
                .filter(|line| line.contains("\"event\":\"config_refused\""))
                .count(),
            1
        );
        assert!(!log.contains("private audio"));
    }
}

#[test]
fn config_refusal_before_root_validation_never_writes_history_or_calls_hooks() {
    for seeded in [false, true] {
        let sandbox = configured();
        if seeded {
            fixture::seed(&sandbox, vec![]);
        }
        let marker = sandbox.path().join("alert-called");
        let config = fs::read_to_string(sandbox.config_path()).unwrap();
        fs::write(sandbox.config_path(), format!("{config}\n[stores]\naudio = {:?}\n[alert]\ncommand = ['/usr/bin/touch', {marker:?}]\n", sandbox.path().join("missing-parent/audio"))).unwrap();
        let before = snapshot(&sandbox);
        let output = run(sandbox.vpt().args(["ingest", "--json"]));
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let error: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
        assert_eq!(error["error"]["kind"], "config");
        assert!(stderr(&output).contains("parent directory does not exist"));
        assert_eq!(snapshot(&sandbox), before);
        assert!(!marker.exists());
    }
}
