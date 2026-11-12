use super::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;

const ABSENT: &str =
    "Voice Memos has not stored a recording on this Mac yet; nothing to ingest from recordings_dir";

fn without_recordings() -> Sandbox {
    let sandbox = configured();
    fs::remove_dir(sandbox.path().join("voice-memos/Recordings")).unwrap();
    sandbox
}

#[test]
fn missing_recordings_folder_means_nothing_to_ingest() {
    let sandbox = without_recordings();
    for dry_run in [true, false] {
        let mut command = sandbox.vpt();
        command.args(["ingest", "--json"]);
        if dry_run {
            command.arg("--dry-run");
        }
        let doc = json(&run(&mut command));
        assert_eq!(doc["ingested"], serde_json::json!([]));
        assert_eq!(doc["would_ingest"], serde_json::json!([]));
        assert_eq!(doc["diagnostics"], serde_json::json!([ABSENT]));
    }
    assert!(!sandbox.path().join("voice-memos/Recordings").exists());
    let events = fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
    assert!(!events.contains("ingest_failed"));
}

#[test]
fn human_ingest_names_the_missing_recordings_folder() {
    let sandbox = without_recordings();
    let output = run(sandbox.vpt().arg("ingest"));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(stdout(&output).contains(&format!("Warning: {ABSENT}\n")));
}

#[test]
fn recordings_folder_that_vanished_after_ingesting_still_aborts() {
    let sandbox = configured();
    let source = add(&sandbox, "gone.m4a");
    json(&run(sandbox.vpt().args(["ingest", "--json"])));
    let recordings = source.parent().unwrap();
    fs::rename(recordings, sandbox.path().join("held-recordings")).unwrap();
    let output = run(sandbox.vpt().args(["ingest", "--json"]));
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("previously populated Voice Memos source is empty"));
    assert!(!stderr(&output).contains(ABSENT));
}

#[test]
fn unreadable_recordings_folder_still_fails() {
    let sandbox = configured();
    let recordings = sandbox.path().join("voice-memos/Recordings");
    fs::set_permissions(&recordings, fs::Permissions::from_mode(0o000)).unwrap();
    let output = run(sandbox.vpt().args(["ingest", "--json"]));
    fs::set_permissions(&recordings, fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("Voice Memos recordings directory is unreadable"));
}
