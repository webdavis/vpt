use super::super::{WithdrawOutcome, withdraw_action};
use crate::config::{from_table, load_text};
use std::path::Path;
use vpt_application::Settings;
use vpt_domain::identity::RecordingId;

const TOKEN: &str = "made-up-withdraw-env-fixture-not-a-credential";

fn settings(withdraw_command: &str) -> Settings {
    from_table(
        &load_text(&format!(
            "config_version = 1\n[actions]\ncommand = ['todo-tool', 'add', '{{id}}']\nwithdraw_command = {withdraw_command}\nfinal_exit_codes = [1, 9, 15, 124, 137, 143]\n[actions.env]\nTODO_TOOL_TOKEN = {TOKEN:?}\n"
        ))
        .unwrap(),
        Path::new("/u"),
    )
    .unwrap()
}

fn recording() -> RecordingId {
    RecordingId::parse("2026-08-24T144736-4f3ab19c02de").unwrap()
}

#[test]
fn the_withdraw_command_gets_the_configured_env_and_nothing_on_standard_input() {
    let dir = crate::test_temp::tempdir().unwrap();
    let seen = dir.path().join("seen");
    let script = "printf '%s|%s|' \"$TODO_TOOL_TOKEN\" \"$1\" > \"$0\"; /bin/cat >> \"$0\"";
    let settings = settings(&format!(
        "['/bin/sh', '-c', {script:?}, {seen:?}, '{{id}}']"
    ));
    let outcome = withdraw_action(&settings, &recording(), "x#1");
    assert_eq!(outcome, WithdrawOutcome::Withdrawn);
    assert_eq!(
        std::fs::read_to_string(&seen).unwrap(),
        format!("{TOKEN}|x#1|")
    );
}
