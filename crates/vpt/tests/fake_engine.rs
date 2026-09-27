#![cfg(feature = "dev-tools")]

#[path = "fake_engine/process.rs"]
mod process;
mod support;

use serde_json::{Value, json};
use std::process::{Command, Stdio};
use support::{Sandbox, broken_output, run, stderr, stdout};

fn fake(sandbox: &Sandbox) -> Command {
    let environment = sandbox.vpt();
    let mut command = Command::new(env!("CARGO_BIN_EXE_vpt-fake-engine"));
    command
        .env_clear()
        .envs(
            environment
                .get_envs()
                .map(|(key, value)| (key, value.expect("set"))),
        )
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

#[test]
fn fake_version_defaults_to_the_helper_document() {
    let sandbox = Sandbox::new("fake-version");
    let output = run(fake(&sandbox).arg("--version"));
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stderr(&output), "");
    let document: Value = serde_json::from_slice(&output.stdout).expect("version JSON");
    assert_eq!(
        document,
        json!({"schema": "vpt.helper/1", "version": "1.0.0"})
    );
}

#[test]
fn fake_version_serializes_hostile_fixture_values() {
    let sandbox = Sandbox::new("fake-version-escaped");
    let version = "1.0.0\"\\\n\t\u{1b}";
    let output = run(fake(&sandbox)
        .arg("--version")
        .env("VPT_FAKE_VERSION", version));
    assert_eq!(output.status.code(), Some(0));
    let document: Value = serde_json::from_slice(&output.stdout).expect("escaped version JSON");
    assert_eq!(document["version"], version);
}

#[test]
fn fake_version_reports_a_failed_output_write() {
    let sandbox = Sandbox::new("fake-version-output-failure");
    let (writer, _peer) = broken_output();
    let output = run(fake(&sandbox).arg("--version").stdout(writer));
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
}

#[test]
fn fake_trash_moves_only_the_owned_file_and_serializes_its_path() {
    let sandbox = Sandbox::new("fake-trash");
    let trash = sandbox.path().join("trash");
    std::fs::create_dir(&trash).expect("synthetic Trash");
    let source = sandbox.path().join("owned \"quote\" \\ line\n.txt");
    let sibling = sandbox.path().join("untouched.txt");
    std::fs::write(&source, b"owned payload").expect("owned source");
    std::fs::write(&sibling, b"untouched").expect("owned sibling");
    let output = run(fake(&sandbox)
        .arg("trash")
        .arg(&source)
        .env("VPT_FAKE_TRASH", &trash));
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stderr(&output), "");
    let document: Value = serde_json::from_slice(&output.stdout).expect("escaped path JSON");
    assert_eq!(document, json!({"trashed": source}));
    assert!(!source.exists());
    assert_eq!(
        std::fs::read(trash.join(source.file_name().expect("leaf"))).expect("moved"),
        b"owned payload"
    );
    assert_eq!(std::fs::read(sibling).expect("sibling"), b"untouched");
}

#[test]
fn fake_trash_without_a_destination_preserves_the_source() {
    let sandbox = Sandbox::new("fake-trash-unset");
    let source = sandbox.path().join("owned.txt");
    std::fs::write(&source, b"keep").expect("owned source");
    let output = run(fake(&sandbox).arg("trash").arg(&source));
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(std::fs::read(source).expect("unmoved"), b"keep");
}

#[test]
fn fake_trash_reports_rename_failure_without_a_success_document() {
    let sandbox = Sandbox::new("fake-trash-failure");
    let source = sandbox.path().join("owned.txt");
    std::fs::write(&source, b"keep").expect("owned source");
    let output = run(fake(&sandbox)
        .arg("trash")
        .arg(&source)
        .env("VPT_FAKE_TRASH", sandbox.path().join("missing")));
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(std::fs::read(source).expect("unmoved"), b"keep");
}

#[test]
fn fake_command_sink_records_exact_arguments_and_stdin_then_uses_the_requested_exit() {
    let sandbox = Sandbox::new("fake-command-sink");
    let log = sandbox.path().join("capture.jsonl");
    let input = sandbox.path().join("stdin");
    let text = "first line\nsecond\t\"quoted\"\\line\n";
    std::fs::write(&input, text).expect("owned input");
    let arguments = ["one two", "one", "two", "", "$(literal);*'\"", "{artifact}"];
    let output = run(fake(&sandbox)
        .arg("command-sink")
        .args(arguments)
        .env("VPT_FAKE_LOG", &log)
        .env("VPT_FAKE_EXIT", "7")
        .stdin(std::fs::File::open(input).expect("fixture stdin")));
    assert_eq!(output.status.code(), Some(7));
    assert_eq!(stdout(&output), "");
    assert_eq!(stderr(&output), "");
    let captured = std::fs::read_to_string(&log).expect("command capture");
    assert_eq!(captured.lines().count(), 1);
    let record: Value = serde_json::from_str(&captured).expect("structured capture");
    assert_eq!(
        record,
        json!({"command": "command-sink", "argv": arguments, "stdin": text})
    );
    let second = run(fake(&sandbox)
        .arg("command-sink")
        .arg("next")
        .env("VPT_FAKE_LOG", &log));
    assert_eq!(second.status.code(), Some(0));
    let captured = std::fs::read_to_string(log).expect("appended capture");
    let records: Vec<Value> = captured
        .lines()
        .map(|line| serde_json::from_str(line).expect("record"))
        .collect();
    assert_eq!(records.len(), 2);
    assert_eq!(
        records[1],
        json!({"command": "command-sink", "argv": ["next"], "stdin": ""})
    );
}

#[test]
fn fake_command_sink_without_a_log_still_consumes_input_and_succeeds() {
    let sandbox = Sandbox::new("fake-sink-no-log");
    let output = run(fake(&sandbox).arg("command-sink"));
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout(&output), "");
    assert_eq!(stderr(&output), "");
}

#[test]
fn fake_rejects_missing_trash_arguments_and_notification_routes() {
    let sandbox = Sandbox::new("fake-usage");
    for arguments in [&["trash"][..], &["notify", "--title", "owned"]] {
        let output = run(fake(&sandbox).args(arguments));
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(stdout(&output), "");
    }
}
