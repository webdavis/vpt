use super::reporting::report_command;
use super::*;
use vpt_application::ports::LedgerCommit;

#[test]
fn log_export_failure_preserves_move_and_still_delivers_report() {
    let sandbox = configured(true);
    sandbox.install_fake_helper();
    let row = expired_audio(&sandbox);
    let log = sandbox.path().join("state/vpt/events.jsonl");
    std::fs::create_dir(&log).unwrap();
    report_command(
        &sandbox,
        &format!("[{:?}, \"command-sink\"]", support::FAKE_ENGINE),
    );
    let result = json(&run(sandbox.vpt().args(["retention", "run", "--json"])));
    assert_eq!(result["completed"], serde_json::json!([row.id.as_str()]));
    assert!(
        result["diagnostics"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("event log export is unavailable"))
    );
    assert!(!row.audio_path.exists());
    assert!(log.is_dir());
    let calls = std::fs::read_to_string(sandbox.fake_log()).unwrap();
    assert_eq!(
        calls
            .lines()
            .filter(|line| line.contains("command-sink"))
            .count(),
        1
    );
    std::fs::rename(&log, sandbox.path().join("held-log-directory")).unwrap();
    json(&run(sandbox.vpt().args(["retention", "run", "--json"])));
    let recovered = std::fs::read_to_string(&log).unwrap();
    assert!(recovered.contains("recovery"));
    assert_eq!(
        std::fs::read_to_string(sandbox.fake_log()).unwrap(),
        calls,
        "recovery must not resend hooks or repeat moves"
    );
}

#[test]
fn later_helper_failure_preserves_prior_committed_identifiers() {
    let sandbox = configured(true);
    sandbox.install_fake_helper();
    let first = expired_audio(&sandbox);
    let mut second = record(&sandbox, 2, StageStates::fresh());
    std::fs::write(&second.audio_path, b"second audio").unwrap();
    let mut file = std::fs::File::open(&second.audio_path).unwrap();
    second.digest = digest_open(&mut file).unwrap();
    file.set_modified(SystemTime::UNIX_EPOCH).unwrap();
    let root = RootDir::open(&sandbox.path().join("state/vpt")).unwrap();
    let ledger = SqliteLedger::open(&root).unwrap();
    ledger
        .commit(&LedgerCommit {
            recordings: vec![second.clone()],
            ..Default::default()
        })
        .unwrap();
    drop(ledger);
    std::fs::create_dir(
        sandbox
            .path()
            .join("trash")
            .join(second.audio_path.file_name().unwrap()),
    )
    .unwrap();
    let result = error(
        &run(sandbox.vpt().args(["retention", "run", "--json"])),
        1,
        "helper",
    );
    assert_eq!(
        result["error"]["completed"],
        serde_json::json!([first.id.as_str()])
    );
    assert!(!first.audio_path.exists());
    assert_eq!(std::fs::read(&second.audio_path).unwrap(), b"second audio");
    let ledger = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
    assert!(
        ledger
            .by_id(&first.id)
            .unwrap()
            .unwrap()
            .audio_trashed_at
            .is_some()
    );
    assert!(
        ledger
            .by_id(&second.id)
            .unwrap()
            .unwrap()
            .audio_trashed_at
            .is_none()
    );
}
