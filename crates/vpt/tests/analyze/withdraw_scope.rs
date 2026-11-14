use super::*;
use crate::actions::{configure_with, propose, sent};
use crate::withdraw::{analyze, attempts, configured, counts, keep_first_two, withdraw_setting};
use serde_json::json;
use std::os::unix::process::ExitStatusExt;

const RECORD_THEN_FAIL: &str = "#!/bin/sh\nlog=\"$1\"\nshift\nprintf '%s\\037' \"$@\" >> \"$log\"\nprintf '\\n' >> \"$log\"\n[ -e \"$log.fail\" ] && exit 1\nexit 0\n";

const RECORD_THEN_CRASH_ON_FIRST_UNTIMED: &str = "#!/bin/sh\nlog=\"$1\"\nshift\nprintf '%s\\037' \"$@\" >> \"$log\"\nprintf '\\n' >> \"$log\"\ncase \"$1 $3\" in \"add \"*'#1') kill -9 \"$PPID\";; esac\nexit 0\n";

fn keep_first(sandbox: &Sandbox) {
    let proposal = fs::read_to_string(sandbox.path().join("captured.proposal")).unwrap();
    let proposal: serde_json::Value = serde_json::from_str(&proposal).unwrap();
    let kept = &proposal["actions"].as_array().unwrap()[..1];
    propose(sandbox, &json!(kept).to_string());
}

fn withdrawn_ids(withdrawn: &std::path::Path) -> Vec<String> {
    sent(withdrawn)
        .into_iter()
        .map(|args| args[2].clone())
        .collect()
}

#[test]
fn actions_dropped_before_the_withdraw_command_was_set_are_never_withdrawn() {
    let (sandbox, mut rows) = note::configured_saying(&[7], "um private words tomorrow");
    let id = rows.remove(0).id.as_str().to_owned();
    configure_with(&sandbox, "");
    analyze(&sandbox, &id);
    keep_first_two(&sandbox);
    analyze(&sandbox, &id);
    let proposal = sandbox.path().join("captured.proposal");
    let kept = fs::read_to_string(&proposal).unwrap();
    let (setting, withdrawn) = withdraw_setting(&sandbox);
    configure_with(&sandbox, &setting);
    fs::write(&proposal, kept).unwrap();
    assert_eq!(analyze(&sandbox, &id)["actions"], counts(0, 0, 0, 0));
    assert_eq!(attempts(&withdrawn), 0);
    keep_first(&sandbox);
    assert_eq!(analyze(&sandbox, &id)["actions"], counts(0, 0, 0, 1));
    assert_eq!(withdrawn_ids(&withdrawn), [format!("{id}@00:00#2")]);
}

#[test]
fn a_send_that_failed_after_the_tool_stored_the_item_is_withdrawn_when_dropped() {
    let (sandbox, id, log, withdrawn) = configured("");
    fs::write(sandbox.path().join("todo-tool.sh"), RECORD_THEN_FAIL).unwrap();
    fs::write(log.with_extension("fail"), "").unwrap();
    assert_eq!(analyze(&sandbox, &id)["actions"], counts(0, 4, 0, 0));
    fs::remove_file(log.with_extension("fail")).unwrap();
    keep_first_two(&sandbox);
    assert_eq!(analyze(&sandbox, &id)["actions"], counts(2, 0, 0, 2));
    assert_eq!(
        withdrawn_ids(&withdrawn),
        [format!("{id}#1"), format!("{id}@00:00#3")]
    );
}

#[test]
fn a_send_vpt_dies_during_is_withdrawn_when_dropped_and_an_unattempted_one_is_not() {
    let (sandbox, id, log, withdrawn) = configured("");
    fs::write(
        sandbox.path().join("todo-tool.sh"),
        RECORD_THEN_CRASH_ON_FIRST_UNTIMED,
    )
    .unwrap();
    let crashed = invoke(&sandbox, &id, false);
    assert_eq!(crashed.status.signal(), Some(9), "{}", stderr(&crashed));
    assert_eq!(sent(&log).len(), 3);
    keep_first_two(&sandbox);
    assert_eq!(analyze(&sandbox, &id)["actions"], counts(0, 0, 0, 1));
    assert_eq!(analyze(&sandbox, &id)["actions"], counts(0, 0, 0, 0));
    assert_eq!(withdrawn_ids(&withdrawn), [format!("{id}#1")]);
}
