use super::*;
use crate::actions::{configure_with, propose, sent};
use serde_json::json;
use std::path::{Path, PathBuf};

pub(super) fn withdraw_setting(sandbox: &Sandbox) -> (String, PathBuf) {
    let script = sandbox.path().join("todo-tool.sh");
    let withdrawn = sandbox.path().join("withdrawn");
    let setting = format!(
        "withdraw_command = ['/bin/sh', {script:?}, {withdrawn:?}, 'remove', '--id', '{{id}}', '--recording', '{{recording}}']\n"
    );
    (setting, withdrawn)
}

pub(super) fn configured(extra: &str) -> (Sandbox, String, PathBuf, PathBuf) {
    let (sandbox, mut rows) = note::configured_saying(&[7], "um private words tomorrow");
    let id = rows.remove(0).id.as_str().to_owned();
    let (setting, withdrawn) = withdraw_setting(&sandbox);
    let log = configure_with(&sandbox, &format!("{setting}{extra}"));
    (sandbox, id, log, withdrawn)
}

pub(super) fn keep_first_two(sandbox: &Sandbox) {
    let actions = fs::read_to_string(sandbox.path().join("captured.proposal")).unwrap();
    let proposal: serde_json::Value = serde_json::from_str(&actions).unwrap();
    let kept = &proposal["actions"].as_array().unwrap()[..2];
    propose(sandbox, &json!(kept).to_string());
}

pub(super) fn analyze(sandbox: &Sandbox, id: &str) -> serde_json::Value {
    note::success(&invoke(sandbox, id, false))
}

pub(super) fn attempts(log: &Path) -> usize {
    sent(&log.with_extension("attempts")).len()
}

pub(super) fn counts(sent: u32, failed: u32, refused: u32, withdrawn: u32) -> serde_json::Value {
    json!({"sent": sent, "failed": failed, "refused": refused, "withdrawn": withdrawn})
}

#[test]
fn a_redo_that_drops_sent_actions_withdraws_each_once_by_id() {
    let (sandbox, id, log, withdrawn) = configured("[report]\ncommand = ['report']\n");
    let capture = crate::report_hook::install(&sandbox);
    assert_eq!(analyze(&sandbox, &id)["actions"], counts(4, 0, 0, 0));
    let original = fs::read_to_string(sandbox.path().join("captured.proposal")).unwrap();
    keep_first_two(&sandbox);
    let result = analyze(&sandbox, &id);
    assert_eq!(result["actions"], counts(0, 0, 0, 2));
    assert_eq!(result["warnings"], json!([]));
    let remove = |action: &str| -> Vec<String> {
        ["remove", "--id", action, "--recording", &id]
            .map(str::to_owned)
            .to_vec()
    };
    let expected = [remove(&format!("{id}#1")), remove(&format!("{id}@00:00#3"))];
    assert_eq!(sent(&withdrawn), expected);
    assert_eq!(analyze(&sandbox, &id)["actions"], counts(0, 0, 0, 0));
    assert_eq!(sent(&withdrawn), expected);
    assert_eq!(sent(&log).len(), 4);
    let details: Vec<_> = crate::report_hook::events(&capture)
        .into_iter()
        .filter(|event| event["event"] == "actions_sent")
        .map(|event| event["detail"].clone())
        .collect();
    assert_eq!(
        details,
        [
            "4 sent, 0 failed, 0 refused, 0 withdrawn",
            "0 sent, 0 failed, 0 refused, 2 withdrawn"
        ]
    );
    fs::write(sandbox.path().join("captured.proposal"), original).unwrap();
    assert_eq!(analyze(&sandbox, &id)["actions"], counts(2, 0, 0, 0));
    assert_eq!(sent(&log).len(), 6);
}

