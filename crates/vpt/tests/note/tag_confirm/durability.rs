use super::*;

pub(super) fn sql(sandbox: &Sandbox, statement: &str) {
    let connection = rusqlite::Connection::open(sandbox.path().join("state/vpt/vpt.db")).unwrap();
    fixture::preserve_wal(&connection);
    connection.execute_batch(statement).unwrap();
}

#[test]
fn export_failure_keeps_tag_success_and_reports_a_safe_warning() {
    let (sandbox, row, path) = configured_tags();
    fs::create_dir(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
    let output = confirm(&sandbox, row.id.as_str(), "--tag", "Private topic");
    let result = success(&output);
    assert!(!result["warnings"].as_array().unwrap().is_empty());
    assert_eq!(fs::read_to_string(path).unwrap(), "private-topic\n");
    assert!(!stdout(&output).contains("private-topic"));
    assert_eq!(saved(&sandbox, &row).managed_tags, ["private-topic"]);
}

#[test]
fn output_failure_preserves_confirmed_tag_and_vocabulary_without_leaking_tag_text() {
    let (sandbox, row, path) = configured_tags();
    let output = support::run_with_broken_stdout(
        sandbox.vpt().args([
            "confirm",
            row.id.as_str(),
            "--tag",
            "Private topic",
            "--json",
        ]),
        std::time::Duration::from_secs(5),
    );
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(output.stdout.is_empty());
    assert!(!stderr(&output).contains("private-topic"));
    assert!(!stderr(&output).contains("Private topic"));
    let document: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(document["error"]["message"], "standard output write failed");
    assert_eq!(
        document["error"]["completed"],
        serde_json::json!([row.id.as_str()])
    );
    assert_eq!(fs::read_to_string(path).unwrap(), "private-topic\n");
    assert_eq!(saved(&sandbox, &row).managed_tags, ["private-topic"]);
}

#[test]
fn malformed_note_prevents_both_tag_and_vocabulary_commit() {
    let (sandbox, row, path) = configured_tags();
    success(&invoke(&sandbox, row.id.as_str()));
    let note = saved(&sandbox, &row);
    let edited = note.text.replace("<!-- vpt:content end -->", "removed");
    assert_ne!(edited, note.text);
    fs::write(&note.target, &edited).unwrap();
    let output = confirm(&sandbox, row.id.as_str(), "--tag", "private");
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    assert_eq!(saved(&sandbox, &row), note);
    assert_eq!(fs::read_to_string(&note.target).unwrap(), edited);
    assert!(!path.exists());
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    assert_eq!(ledger.tags(&row.id).unwrap(), None);
    assert!(ledger.pending_publications().unwrap().is_empty());
}

#[test]
fn a_late_history_failure_rolls_back_tag_note_and_vocabulary_together() {
    let (sandbox, row, path) = configured_tags();
    success(&invoke(&sandbox, row.id.as_str()));
    let note = saved(&sandbox, &row);
    sql(
        &sandbox,
        "CREATE TRIGGER refuse_tag_history BEFORE INSERT ON history_records WHEN instr(NEW.fact, '\"tag\":\"work\"') > 0 BEGIN SELECT RAISE(ABORT, 'Private canary'); END;",
    );
    let output = confirm(&sandbox, row.id.as_str(), "--tag", "private");
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(!stderr(&output).contains("Private canary"));
    assert_eq!(saved(&sandbox, &row), note);
    assert_eq!(fs::read_to_string(&note.target).unwrap(), note.text);
    assert!(!path.exists());
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    assert_eq!(ledger.tags(&row.id).unwrap(), None);
    assert!(ledger.pending_publications().unwrap().is_empty());
}

#[test]
fn failed_acknowledgement_reports_committed_recording_and_retry_repairs_it() {
    let (sandbox, row, path) = configured_tags();
    sql(
        &sandbox,
        "CREATE TRIGGER refuse_clear BEFORE DELETE ON dirty_publications BEGIN SELECT RAISE(ABORT, 'Private canary'); END;",
    );
    let output = confirm(&sandbox, row.id.as_str(), "--tag", "private");
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    let error: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(
        error["error"]["completed"],
        serde_json::json!([row.id.as_str()])
    );
    let before = tags(&sandbox, &row);
    sql(&sandbox, "DROP TRIGGER refuse_clear");
    success(&confirm(&sandbox, row.id.as_str(), "--tag", "private"));
    assert_eq!(tags(&sandbox, &row), before);
    assert_eq!(fs::read_to_string(path).unwrap(), "private\n");
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    assert!(ledger.pending_publications().unwrap().is_empty());
}
