use super::*;

fn sql(sandbox: &Sandbox, statement: &str) {
    let connection = rusqlite::Connection::open(sandbox.path().join("state/vpt/vpt.db")).unwrap();
    fixture::preserve_wal(&connection);
    connection.execute_batch(statement).unwrap();
}

#[test]
fn malformed_notes_and_late_history_failures_leave_relations_and_note_unchanged() {
    for late_failure in [false, true] {
        let (sandbox, row) = configured();
        external(&sandbox);
        success(&invoke(&sandbox, row.id.as_str()));
        let before = saved(&sandbox, &row);
        let expected = if late_failure {
            sql(
                &sandbox,
                "CREATE TRIGGER refuse_relation_history BEFORE INSERT ON history_records WHEN instr(NEW.fact, '\"tag\":\"work\"') > 0 BEGIN SELECT RAISE(ABORT, 'Private canary'); END;",
            );
            before.text.clone()
        } else {
            let edited = before.text.replace("<!-- vpt:content end -->", "removed");
            fs::write(&before.target, &edited).unwrap();
            edited
        };
        let output = confirm(&sandbox, row.id.as_str(), "related:Alias");
        assert_eq!(
            output.status.code(),
            Some(if late_failure { 1 } else { 3 }),
            "{}",
            stderr(&output)
        );
        assert!(output.stdout.is_empty());
        assert!(!stderr(&output).contains("Private canary"));
        assert_eq!(saved(&sandbox, &row), before);
        assert_eq!(fs::read_to_string(before.target).unwrap(), expected);
        assert!(ledger(&sandbox).relations(&row.id).unwrap().is_empty());
        assert!(ledger(&sandbox).pending_publications().unwrap().is_empty());
    }
}

#[test]
fn committed_relations_survive_failed_acknowledgement_and_retry_repairs_publication() {
    let (sandbox, row) = configured();
    external(&sandbox);
    sql(
        &sandbox,
        "CREATE TRIGGER refuse_clear_relation BEFORE DELETE ON dirty_publications BEGIN SELECT RAISE(ABORT, 'Private canary'); END;",
    );
    let output = confirm(&sandbox, row.id.as_str(), "related:Alias");
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    let error: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(
        error["error"]["completed"],
        serde_json::json!([row.id.as_str()])
    );
    assert!(!stderr(&output).contains("Private canary"));
    let before = ledger(&sandbox).relations(&row.id).unwrap();
    assert_eq!(before.len(), 1);
    sql(&sandbox, "DROP TRIGGER refuse_clear_relation");
    let result = success(&confirm(&sandbox, row.id.as_str(), "related:Alias"));
    assert_eq!(result["confirmed"]["changed"], false);
    assert_eq!(ledger(&sandbox).relations(&row.id).unwrap(), before);
    assert!(ledger(&sandbox).pending_publications().unwrap().is_empty());
    let note = saved(&sandbox, &row);
    assert_eq!(fs::read_to_string(note.target).unwrap(), note.text);
}
