use super::*;

#[test]
fn output_failure_preserves_retention_completions_but_dry_run_claims_none() {
    for dry in [false, true] {
        let sandbox = configured(true);
        sandbox.install_fake_helper();
        let row = expired_audio(&sandbox);
        let before = snapshot(&sandbox);
        let mut command = sandbox.vpt();
        command.args(["retention", "run", "--json"]);
        if dry {
            command.arg("--dry-run");
        }
        let output = support::run_with_broken_stdout(&mut command, Duration::from_secs(5));
        let document = error(&output, 1, "store");
        assert_eq!(document["error"]["message"], "standard output write failed");
        if dry {
            assert_eq!(document["error"]["completed"], serde_json::json!([]));
            assert_eq!(snapshot(&sandbox), before);
        } else {
            assert_eq!(
                document["error"]["completed"],
                serde_json::json!([row.id.as_str()])
            );
            assert!(!row.audio_path.exists());
            let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
                .unwrap()
                .unwrap();
            assert!(
                ledger
                    .by_id(&row.id)
                    .unwrap()
                    .unwrap()
                    .audio_trashed_at
                    .is_some()
            );
        }
    }
}
