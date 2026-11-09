use super::*;
use std::time::Duration;
use vpt_adapters::SqliteLedger;
use vpt_application::ports::{RecordingLedger, RetentionJournal};

fn fail_output(sandbox: &Sandbox, extra: &[&str]) -> serde_json::Value {
    let output = support::run_with_broken_stdout(
        sandbox.vpt().args(["ingest", "--json"]).args(extra),
        Duration::from_secs(5),
    );
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(output.stdout.is_empty());
    let document: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(document["error"]["message"], "standard output write failed");
    assert!(!stderr(&output).contains("private audio"));
    document
}

#[test]
fn output_failure_reports_new_ingestion_and_recovered_retention_but_not_old_work() {
    let sandbox = configured();
    sandbox.install_fake_helper();
    let retained = startup::pending_retention(&sandbox);
    let source = add(&sandbox, "new.m4a");
    let error = fail_output(&sandbox, &[]);
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    let rows = ledger.recordings().unwrap();
    assert_eq!(rows.len(), 2);
    let ingested = rows.iter().find(|row| row.id != retained.id).unwrap();
    assert_eq!(
        std::fs::read(&ingested.audio_path).unwrap(),
        std::fs::read(source).unwrap()
    );
    assert!(!retained.audio_path.exists());
    assert!(ledger.pending_intents().unwrap().is_empty());
    assert_eq!(
        error["error"]["completed"],
        serde_json::json!([ingested.id.as_str(), retained.id.as_str()])
    );
    assert_eq!(
        fail_output(&sandbox, &[])["error"]["completed"],
        serde_json::json!([])
    );
}

#[test]
fn output_failure_during_ingestion_dry_run_claims_no_commits_or_writes() {
    let sandbox = configured();
    add(&sandbox, "new.m4a");
    let before = snapshot(&sandbox);
    let error = fail_output(&sandbox, &["--dry-run"]);
    assert_eq!(error["error"]["completed"], serde_json::json!([]));
    assert_eq!(snapshot(&sandbox), before);
}
