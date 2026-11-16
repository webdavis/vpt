use crate::fixture::{Trial, script};
use crate::support::{stderr, stdout};
use serde_json::Value;
use std::path::PathBuf;

const PROPOSAL: &str = r#"{"schema":"vpt.proposal/1","by":"made-up-model","summary":[{"text":"The kettle club met.","sources":["00:00-00:06"]}],"actions":[],"tags":[],"relations":[]}"#;

struct Logs {
    _directory: tempfile::TempDir,
    input: PathBuf,
    arguments: PathBuf,
}

fn summary(trial: &Trial, answer: &str) -> Logs {
    let directory = tempfile::tempdir().unwrap();
    let logs = Logs {
        input: directory.path().join("input"),
        arguments: directory.path().join("arguments"),
        _directory: directory,
    };
    let program = script(
        &trial.sandbox,
        "made-up-agent",
        &format!(
            "/bin/cat > {:?}\nprintf '%s\\n' \"$@\" > {:?}\n/bin/cat \"$1\" >> {:?}\n{answer}",
            logs.input, logs.arguments, logs.arguments
        ),
    );
    trial.configure(&format!(
        "[analysis]\ncommand = [{program:?}, '{{transcript}}', '{{id}}', '{{language}}', '{{captured_at}}']\n"
    ));
    logs
}

fn row(document: &Value) -> &Value {
    let checks = document["checks"].as_array().unwrap();
    assert_eq!(checks.len(), 1, "{document}");
    assert_eq!(checks[0]["name"], "summary");
    let rows = checks[0]["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1, "{document}");
    assert_eq!(rows[0]["name"], "command");
    &rows[0]
}

#[test]
fn a_valid_proposal_is_ok_and_nothing_is_saved() {
    let trial = Trial::new("test-summary-valid");
    let logs = summary(&trial, &format!("printf '%s' '{PROPOSAL}'"));
    let before = crate::observations_fixture::snapshot(&trial.sandbox);
    let document = trial.json(&["test", "summary"], 0);
    let command = row(&document);
    assert_eq!(command["result"], "ok", "{document}");
    let detail = command["detail"].as_str().unwrap();
    assert!(
        detail.starts_with("a valid vpt.proposal/1 from made-up-model in "),
        "{detail}"
    );
    assert!(
        detail.ends_with("; the made-up transcript went to whatever the command calls"),
        "{detail}"
    );
    assert!(command["advice"].is_null());
    let input = std::fs::read_to_string(&logs.input).unwrap();
    assert!(input.starts_with("[00:00] "), "{input}");
    assert!(input.contains("made-up"), "{input}");
    let arguments = std::fs::read_to_string(&logs.arguments).unwrap();
    let lines: Vec<&str> = arguments.lines().collect();
    assert!(lines[0].starts_with(trial.temp.path().to_str().unwrap()));
    assert_eq!(lines[2], "en-US");
    assert!(arguments.ends_with(&input), "{arguments}");
    assert_eq!(
        crate::observations_fixture::snapshot(&trial.sandbox),
        before
    );
    assert!(trial.temp_is_empty());
}

#[test]
fn an_answer_that_is_not_a_proposal_is_an_issue_with_advice() {
    let trial = Trial::new("test-summary-invalid");
    summary(&trial, "echo 'Here is your summary.'");
    let document = trial.json(&["test", "summary"], 1);
    let command = row(&document);
    assert_eq!(command["result"], "issue");
    let detail = command["detail"].as_str().unwrap();
    assert!(
        detail.contains("analysis output document is invalid"),
        "{detail}"
    );
    assert!(
        command["advice"]
            .as_str()
            .unwrap()
            .contains("vpt.proposal/1")
    );
    assert!(trial.temp_is_empty());
}

#[test]
fn a_failing_command_is_an_issue_with_its_exit_code() {
    let trial = Trial::new("test-summary-exit");
    summary(&trial, "exit 9");
    let document = trial.json(&["test", "summary"], 1);
    let command = row(&document);
    assert_eq!(command["result"], "issue");
    assert!(
        command["detail"].as_str().unwrap().contains("exited 9"),
        "{document}"
    );
}

#[test]
fn without_a_command_a_named_summary_test_is_refused_naming_the_key() {
    let trial = Trial::new("test-summary-unset");
    let output = trial.output(&["test", "summary"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout(&output), "");
    assert!(
        stderr(&output).contains("`analysis.command`"),
        "{}",
        stderr(&output)
    );
}
