use crate::fixture::{check, configured, detail, result, run_json, snapshot};
use crate::support::{Sandbox, run, stderr, stdout};
use std::os::unix::fs::PermissionsExt;

const ABSENT: &str = "not created yet; Voice Memos makes it with its first recording on this Mac, and `vpt import` works without it";
const NOT_RUN: &str = "the Voice Memos folder does not exist yet";

fn without_recordings(name: &str) -> Sandbox {
    let sandbox = configured(name);
    std::fs::remove_dir(sandbox.path().join("voice-memos/Recordings")).unwrap();
    sandbox
}

#[test]
fn missing_recordings_folder_is_ok_and_its_subfolders_are_not_run() {
    let sandbox = without_recordings("doctor-missing-source");
    let before = snapshot(&sandbox);
    let document = run_json(&sandbox, 0);
    assert_eq!(result(&document, "recordings", "folder"), "ok");
    assert_eq!(detail(&document, "recordings", "folder"), ABSENT);
    for name in vpt_adapters::APPLE_SUBDIRECTORIES {
        assert_eq!(result(&document, "recordings", name), "not_run");
        assert_eq!(detail(&document, "recordings", name), NOT_RUN);
    }
    assert_eq!(check(&document, "recordings")["result"], "ok");
    assert_eq!(snapshot(&sandbox), before);
    assert!(!sandbox.path().join("voice-memos/Recordings").exists());
}

#[test]
fn missing_recordings_folder_prints_an_ok_row() {
    let sandbox = without_recordings("doctor-missing-source-human");
    let output = run(sandbox.vpt().current_dir(sandbox.path()).arg("doctor"));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(
        text.lines()
            .any(|line| line == format!("  ✓ folder: {ABSENT}")),
        "{text}"
    );
    assert!(text.contains("\n  · Capture: "), "{text}");
}

#[test]
fn absent_folder_stays_ok_beside_a_failed_check() {
    let sandbox = without_recordings("doctor-missing-source-failed");
    std::fs::rename(
        sandbox.path().join("bin/vpt-macos"),
        sandbox.path().join("held-helper"),
    )
    .unwrap();
    let document = run_json(&sandbox, 1);
    assert_eq!(result(&document, "recordings", "folder"), "ok");
    assert_eq!(result(&document, "helper", "vpt-macos"), "issue");
}

#[test]
fn recordings_path_that_is_a_file_still_fails() {
    let sandbox = without_recordings("doctor-source-file");
    std::fs::write(sandbox.path().join("voice-memos/Recordings"), b"fixture").unwrap();
    let document = run_json(&sandbox, 1);
    assert_eq!(result(&document, "recordings", "folder"), "issue");
    assert_eq!(result(&document, "recordings", "Capture"), "not_run");
    assert_eq!(
        detail(&document, "recordings", "Capture"),
        "the Voice Memos folder could not be opened"
    );
}

#[test]
fn refused_recordings_folder_still_fails() {
    let sandbox = configured("doctor-source-refused");
    let recordings = sandbox.path().join("voice-memos/Recordings");
    std::fs::set_permissions(&recordings, std::fs::Permissions::from_mode(0o000)).unwrap();
    let document = run_json(&sandbox, 1);
    std::fs::set_permissions(&recordings, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(result(&document, "recordings", "folder"), "issue");
    assert_eq!(
        detail(&document, "recordings", "folder"),
        "permission denied at checked path"
    );
    assert_eq!(result(&document, "recordings", "Capture"), "not_run");
}

#[test]
fn relative_recordings_path_still_fails() {
    let sandbox = configured("doctor-source-relative");
    crate::observations_fixture::write_config(
        &sandbox,
        "[source]\nrecordings_dir = \"voice-memos/Recordings\"\n",
    );
    let document = run_json(&sandbox, 1);
    assert_eq!(result(&document, "recordings", "folder"), "issue");
}
