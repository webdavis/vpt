use super::*;
use std::fs;
use vpt_application::ports::{EngineOutputLedger, NoteLedger, NoteStage};

fn sql(sandbox: &Sandbox, statement: &str) {
    let connection = rusqlite::Connection::open(sandbox.path().join("state/vpt/vpt.db")).unwrap();
    fixture::preserve_wal(&connection);
    connection.execute_batch(statement).unwrap();
}

#[test]
fn saved_engine_output_recovers_without_audio_or_another_engine_invocation() {
    let (sandbox, row) = configured();
    sql(
        &sandbox,
        "CREATE TRIGGER refuse_engine_ack BEFORE DELETE ON dirty_publications WHEN OLD.target LIKE '%/engine-outputs/%' BEGIN SELECT RAISE(ABORT, 'Private ack'); END",
    );
    let output = run(&sandbox, row.id.as_str(), &[]);
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    let error: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(
        error["error"]["completed"],
        serde_json::json!([row.id.as_str()])
    );
    let accepted = ledger(&sandbox)
        .accepted_transcript(&row.id)
        .unwrap()
        .unwrap();
    let pending = ledger(&sandbox).pending_publications().unwrap();
    assert_eq!(pending.len(), 1);
    let raw = ledger(&sandbox)
        .engine_output(&pending[0].target)
        .unwrap()
        .unwrap();
    let calls = fs::read(sandbox.path().join("calls")).unwrap();
    sql(&sandbox, "DROP TRIGGER refuse_engine_ack");
    fs::remove_file(&pending[0].target).unwrap();
    fs::remove_file(&row.audio_path).unwrap();
    fs::write(sandbox.path().join("bin/engine"), "#!/bin/sh\nexit 29\n").unwrap();
    let output = support::run_with_budget(
        sandbox
            .vpt()
            .args(["note", "write", row.id.as_str(), "--json"]),
        std::time::Duration::from_secs(5),
    );
    json(&output);
    assert_eq!(
        fs::read(&raw.target).unwrap(),
        fs::read(sandbox.path().join("engine.json")).unwrap()
    );
    assert_eq!(
        ledger(&sandbox).accepted_transcript(&row.id).unwrap(),
        Some(accepted)
    );
    assert_eq!(
        ledger(&sandbox).engine_output(&raw.target).unwrap(),
        Some(raw)
    );
    assert!(ledger(&sandbox).pending_publications().unwrap().is_empty());
    assert_eq!(fs::read(sandbox.path().join("calls")).unwrap(), calls);
    let saved = ledger(&sandbox).by_id(&row.id).unwrap().unwrap();
    assert_eq!(saved.stages.transcribe, StageState::Succeeded);
    assert_eq!(saved.stages.note, StageState::Succeeded);
    assert!(
        ledger(&sandbox)
            .note(&row.id, NoteStage::Transcript)
            .unwrap()
            .unwrap()
            .text
            .contains("private transcript canary")
    );
}
