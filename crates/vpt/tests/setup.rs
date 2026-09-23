#[path = "setup/environment.rs"]
mod environment;
#[path = "setup/source.rs"]
mod source;
mod support;
#[cfg(feature = "dev-tools")]
#[path = "setup/terminal.rs"]
mod terminal;
use std::os::unix::fs::PermissionsExt;
use support::{Sandbox, run, stderr, stdout};

#[cfg(feature = "dev-tools")]
#[test]
fn setup_under_owned_terminal_writes_private_config_and_directories() {
    use std::os::unix::fs::PermissionsExt;
    let sandbox = Sandbox::new("setup-pty");
    let output = terminal::run(&sandbox, terminal::Answer::Line(b"\n"));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let config = std::fs::read_to_string(sandbox.config_path()).expect("config written");
    assert!(config.starts_with("config_version = 1"), "{config}");
    assert!(config.contains("main = \"apple\""), "{config}");
    let mode = std::fs::metadata(sandbox.config_path())
        .expect("metadata")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600);
    let home = sandbox.path().join("home/.vpt");
    assert_eq!(
        std::fs::metadata(&home).expect("home").permissions().mode() & 0o777,
        0o700
    );
    for leaf in [
        "audio",
        "transcripts",
        "analysis",
        "briefs",
        "engine-outputs",
        "drafts",
        "released",
    ] {
        assert!(home.join(leaf).is_dir(), "{leaf} missing");
    }
    assert!(sandbox.path().join("state/vpt").is_dir());
}

#[cfg(feature = "dev-tools")]
#[test]
fn setup_json_terminal_success_keeps_the_prompt_off_standard_streams() {
    let sandbox = Sandbox::new("setup-json-pty");
    let output = terminal::run_args(
        &sandbox,
        terminal::Answer::Line(b"\n"),
        &["setup", "--json"],
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let text = stdout(&output);
    let result: serde_json::Value = serde_json::from_str(&text).expect("one result document");
    assert_eq!(result["command"], "setup");
    assert_eq!(result["main_engine"], "apple");
    assert!(!text.contains("choice [1]"));
    assert_eq!(stderr(&output), "");
}

#[cfg(feature = "dev-tools")]
#[test]
fn setup_terminal_eof_refuses_without_writing() {
    let sandbox = Sandbox::new("setup-eof");
    let output = terminal::run(&sandbox, terminal::Answer::Eof);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("terminal closed before an answer"));
    assert!(!sandbox.config_path().exists());
}

#[cfg(feature = "dev-tools")]
#[test]
fn setup_force_replaces_existing_config_with_selected_engine() {
    let sandbox = Sandbox::new("setup-force");
    std::fs::create_dir_all(sandbox.config_path().parent().expect("parent")).expect("parent");
    std::fs::set_permissions(
        sandbox.config_path().parent().expect("parent"),
        std::fs::Permissions::from_mode(0o700),
    )
    .expect("private config parent");
    std::fs::write(sandbox.config_path(), "PRIVATE-OLD-CONTENT-PLACEHOLDER")
        .expect("existing config");
    let output = terminal::run_args(
        &sandbox,
        terminal::Answer::Line(b"2\n"),
        &["setup", "--force"],
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let config = std::fs::read_to_string(sandbox.config_path()).expect("config");
    assert!(config.contains("main = \"whisply\""), "{config}");
    assert!(!config.contains("PRIVATE-OLD-CONTENT-PLACEHOLDER"));
}

#[cfg(feature = "dev-tools")]
#[test]
fn setup_terminal_answer_timeout_reaps_the_owned_command() {
    let sandbox = Sandbox::new("setup-timeout");
    let started = std::time::Instant::now();
    let failure = std::panic::catch_unwind(|| terminal::run(&sandbox, terminal::Answer::None));
    assert!(failure.is_err(), "missing answer must time out");
    assert!(started.elapsed() < std::time::Duration::from_secs(1));
    assert!(!sandbox.config_path().exists());
}

#[test]
fn setup_without_a_terminal_exits_2_and_writes_nothing() {
    let sandbox = Sandbox::new("setup-no-tty");
    let output = run(sandbox.vpt().args(["setup", "--json"]));
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout(&output), "");
    let error: serde_json::Value = serde_json::from_str(&stderr(&output)).expect("error document");
    assert_eq!(
        error["error"]["message"],
        "vpt setup needs a controlling terminal"
    );
    assert!(!sandbox.config_path().exists());
    assert!(!sandbox.path().join("home/.vpt").exists());
}

#[test]
fn setup_refuses_existing_config_before_the_terminal() {
    let sandbox = Sandbox::new("setup-existing");
    std::fs::create_dir_all(sandbox.config_path().parent().expect("parent")).expect("parent");
    std::fs::set_permissions(
        sandbox.config_path().parent().expect("parent"),
        std::fs::Permissions::from_mode(0o700),
    )
    .expect("private config parent");
    std::fs::write(sandbox.config_path(), "original").expect("existing");
    let output = run(sandbox.vpt().arg("setup"));
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("exists; pass --force"),
        "{}",
        stderr(&output)
    );
    assert!(!stderr(&output).contains("Usage:"));
    assert_eq!(
        std::fs::read_to_string(sandbox.config_path()).expect("kept"),
        "original"
    );
}

#[test]
fn setup_refuses_shared_existing_config_parent_before_prompt_or_write() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let sandbox = Sandbox::new("setup-shared-config");
    let parent = sandbox.path().join("config/shared");
    std::fs::create_dir(&parent).expect("parent");
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o755)).expect("shared mode");
    std::fs::write(parent.join("keep"), "unchanged").expect("sentinel");
    let before = std::fs::metadata(&parent).expect("before metadata");
    let selected = parent.join("selected.toml");
    let output = run(sandbox.vpt().args(["setup", "--config"]).arg(&selected));
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    assert!(stderr(&output).contains("private"), "{}", stderr(&output));
    assert!(!stderr(&output).contains("terminal"));
    let after = std::fs::metadata(&parent).expect("after metadata");
    assert_eq!(after.mode(), before.mode());
    assert_eq!(
        (after.mtime(), after.mtime_nsec()),
        (before.mtime(), before.mtime_nsec())
    );
    assert_eq!(
        std::fs::read_to_string(parent.join("keep")).expect("sentinel"),
        "unchanged"
    );
    assert!(!selected.exists());
}

#[test]
fn setup_refusal_with_broken_stderr_exits_one() {
    let sandbox = Sandbox::new("setup-broken-stderr");
    let (descriptor, _peer) = support::broken_output();
    let output = run(sandbox.vpt().arg("setup").stderr(descriptor));
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
}
