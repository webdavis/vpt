use super::*;
use std::time::Duration;

#[test]
fn output_failure_reports_each_committed_recording_once_and_unchanged_retry_claims_none() {
    let sandbox = configured();
    add(&sandbox, "one.m4a", 1_787_604_456);
    add(&sandbox, "two.m4a", 1_787_604_476);
    for first in [true, false] {
        let output = support::run_with_broken_stdout(
            sandbox.vpt().args(["run", "--json"]),
            Duration::from_secs(8),
        );
        assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
        assert!(output.stdout.is_empty());
        assert!(!stderr(&output).contains("Private words"));
        let error: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
        assert_eq!(error["error"]["message"], "standard output write failed");
        let ledger = ledger(&sandbox);
        let rows = ledger.recordings().unwrap();
        assert_eq!(rows.len(), 2);
        let mut expected = Vec::new();
        for row in rows {
            assert_eq!(row.stages.transcribe, StageState::Succeeded);
            assert_eq!(row.stages.note, StageState::Succeeded);
            let note = ledger
                .note(&row.id, NoteStage::Transcript)
                .unwrap()
                .unwrap();
            assert_eq!(fs::read_to_string(note.target).unwrap(), note.text);
            if first {
                expected.push(row.id.to_string());
            }
        }
        let mut actual: Vec<String> =
            serde_json::from_value(error["error"]["completed"].clone()).unwrap();
        actual.sort();
        expected.sort();
        assert_eq!(actual, expected);
    }
    assert_eq!(
        fs::read_to_string(sandbox.path().join("calls"))
            .unwrap()
            .lines()
            .count(),
        2
    );
}

#[test]
fn dry_run_output_failure_does_not_claim_or_perform_planned_work() {
    let sandbox = configured();
    add(&sandbox, "one.m4a", 1_787_604_456);
    let before = fixture::snapshot(&sandbox);
    let output = support::run_with_broken_stdout(
        sandbox.vpt().args(["run", "--json", "--dry-run"]),
        Duration::from_secs(8),
    );
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    let error: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(error["error"]["message"], "standard output write failed");
    assert_eq!(error["error"]["completed"], serde_json::json!([]));
    assert_eq!(fixture::snapshot(&sandbox), before);
}
