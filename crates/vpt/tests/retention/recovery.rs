use super::*;
use vpt_application::ports::RetentionJournal;

#[test]
fn interrupted_move_is_reconciled_before_new_candidates() {
    let sandbox = configured(true);
    let row = expired_audio(&sandbox);
    error(
        &run(sandbox.vpt().args(["retention", "run", "--json"])),
        3,
        "refused",
    );
    sandbox.install_fake_helper();
    let result = json(&run(sandbox.vpt().args(["retention", "run", "--json"])));
    assert_eq!(result["moved"].as_array().unwrap().len(), 1);
    assert_eq!(result["completed"], serde_json::json!([row.id.as_str()]));
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    assert!(ledger.pending_intents().unwrap().is_empty());
    let log = std::fs::read_to_string(sandbox.fake_log()).unwrap();
    assert_eq!(log.lines().count(), 1, "one actual Trash call");
}

#[test]
fn absent_pending_target_completes_expiration_without_another_move() {
    let sandbox = configured(true);
    let row = expired_audio(&sandbox);
    error(
        &run(sandbox.vpt().args(["retention", "run", "--json"])),
        3,
        "refused",
    );
    std::fs::rename(&row.audio_path, sandbox.path().join("already-trashed")).unwrap();
    let result = json(&run(sandbox.vpt().args(["retention", "run", "--json"])));
    assert_eq!(result["moved"], serde_json::json!([]));
    assert_eq!(result["completed"], serde_json::json!([row.id.as_str()]));
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    assert!(ledger.pending_intents().unwrap().is_empty());
    assert!(
        ledger
            .by_id(&row.id)
            .unwrap()
            .unwrap()
            .audio_trashed_at
            .is_some()
    );
    assert!(!sandbox.fake_log().exists());
}

#[test]
fn replaced_pending_target_refuses_without_expiring_the_record() {
    let sandbox = configured(true);
    let row = expired_audio(&sandbox);
    error(
        &run(sandbox.vpt().args(["retention", "run", "--json"])),
        3,
        "refused",
    );
    std::fs::write(&row.audio_path, b"replacement").unwrap();
    let result = error(
        &run(sandbox.vpt().args(["retention", "run", "--json"])),
        3,
        "refused",
    );
    assert_eq!(result["error"]["rule"], "retention_target_modified");
    assert_eq!(std::fs::read(&row.audio_path).unwrap(), b"replacement");
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    assert!(
        ledger
            .by_id(&row.id)
            .unwrap()
            .unwrap()
            .audio_trashed_at
            .is_none()
    );
}