#[test]
fn a_failed_withdrawal_is_retried_by_the_next_run_and_then_not_repeated() {
    let (sandbox, id, _, withdrawn) = configured("");
    analyze(&sandbox, &id);
    keep_first_two(&sandbox);
    fs::write(withdrawn.with_extension("fail"), "1").unwrap();
    let result = analyze(&sandbox, &id);
    assert_eq!(result["actions"], counts(0, 2, 0, 0));
    assert_eq!(
        result["warnings"],
        json!(["configured withdraw command failed"])
    );
    fs::remove_file(withdrawn.with_extension("fail")).unwrap();
    assert_eq!(analyze(&sandbox, &id)["actions"], counts(0, 0, 0, 2));
    assert_eq!(analyze(&sandbox, &id)["actions"], counts(0, 0, 0, 0));
    assert_eq!(attempts(&withdrawn), 4);
}

#[test]
fn a_withdrawal_refused_with_a_final_code_is_counted_and_never_retried() {
    let (sandbox, id, log, withdrawn) = configured("final_exit_codes = [4]\n");
    analyze(&sandbox, &id);
    keep_first_two(&sandbox);
    fs::write(withdrawn.with_extension("fail"), "4").unwrap();
    let result = analyze(&sandbox, &id);
    assert_eq!(result["actions"], counts(0, 0, 2, 0));
    assert_eq!(
        result["warnings"],
        json!(["configured withdraw command refused a withdrawal"])
    );
    fs::remove_file(withdrawn.with_extension("fail")).unwrap();
    let mut resend = sandbox.vpt();
    resend.args(["analyze", &id, "--resend-refused", "--json"]);
    let resent = note::success(&support::run_with_budget(
        &mut resend,
        Duration::from_secs(5),
    ));
    assert_eq!(resent["actions"], counts(0, 0, 0, 0));
    analyze(&sandbox, &id);
    assert_eq!(attempts(&withdrawn), 2);
    assert_eq!(attempts(&log), 4);
}

#[test]
fn an_action_sent_then_changed_while_its_send_fails_and_then_dropped_is_withdrawn_once() {
    let (sandbox, id, log, withdrawn) = configured("");
    analyze(&sandbox, &id);
    let proposal = sandbox.path().join("captured.proposal");
    let changed = fs::read_to_string(&proposal)
        .unwrap()
        .replace("maybe follow up secretly", "maybe follow up later");
    fs::write(&proposal, changed).unwrap();
    fs::write(log.with_extension("fail"), "1").unwrap();
    assert_eq!(analyze(&sandbox, &id)["actions"], counts(0, 1, 0, 0));
    keep_first_two(&sandbox);
    assert_eq!(analyze(&sandbox, &id)["actions"], counts(0, 0, 0, 2));
    assert_eq!(analyze(&sandbox, &id)["actions"], counts(0, 0, 0, 0));
    let ids: Vec<_> = sent(&withdrawn)
        .into_iter()
        .map(|args| args[2].clone())
        .collect();
    assert_eq!(ids, [format!("{id}#1"), format!("{id}@00:00#3")]);
}

#[test]
fn a_dry_run_withdraws_nothing() {
    let (sandbox, id, _, withdrawn) = configured("");
    analyze(&sandbox, &id);
    keep_first_two(&sandbox);
    note::success(&invoke(&sandbox, &id, true));
    assert_eq!(attempts(&withdrawn), 0);
}

#[test]
fn a_withdraw_command_without_an_actions_command_is_a_config_error() {
    let (sandbox, record) = note::configured();
    agent(
        &sandbox,
        "[actions]\nwithdraw_command = ['todo-tool', 'remove', '--id', '{id}']\n",
    );
    let output = invoke(&sandbox, record.id.as_str(), false);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("actions.withdraw_command"));
}

#[test]
fn a_withdraw_command_with_a_placeholder_it_cannot_fill_is_a_config_error() {
    let (sandbox, record) = note::configured();
    agent(
        &sandbox,
        "[actions]\ncommand = ['todo-tool', 'add', '{id}']\nwithdraw_command = ['todo-tool', 'remove', '{id}', '--', '{text}']\n",
    );
    let output = invoke(&sandbox, record.id.as_str(), false);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("actions.withdraw_command"));
    assert!(stderr(&output).contains("{text}"));
}
