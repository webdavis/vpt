use crate::fixture::{Trial, script};
use crate::support::{stderr, stdout};
use serde_json::Value;
use std::path::PathBuf;

struct Logs {
    _directory: tempfile::TempDir,
    sent: PathBuf,
    withdrawn: PathBuf,
}

fn actions(trial: &Trial, send: &str, withdraw: Option<&str>) -> Logs {
    let directory = tempfile::tempdir().unwrap();
    let logs = Logs {
        sent: directory.path().join("sent"),
        withdrawn: directory.path().join("withdrawn"),
        _directory: directory,
    };
    let sender = script(
        &trial.sandbox,
        "made-up-send",
        &format!(
            "printf '%s\\n' \"$@\" \"$MADE_UP_LIST\" >> {:?}\n{send}",
            logs.sent
        ),
    );
    let withdraw = withdraw.map_or_else(String::new, |body| {
        let program = script(
            &trial.sandbox,
            "made-up-withdraw",
            &format!("printf '%s\\n' \"$@\" >> {:?}\n{body}", logs.withdrawn),
        );
        format!("withdraw_command = [{program:?}, '{{id}}']\n")
    });
    trial.configure(&format!(
        "[actions]\ncommand = [{sender:?}, '{{id}}', '{{text}}', '{{owner}}']\n{withdraw}final_exit_codes = [5]\n[actions.env]\nMADE_UP_LIST = 'inbox'\n"
    ));
    logs
}

fn rows(document: &Value) -> &Vec<Value> {
    let checks = document["checks"].as_array().unwrap();
    assert_eq!(checks.len(), 1, "{document}");
    assert_eq!(checks[0]["name"], "actions");
    checks[0]["rows"].as_array().unwrap()
}

fn lines(path: &PathBuf) -> Vec<String> {
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect()
}

#[test]
fn one_follow_up_is_sent_then_withdrawn_and_nothing_is_queued() {
    let trial = Trial::new("test-actions-both");
    let logs = actions(&trial, "", Some(""));
    let before = crate::observations_fixture::snapshot(&trial.sandbox);
    let document = trial.json(&["test", "actions"], 0);
    let rows = rows(&document);
    let names: Vec<&str> = rows
        .iter()
        .map(|row| row["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["send", "withdraw"]);
    assert_eq!(rows[0]["result"], "ok", "{document}");
    assert_eq!(rows[1]["result"], "ok", "{document}");
    let sent = lines(&logs.sent);
    let id = &sent[0];
    assert!(sent[1].starts_with("vpt test:"), "{sent:?}");
    assert_eq!(sent[2], "me");
    assert_eq!(sent[3], "inbox");
    assert_eq!(lines(&logs.withdrawn), std::slice::from_ref(id));
    let detail = rows[0]["detail"].as_str().unwrap();
    assert!(detail.contains(id.as_str()), "{detail}");
    assert!(detail.contains("whatever it calls"), "{detail}");
    assert_eq!(
        crate::observations_fixture::snapshot(&trial.sandbox),
        before
    );
}

#[test]
fn every_run_sends_a_fresh_id() {
    let trial = Trial::new("test-actions-fresh");
    let logs = actions(&trial, "", Some(""));
    trial.json(&["test", "actions"], 0);
    std::thread::sleep(std::time::Duration::from_millis(1_100));
    trial.json(&["test", "actions"], 0);
    let withdrawn = lines(&logs.withdrawn);
    assert_eq!(withdrawn.len(), 2);
    assert_ne!(withdrawn[0], withdrawn[1]);
}

#[test]
fn a_failed_withdrawal_is_an_issue_naming_the_id_to_remove_by_hand() {
    let trial = Trial::new("test-actions-left-behind");
    let logs = actions(&trial, "", Some("exit 4"));
    let document = trial.json(&["test", "actions"], 1);
    let rows = rows(&document);
    assert_eq!(rows[0]["result"], "ok");
    assert_eq!(rows[1]["result"], "issue");
    let id = &lines(&logs.sent)[0];
    assert!(rows[1]["detail"].as_str().unwrap().contains(id.as_str()));
    let advice = rows[1]["advice"].as_str().unwrap();
    assert!(advice.contains(id.as_str()), "{advice}");
    assert!(advice.contains("by hand"), "{advice}");
}

#[test]
fn a_failed_send_is_an_issue_and_nothing_is_withdrawn() {
    let trial = Trial::new("test-actions-send-fails");
    let logs = actions(&trial, "exit 5", Some(""));
    let document = trial.json(&["test", "actions"], 1);
    let rows = rows(&document);
    assert_eq!(rows[0]["result"], "issue");
    assert!(rows[0]["detail"].as_str().unwrap().contains("refused"));
    assert!(rows[0]["advice"].as_str().is_some());
    assert_eq!(rows[1]["result"], "not_run");
    assert!(!logs.withdrawn.exists());
}

#[test]
fn without_a_withdraw_command_a_named_actions_test_is_refused_and_sends_nothing() {
    let trial = Trial::new("test-actions-no-withdraw");
    let logs = actions(&trial, "", None);
    let output = trial.output(&["test", "actions"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout(&output), "");
    assert!(
        stderr(&output).contains("set `actions.withdraw_command`"),
        "{}",
        stderr(&output)
    );
    assert!(!logs.sent.exists());
}

#[test]
fn without_a_withdraw_command_a_bare_run_skips_actions_with_the_reason() {
    let trial = Trial::new("test-actions-bare-skip");
    let logs = actions(&trial, "", None);
    let document = trial.json(&["test"], 0);
    let actions = document["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|check| check["name"] == "actions")
        .unwrap();
    let rows = actions["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["result"], "not_run");
    assert!(
        rows[0]["detail"]
            .as_str()
            .unwrap()
            .contains("`actions.withdraw_command`")
    );
    assert!(!logs.sent.exists());
}

#[test]
fn without_a_send_command_a_named_actions_test_is_refused_naming_the_key() {
    let trial = Trial::new("test-actions-unset");
    let output = trial.output(&["test", "actions"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("`actions.command`"),
        "{}",
        stderr(&output)
    );
}
