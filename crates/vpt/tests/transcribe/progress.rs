use super::*;
use serde_json::{Value, json};

fn lines(output: &std::process::Output) -> Vec<Value> {
    stderr(output)
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn steps(lines: &[Value]) -> Vec<Value> {
    lines
        .iter()
        .map(|line| {
            json!([
                line["schema"],
                line["recording"],
                line["step"],
                line["state"]
            ])
        })
        .collect()
}

#[test]
fn progress_lines_bracket_the_engine_on_stderr_and_leave_stdout_alone() {
    let (sandbox, row) = configured();
    let output = run(&sandbox, row.id.as_str(), &["--progress"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let result: Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert_eq!(result["command"], "transcribe");
    let lines = lines(&output);
    let id = row.id.as_str();
    assert_eq!(
        steps(&lines),
        [
            json!(["vpt.progress/1", id, "transcribe", "started"]),
            json!(["vpt.progress/1", id, "transcribe", "done"]),
        ]
    );
    assert_eq!(lines[0]["audio_secs"], 12.0);
}

#[test]
fn a_failed_engine_reports_failed_before_the_error_document() {
    let (sandbox, row) = configured();
    std::fs::write(sandbox.path().join("bin/engine"), "#!/bin/sh\nexit 1\n").unwrap();
    let output = run(&sandbox, row.id.as_str(), &["--progress"]);
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(output.stdout.is_empty());
    let lines = lines(&output);
    let id = row.id.as_str();
    assert_eq!(
        steps(&lines[..2]),
        [
            json!(["vpt.progress/1", id, "transcribe", "started"]),
            json!(["vpt.progress/1", id, "transcribe", "failed"]),
        ]
    );
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[2]["schema"], "vpt.error/1");
}
