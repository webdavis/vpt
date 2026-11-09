use super::*;
use std::fs;
use vpt_application::ports::{NoteLedger, NoteStage};

#[test]
fn threshold_equal_to_recording_count_keeps_individual_alert_and_history_reopens() {
    let (sandbox, row) = uncertainty::apple();
    notes::hooks(&sandbox, &notes::target(&sandbox, &row), false);
    let config = fs::read_to_string(sandbox.config_path())
        .unwrap()
        .replace("[alert]", "[alert]\naggregate_after = 1");
    fs::write(sandbox.config_path(), config).unwrap();
    json(&run(&sandbox, row.id.as_str(), &[]));
    let events = notes::alerts(&sandbox);
    assert_eq!(events.len(), 1);
    assert!(events[0]["counts"]["recordings"].is_null());
    assert_eq!(events[0]["counts"]["agreed_unverified"], 2);
    assert_eq!(events[0]["recording"], row.id.as_str());
    assert_eq!(
        events[0]["paths"]["note"],
        notes::target(&sandbox, &row).to_str().unwrap()
    );
    use vpt_application::ports::RunHistory;
    let history = ledger(&sandbox).recovery_view().unwrap();
    assert!(format!("{history:?}").contains("ReviewNeeded"));
}

#[test]
fn note_acknowledgement_failure_keeps_saved_inputs_and_emits_no_premature_alert() {
    let (sandbox, row) = uncertainty::apple();
    notes::hooks(&sandbox, &notes::target(&sandbox, &row), false);
    let connection = rusqlite::Connection::open(sandbox.path().join("state/vpt/vpt.db")).unwrap();
    fixture::preserve_wal(&connection);
    connection.execute_batch("CREATE TRIGGER refuse_note_ack BEFORE DELETE ON dirty_publications WHEN EXISTS (SELECT 1 FROM owned_notes WHERE target = OLD.target) BEGIN SELECT RAISE(ABORT, 'Private failure'); END;").unwrap();
    drop(connection);
    let output = run(&sandbox, row.id.as_str(), &[]);
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(!stderr(&output).contains("Private failure"));
    let error: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(
        error["error"]["completed"],
        serde_json::json!([row.id.as_str()])
    );
    let saved = ledger(&sandbox)
        .note(&row.id, NoteStage::Transcript)
        .unwrap()
        .unwrap();
    assert_eq!(fs::read_to_string(&saved.target).unwrap(), saved.text);
    assert!(!ledger(&sandbox).pending_publications().unwrap().is_empty());
    assert_eq!(
        ledger(&sandbox)
            .by_id(&row.id)
            .unwrap()
            .unwrap()
            .stages
            .note,
        StageState::Pending
    );
    assert!(notes::alerts(&sandbox).is_empty());
    let connection = rusqlite::Connection::open(sandbox.path().join("state/vpt/vpt.db")).unwrap();
    fixture::preserve_wal(&connection);
    connection
        .execute_batch("DROP TRIGGER refuse_note_ack")
        .unwrap();
    drop(connection);
    json(&support::run_with_budget(
        sandbox
            .vpt()
            .args(["confirm", "--term", "Recovered", "--json"]),
        std::time::Duration::from_secs(5),
    ));
    assert!(ledger(&sandbox).pending_publications().unwrap().is_empty());
    assert_eq!(
        ledger(&sandbox)
            .by_id(&row.id)
            .unwrap()
            .unwrap()
            .stages
            .note,
        StageState::Succeeded
    );
    assert!(notes::alerts(&sandbox).is_empty());
}

#[test]
fn unavailable_log_preserves_publication_and_delivers_original_alert() {
    let (sandbox, row) = uncertainty::apple();
    notes::hooks(&sandbox, &notes::target(&sandbox, &row), false);
    fs::create_dir(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
    let result = json(&run(&sandbox, row.id.as_str(), &[]));
    assert_eq!(notes::alerts(&sandbox).len(), 1);
    assert!(
        result["warnings"]
            .as_array()
            .is_some_and(|warnings| !warnings.is_empty())
    );
    assert_eq!(
        ledger(&sandbox)
            .by_id(&row.id)
            .unwrap()
            .unwrap()
            .stages
            .note,
        StageState::Succeeded
    );
}
