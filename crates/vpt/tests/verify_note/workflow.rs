use super::*;

fn sql(sandbox: &Sandbox, sql: &str) {
    let connection = rusqlite::Connection::open(sandbox.path().join("state/vpt/vpt.db")).unwrap();
    fixture::preserve_wal(&connection);
    connection.execute_batch(sql).unwrap();
}
fn owned() -> (Sandbox, RecordingRecord, NoteArtifact) {
    let (sandbox, record) = note::configured();
    success(&note::invoke(&sandbox, record.id.as_str()));
    let artifact = note::saved(&sandbox, &record);
    (sandbox, record, artifact)
}

#[test]
fn unchanged_claim_keeps_its_correction_and_changed_claim_starts_open() {
    let (sandbox, record, artifact) = owned();
    success(&invoke(&sandbox, &artifact.target, record.id.as_str()));
    let first = ledger(&sandbox)
        .review_snapshot(&record.id)
        .unwrap()
        .unwrap()
        .flags;
    assert_eq!(first.len(), 1);
    sql(
        &sandbox,
        "UPDATE flags SET state='corrected', resolution_text='Private correction', resolved_at=90 WHERE shape='verification'",
    );
    success(&invoke(&sandbox, &artifact.target, record.id.as_str()));
    let corrected = note::saved(&sandbox, &record);
    assert!(corrected.text.contains("[corrected: Private correction]"));
    assert!(corrected.text.contains("vptOpenFlags: 0\n"));
    success(&invoke(&sandbox, &artifact.target, record.id.as_str()));
    assert_eq!(note::saved(&sandbox, &record).text, corrected.text);
    let edited = corrected
        .text
        .replace("um private words", "different claim");
    fs::write(&artifact.target, edited).unwrap();
    success(&invoke(&sandbox, &artifact.target, record.id.as_str()));
    let after = ledger(&sandbox)
        .review_snapshot(&record.id)
        .unwrap()
        .unwrap()
        .flags;
    assert_ne!(after[0].identifier, first[0].identifier);
    assert_eq!(after[0].resolution, vpt_domain::review::Resolution::Open);
    assert!(
        !note::saved(&sandbox, &record)
            .text
            .contains("Private correction")
    );
}

#[test]
fn late_transaction_failure_preserves_prior_note_findings_and_publication_state() {
    let (sandbox, record, artifact) = owned();
    sql(
        &sandbox,
        "CREATE TRIGGER refuse_note BEFORE UPDATE ON owned_notes BEGIN SELECT RAISE(ABORT, 'Private canary'); END",
    );
    let output = invoke(&sandbox, &artifact.target, record.id.as_str());
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(output.stdout.is_empty());
    assert!(!stderr(&output).contains("Private"));
    assert_eq!(note::saved(&sandbox, &record), artifact);
    assert_eq!(fs::read_to_string(&artifact.target).unwrap(), artifact.text);
    assert!(
        ledger(&sandbox)
            .review_snapshot(&record.id)
            .unwrap()
            .unwrap()
            .flags
            .is_empty()
    );
    assert!(ledger(&sandbox).pending_publications().unwrap().is_empty());
}

#[test]
fn retry_repairs_committed_publication_without_losing_findings() {
    let (sandbox, record, artifact) = owned();
    sql(
        &sandbox,
        "CREATE TRIGGER refuse_clear BEFORE DELETE ON dirty_publications BEGIN SELECT RAISE(ABORT, 'Private canary'); END",
    );
    let output = invoke(&sandbox, &artifact.target, record.id.as_str());
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    let error: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(
        error["error"]["completed"],
        serde_json::json!([record.id.as_str()])
    );
    let flags = ledger(&sandbox)
        .review_snapshot(&record.id)
        .unwrap()
        .unwrap()
        .flags;
    assert_eq!(flags.len(), 1);
    assert!(!ledger(&sandbox).pending_publications().unwrap().is_empty());
    sql(&sandbox, "DROP TRIGGER refuse_clear");
    success(&invoke(&sandbox, &artifact.target, record.id.as_str()));
    assert!(ledger(&sandbox).pending_publications().unwrap().is_empty());
    assert_eq!(
        ledger(&sandbox)
            .review_snapshot(&record.id)
            .unwrap()
            .unwrap()
            .flags,
        flags
    );
    let events = fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
    assert!(events.contains("verify-note"));
    assert!(!events.contains("private words"));
}

#[test]
fn log_export_failure_preserves_verification_success_and_reports_a_safe_warning() {
    let (sandbox, record, artifact) = owned();
    let log = sandbox.path().join("state/vpt/events.jsonl");
    fs::remove_file(&log).unwrap();
    let outside = sandbox.path().join("operator.log");
    fs::write(&outside, "Operator bytes").unwrap();
    std::os::unix::fs::symlink(&outside, &log).unwrap();
    let result = success(&invoke(&sandbox, &artifact.target, record.id.as_str()));
    assert!(
        result["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| warning == "event log export is unavailable")
    );
    assert!(note::saved(&sandbox, &record).text.contains("[unsourced]"));
    assert_eq!(fs::read_to_string(outside).unwrap(), "Operator bytes");
}
