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
    assert!(
        is_command_deadline(&failure),
        "missing answer must hit the command deadline: {failure:?}"
    );
    assert!(started.elapsed() < std::time::Duration::from_secs(1));
    assert!(!sandbox.config_path().exists());
}

#[cfg(feature = "dev-tools")]
#[test]
fn setup_terminal_exit_before_prompt_reports_status_and_output() {
    let sandbox = Sandbox::new("setup-before-prompt");
    let failure = std::panic::catch_unwind(|| {
        terminal::run_args(&sandbox, terminal::Answer::None, &["--version"])
    })
    .expect_err("version must exit before a setup prompt");
    let message = terminal_panic_message(&failure);
    assert!(message.starts_with("prompt not observed:"), "{message}");
    assert!(message.contains("status=Some(0)"), "{message}");
    assert!(message.contains("stdout_bytes="), "{message}");
    assert!(message.contains("stdout_preview=vpt "), "{message}");
    assert!(message.contains("terminal_bytes="), "{message}");
    assert!(!sandbox.config_path().exists());
}

#[cfg(feature = "dev-tools")]
#[test]
fn setup_terminal_missing_prompt_escapes_and_bounds_captured_bytes() {
    let sandbox = Sandbox::new("setup-diagnostic-bytes");
    let payload = "A".repeat(80);
    let script =
        "printf '\\033%s' \"$1\" > /dev/tty; printf '\\002%s' \"$2\"; printf '\\003%s' \"$3\" >&2";
    let failure = std::panic::catch_unwind(|| {
        terminal::run_command(
            &sandbox,
            terminal::Answer::None,
            "/bin/sh",
            &[
                "-c",
                script,
                "terminal-fixture",
                &payload,
                &payload,
                &payload,
            ],
        )
    })
    .expect_err("owned shell must exit before a setup prompt");
    let message = terminal_panic_message(&failure);
    for field in ["terminal_bytes=81", "stdout_bytes=81", "stderr_bytes=81"] {
        assert!(message.contains(field), "missing {field}: {message}");
    }
    for field in [
        "terminal_preview=\\x1b",
        "stdout_preview=\\x02",
        "stderr_preview=\\x03",
    ] {
        assert!(message.contains(field), "missing {field}: {message}");
    }
    assert!(
        !message.contains('\u{1b}') && !message.contains('\u{2}') && !message.contains('\u{3}')
    );
    assert!(!message.contains(&"A".repeat(64)), "{message}");
}

#[cfg(feature = "dev-tools")]
#[test]
fn setup_terminal_timeout_assertion_rejects_pre_prompt_exit() {
    let sandbox = Sandbox::new("setup-timeout-classification");
    let outcome = std::panic::catch_unwind(|| {
        terminal::run_args(&sandbox, terminal::Answer::None, &["--version"])
    });
    let failure = outcome
        .as_ref()
        .expect_err("version must exit before a setup prompt");
    let message = terminal_panic_message(failure);
    assert!(message.starts_with("prompt not observed:"), "{message}");
    assert!(
        !is_command_deadline(&outcome),
        "early exit classified as timeout"
    );
}

#[cfg(feature = "dev-tools")]
fn is_command_deadline(outcome: &std::thread::Result<std::process::Output>) -> bool {
    outcome
        .as_ref()
        .err()
        .and_then(|failure| {
            failure
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| failure.downcast_ref::<&str>().copied())
        })
        .is_some_and(|message| message.starts_with("command exceeded 500 ms deadline:"))
}

#[cfg(feature = "dev-tools")]
fn terminal_panic_message(failure: &Box<dyn std::any::Any + Send>) -> &str {
    failure
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| failure.downcast_ref::<&str>().copied())
        .expect("panic message")
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

#[test]
fn setup_refuses_shared_home_state_or_store_before_prompt_or_config_write() {
    use std::os::unix::fs::MetadataExt;
    for leaf in [
        "home/.vpt",
        "state/vpt",
        "home/.vpt/audio",
        "home/.vpt/released",
    ] {
        let sandbox = Sandbox::new("setup-shared-directory");
        let home = sandbox.path().join("home/.vpt");
        std::fs::create_dir(&home).unwrap();
        std::fs::set_permissions(&home, std::fs::Permissions::from_mode(0o700)).unwrap();
        let shared = sandbox.path().join(leaf);
        std::fs::create_dir_all(&shared).unwrap();
        std::fs::set_permissions(&shared, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::write(shared.join("operator"), b"keep").unwrap();
        let before = std::fs::metadata(&shared).unwrap();
        for structured in [false, true] {
            let mut command = sandbox.vpt();
            command.arg("setup");
            if structured {
                command.arg("--json");
            }
            let output = run(&mut command);
            assert_eq!(output.status.code(), Some(3), "{leaf}: {}", stderr(&output));
            assert!(output.stdout.is_empty());
            assert!(!stderr(&output).contains("terminal"));
            if structured {
                let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
                assert_eq!(error["error"]["rule"], "nonprivate_directory");
            }
            let after = std::fs::metadata(&shared).unwrap();
            assert_eq!(
                (
                    after.mode(),
                    after.ctime(),
                    after.ctime_nsec(),
                    after.mtime(),
                    after.mtime_nsec()
                ),
                (
                    before.mode(),
                    before.ctime(),
                    before.ctime_nsec(),
                    before.mtime(),
                    before.mtime_nsec()
                )
            );
            assert_eq!(std::fs::read(shared.join("operator")).unwrap(), b"keep");
            assert!(!sandbox.config_path().exists());
        }
    }
}
