use super::*;
use vpt_domain::run::HookFailureClass;

fn run(case: &refusal::Fixture, verb: &str) -> std::process::Output {
    support::run_with_budget(
        case.sandbox.vpt().args(case.args(verb)).arg("--json"),
        if verb == "run" {
            Duration::from_secs(8)
        } else {
            Duration::from_secs(5)
        },
    )
}

#[test]
fn export_and_hook_failure_preserve_configuration_result_for_all_run_lifecycles() {
    for verb in ["run", "brief", "transcribe"] {
        let case = refusal::Fixture::new();
        let log = case.sandbox.path().join("state/vpt/events.jsonl");
        fs::rename(&log, case.sandbox.path().join("held-events.jsonl")).unwrap();
        fs::create_dir(&log).unwrap();
        fs::write(case.sandbox.path().join("bin/alert"), "#!/bin/sh\n/bin/cat >> \"$1\"\nprintf '\\n' >> \"$1\"\nprintf 'PRIVATE child failure' >&2\nexit 7\n").unwrap();
        let output = run(&case, verb);
        assert_eq!(output.status.code(), Some(2), "{verb}: {}", stderr(&output));
        assert!(output.stdout.is_empty());
        let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"]["kind"], "config");
        let diagnostics = error["error"]["diagnostics"].as_array().unwrap();
        assert!(diagnostics.contains(&serde_json::json!("event log export is unavailable")));
        assert!(diagnostics.contains(&serde_json::json!("configured notification command failed")));
        assert!(!stderr(&output).contains("PRIVATE child failure"));
        let ledger = case.ledger();
        let history = ledger.recovery_view().unwrap();
        assert_eq!(
            history
                .records
                .iter()
                .filter(|record| matches!(
                    record.fact,
                    HistoryFact::HookFailed {
                        failure: HookFailureClass::Exited(7),
                        ..
                    }
                ))
                .count(),
            1
        );
        assert!(!ledger.pending_exports().unwrap().is_empty());
        let alerts = fs::read_to_string(case.sandbox.path().join("alerts.jsonl")).unwrap();
        assert_eq!(alerts.lines().count(), 1);
        assert!(!case.sandbox.path().join("report-called").exists());
        assert_eq!(
            fs::read_to_string(case.sandbox.path().join("engine-calls")).unwrap(),
            "called\n"
        );
    }
}

#[test]
fn required_configuration_event_commit_failure_stops_without_hook_delivery() {
    for verb in ["run", "brief", "transcribe"] {
        let case = refusal::Fixture::new();
        let db = rusqlite::Connection::open(case.sandbox.path().join("state/vpt/vpt.db")).unwrap();
        fixture::preserve_wal(&db);
        db.execute_batch("CREATE TRIGGER refuse_configuration_event BEFORE INSERT ON history_records WHEN NEW.fact LIKE '%config_refused%' BEGIN SELECT RAISE(ABORT, 'PRIVATE history failure'); END").unwrap();
        drop(db);
        let output = run(&case, verb);
        assert_eq!(output.status.code(), Some(1), "{verb}: {}", stderr(&output));
        assert!(output.stdout.is_empty());
        let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"]["kind"], "ledger");
        assert_eq!(error["error"]["completed"], serde_json::json!([]));
        assert!(!stderr(&output).contains("PRIVATE history failure"));
        assert!(!case.sandbox.path().join("alerts.jsonl").exists());
        assert!(!case.sandbox.path().join("report-called").exists());
        assert!(!case.parent.join("drafts").exists());
        assert_eq!(
            fs::read_to_string(case.sandbox.path().join("engine-calls")).unwrap(),
            "called\n"
        );
        let history = case.ledger().recovery_view().unwrap();
        assert!(!history.records.iter().any(|record| matches!(&record.fact, HistoryFact::Event(event) if event.kind == EventKind::ConfigRefused)));
    }
}
