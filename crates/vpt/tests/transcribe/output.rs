use super::*;
use std::time::Duration;

#[test]
fn output_failure_preserves_accepted_transcription_and_note_without_private_text() {
    for dry in [false, true] {
        let (sandbox, row) = configured();
        let before = fixture::snapshot(&sandbox);
        let mut command = sandbox.vpt();
        command.args(["transcribe", row.id.as_str(), "--json"]);
        if dry {
            command.arg("--dry-run");
        }
        let output = support::run_with_broken_stdout(&mut command, Duration::from_secs(5));
        assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
        assert!(output.stdout.is_empty());
        assert!(!stderr(&output).contains("private transcript canary"));
        let document: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
        assert_eq!(document["error"]["message"], "standard output write failed");
        if dry {
            assert_eq!(document["error"]["completed"], serde_json::json!([]));
            assert_eq!(fixture::snapshot(&sandbox), before);
        } else {
            assert_eq!(
                document["error"]["completed"],
                serde_json::json!([row.id.as_str()])
            );
            let ledger = ledger(&sandbox);
            assert!(ledger.accepted_transcript(&row.id).unwrap().is_some());
            let saved = ledger.by_id(&row.id).unwrap().unwrap();
            assert_eq!(saved.stages.transcribe, StageState::Succeeded);
            assert_eq!(saved.stages.note, StageState::Succeeded);
            assert!(ledger.pending_publications().unwrap().is_empty());
        }
    }
}
