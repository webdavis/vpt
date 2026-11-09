use super::*;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;

pub(super) fn report_command(sandbox: &Sandbox, command: &str) {
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(sandbox.config_path())
        .unwrap();
    writeln!(file, "[report]\ncommand = {command}").unwrap();
}

#[test]
fn hook_failure_warns_without_changing_the_completed_move() {
    let sandbox = configured(true);
    sandbox.install_fake_helper();
    let row = expired_audio(&sandbox);
    report_command(&sandbox, "[\"missing-report-command\"]");
    let result = json(&run(sandbox.vpt().args(["retention", "run", "--json"])));
    assert_eq!(result["completed"], serde_json::json!([row.id.as_str()]));
    assert_eq!(
        result["diagnostics"],
        serde_json::json!(["configured notification command failed"])
    );
    assert!(!row.audio_path.exists());
}

#[test]
fn human_dry_run_describes_proposed_moves() {
    let sandbox = configured(true);
    let row = expired_audio(&sandbox);
    let output = run(sandbox.vpt().args(["retention", "run", "--dry-run"]));
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout(&output).contains("Would move 1 file(s)"));
    assert!(stdout(&output).contains(row.audio_path.to_str().unwrap()));
    assert!(output.stderr.is_empty());
}

#[test]
fn unchanged_untracked_and_modified_files_are_kept() {
    let sandbox = configured(true);
    let row = expired_audio(&sandbox);
    let unknown = row.audio_path.with_file_name("untracked.m4a");
    std::fs::write(&unknown, b"untracked").unwrap();
    std::fs::write(&row.audio_path, b"edited").unwrap();
    std::fs::File::open(&row.audio_path)
        .unwrap()
        .set_modified(SystemTime::UNIX_EPOCH)
        .unwrap();
    let result = json(&run(sandbox.vpt().args(["retention", "run", "--json"])));
    assert_eq!(result["moved"], serde_json::json!([]));
    let reasons: Vec<_> = result["kept"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["reason"].as_str().unwrap())
        .collect();
    assert!(reasons.contains(&"untracked"));
    assert!(reasons.contains(&"digest_mismatch"));
    assert_eq!(std::fs::read(&row.audio_path).unwrap(), b"edited");
    assert_eq!(std::fs::read(unknown).unwrap(), b"untracked");
}

#[test]
fn unsafe_state_is_refused_before_lock_or_history_creation() {
    let sandbox = configured(true);
    expired_audio(&sandbox);
    let state = sandbox.path().join("state/vpt");
    std::fs::set_permissions(&state, std::fs::Permissions::from_mode(0o755)).unwrap();
    let before = snapshot(&sandbox);
    let result = error(
        &run(sandbox.vpt().args(["retention", "run", "--json"])),
        3,
        "refused",
    );
    assert_eq!(result["error"]["rule"], "unsafe_state");
    assert_eq!(snapshot(&sandbox), before);
}

#[test]
fn retention_waits_for_an_existing_writer_to_finish() {
    let sandbox = configured(true);
    sandbox.install_fake_helper();
    let row = expired_audio(&sandbox);
    let root = RootDir::open(&sandbox.path().join("state/vpt")).unwrap();
    let held = WriteLock::acquire(&root, Duration::ZERO).unwrap();
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(150));
        drop(held);
    });
    let output = run(sandbox.vpt().args(["retention", "run", "--json"]));
    release.join().unwrap();
    let result = json(&output);
    assert_eq!(result["completed"], serde_json::json!([row.id.as_str()]));
}

#[test]
fn completed_move_sends_one_private_safe_report_document() {
    let sandbox = configured(true);
    sandbox.install_fake_helper();
    expired_audio(&sandbox);
    report_command(
        &sandbox,
        &format!("[{:?}, \"command-sink\"]", support::FAKE_ENGINE),
    );
    json(&run(sandbox.vpt().args(["retention", "run", "--json"])));
    let log = std::fs::read_to_string(sandbox.fake_log()).unwrap();
    let records: Vec<serde_json::Value> = log
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let hooks: Vec<_> = records
        .iter()
        .filter(|record| record["command"] == "command-sink")
        .collect();
    assert_eq!(hooks.len(), 1);
    let document: serde_json::Value =
        serde_json::from_str(hooks[0]["stdin"].as_str().unwrap()).unwrap();
    assert_eq!(document["schema"], "vpt.event/1");
    assert_eq!(document["event"], "retention");
    assert_eq!(document["counts"]["audio"], 1);
    assert!(!document.to_string().contains("title-1"));
}

#[test]
fn redirected_audio_is_kept_while_regular_candidates_are_moved() {
    let sandbox = configured(true);
    sandbox.install_fake_helper();
    let row = expired_audio(&sandbox);
    let outside = sandbox.path().join("outside.m4a");
    std::fs::write(&outside, b"external audio").unwrap();
    let link = row.audio_path.with_file_name("redirected.m4a");
    std::os::unix::fs::symlink(&outside, &link).unwrap();
    let result = json(&run(sandbox.vpt().args(["retention", "run", "--json"])));
    assert_eq!(result["moved"].as_array().unwrap().len(), 1);
    assert_eq!(
        result["kept"],
        serde_json::json!([{"path": link, "reason": "path_escape"}])
    );
    assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
    assert_eq!(std::fs::read(outside).unwrap(), b"external audio");
}

#[test]
fn ledger_audio_outside_configured_authority_is_reported_kept() {
    let sandbox = configured(true);
    let mut row = record(&sandbox, 1, StageStates::fresh());
    row.audio_path = sandbox.path().join("outside-authority.m4a");
    std::fs::write(&row.audio_path, b"outside").unwrap();
    seed(&sandbox, vec![row.clone()]);
    let result = json(&run(sandbox.vpt().args(["retention", "run", "--json"])));
    assert_eq!(
        result["kept"],
        serde_json::json!([{"path": row.audio_path, "reason": "path_escape"}])
    );
    assert_eq!(std::fs::read(&row.audio_path).unwrap(), b"outside");
    assert_eq!(result["moved"], serde_json::json!([]));
}
