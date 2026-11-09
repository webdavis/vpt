use super::*;
use std::{process::Stdio, time::Duration};

fn broken_write(json: bool, blocked_log: bool) {
    let (sandbox, row) = configured();
    if blocked_log {
        std::fs::create_dir(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
    }
    let (descriptor, _peer) = support::broken_output();
    let mut command = sandbox.vpt();
    command.args(["note", "write", row.id.as_str()]);
    if json {
        command.arg("--json");
    }
    command.stdout(Stdio::from(descriptor));
    let output = support::run_with_budget(&mut command, Duration::from_secs(5));
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(output.stdout.is_empty());
    let note = saved(&sandbox, &row);
    assert_eq!(std::fs::read_to_string(&note.target).unwrap(), note.text);
    let error = stderr(&output);
    assert!(!error.contains("private words"));
    assert!(!error.contains("title-7"));
    if json {
        let document: serde_json::Value = serde_json::from_str(&error).unwrap();
        assert_eq!(document["schema"], "vpt.error/1");
        assert_eq!(document["error"]["message"], "standard output write failed");
        assert_eq!(
            document["error"]["completed"],
            serde_json::json!([row.id.as_str()])
        );
    } else {
        assert!(error.contains("standard output write failed"));
        assert!(error.contains(row.id.as_str()), "{error}");
    }
    if blocked_log {
        assert!(error.contains("event log export is unavailable"), "{error}");
    }
}

#[test]
fn json_output_failure_preserves_committed_note_and_recording_id() {
    broken_write(true, false);
}

#[test]
fn human_output_failure_preserves_committed_note_and_recording_id() {
    broken_write(false, false);
}

#[test]
fn output_failure_preserves_log_export_warnings_in_both_modes() {
    for json in [true, false] {
        broken_write(json, true);
    }
}
