use crate::fixture::{check, configured, detail, run_json, snapshot};
use crate::support::{Sandbox, run, stderr, stdout};
use std::os::unix::fs::PermissionsExt;

const ABSENT: &str = "absent: Voice Memos has not stored a recording on this Mac yet; the folder appears after the first recording, and `vpt import` works without it";
const NOT_RUN: &str = "not run: recordings_dir does not exist yet";

fn without_recordings(name: &str) -> Sandbox {
    let sandbox = configured(name);
    std::fs::remove_dir(sandbox.path().join("voice-memos/Recordings")).unwrap();
    sandbox
}

#[test]
fn missing_recordings_folder_is_a_notice() {
    let sandbox = without_recordings("doctor-missing-source");
    let before = snapshot(&sandbox);
    let checks = run_json(&sandbox, 0);
    let recordings = check(&checks, "recordings_dir");
    assert_eq!(recordings["ok"], true);
    assert_eq!(recordings["notice"], true);
    assert_eq!(recordings["detail"], ABSENT);
    for name in vpt_adapters::APPLE_SUBDIRECTORIES {
        let subdirectory = check(&checks, &format!("subdirectory:{name}"));
        assert_eq!(subdirectory["ok"], true);
        assert_eq!(subdirectory["notice"], true);
        assert_eq!(subdirectory["detail"], NOT_RUN);
    }
    assert!(check(&checks, "helper").get("notice").is_none());
    assert_eq!(snapshot(&sandbox), before);
    assert!(!sandbox.path().join("voice-memos/Recordings").exists());
}

#[test]
fn missing_recordings_folder_prints_a_note_line() {
    let sandbox = without_recordings("doctor-missing-source-human");
    let output = run(sandbox.vpt().current_dir(sandbox.path()).arg("doctor"));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(
        text.lines()
            .any(|line| line == format!("note {:<26} {ABSENT}", "recordings_dir")),
        "{text}"
    );
    assert!(text.lines().all(|line| !line.starts_with("FAIL")), "{text}");
}

#[test]
fn notice_survives_beside_a_failed_check() {
    let sandbox = without_recordings("doctor-missing-source-failed");
    std::fs::rename(
        sandbox.path().join("bin/vpt-macos"),
        sandbox.path().join("held-helper"),
    )
    .unwrap();
    let checks = run_json(&sandbox, 3);
    assert_eq!(check(&checks, "recordings_dir")["notice"], true);
    assert_eq!(check(&checks, "helper")["ok"], false);
}

#[test]
fn recordings_path_that_is_a_file_still_fails() {
    let sandbox = without_recordings("doctor-source-file");
    std::fs::write(sandbox.path().join("voice-memos/Recordings"), b"fixture").unwrap();
    let checks = run_json(&sandbox, 3);
    assert_eq!(check(&checks, "recordings_dir")["ok"], false);
    assert!(check(&checks, "recordings_dir").get("notice").is_none());
    assert_eq!(
        detail(&checks, "subdirectory:Capture"),
        "not run: recordings_dir could not be opened"
    );
}

#[test]
fn refused_recordings_folder_still_fails() {
    let sandbox = configured("doctor-source-refused");
    let recordings = sandbox.path().join("voice-memos/Recordings");
    std::fs::set_permissions(&recordings, std::fs::Permissions::from_mode(0o000)).unwrap();
    let checks = run_json(&sandbox, 3);
    std::fs::set_permissions(&recordings, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(check(&checks, "recordings_dir")["ok"], false);
    assert_eq!(
        detail(&checks, "recordings_dir"),
        "permission denied at checked path"
    );
    assert_eq!(check(&checks, "subdirectory:Capture")["ok"], false);
}

#[test]
fn relative_recordings_path_still_fails() {
    let sandbox = configured("doctor-source-relative");
    crate::observations_fixture::write_config(
        &sandbox,
        "[source]\nrecordings_dir = \"voice-memos/Recordings\"\n",
    );
    let checks = run_json(&sandbox, 3);
    assert_eq!(check(&checks, "recordings_dir")["ok"], false);
    assert!(check(&checks, "recordings_dir").get("notice").is_none());
}
