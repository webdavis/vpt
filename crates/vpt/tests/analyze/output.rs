use super::*;

#[test]
fn output_failure_preserves_analysis_and_dry_run_claims_no_commits() {
    for dry in [true, false] {
        let (sandbox, record) = note::configured();
        agent(&sandbox, "");
        let before = fixture::snapshot(&sandbox);
        let mut command = sandbox.vpt();
        command.args(["analyze", record.id.as_str(), "--json"]);
        if dry {
            command.arg("--dry-run");
        }
        let output = support::run_with_broken_stdout(&mut command, Duration::from_secs(5));
        assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
        assert!(output.stdout.is_empty());
        assert!(!stderr(&output).contains("private"));
        let document: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
        assert_eq!(document["error"]["message"], "standard output write failed");
        if dry {
            assert_eq!(document["error"]["completed"], serde_json::json!([]));
            assert_eq!(fixture::snapshot(&sandbox), before);
        } else {
            assert_eq!(
                document["error"]["completed"],
                serde_json::json!([record.id.as_str()])
            );
            let ledger = ledger(&sandbox);
            assert!(ledger.accepted_proposal(&record.id).unwrap().is_some());
            let note = ledger
                .note(&record.id, NoteStage::Analysis)
                .unwrap()
                .unwrap();
            assert_eq!(fs::read_to_string(note.target).unwrap(), note.text);
            assert!(ledger.pending_publications().unwrap().is_empty());
        }
    }
}
