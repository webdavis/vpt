use super::checks::Context;
use super::parts::{command, not_configured, not_tested};
use crate::report::Row;

pub(super) fn rows(context: &Context) -> Vec<Row> {
    let settings = &context.settings;
    if settings.actions_command.is_empty() {
        return vec![not_configured("command", "actions.command")];
    }
    let send = command(
        "command",
        &settings.actions_command,
        "actions.command",
        "it sends each follow-up to whatever it calls",
    );
    let withdraw = &settings.actions_withdraw_command;
    let (withdraw, test) = if withdraw.is_empty() {
        (
            not_configured("withdraw_command", "actions.withdraw_command"),
            not_tested("actions")
                .advice("set `actions.withdraw_command`, then run `vpt test actions`"),
        )
    } else {
        (
            command(
                "withdraw_command",
                withdraw,
                "actions.withdraw_command",
                "it sends each withdrawal to whatever it calls",
            ),
            not_tested("actions"),
        )
    };
    let names: Vec<&str> = settings
        .actions_env
        .0
        .iter()
        .map(|(name, _)| name.as_str())
        .collect();
    let env = if names.is_empty() {
        Row::ok("env", "none set")
    } else {
        Row::ok("env", format!("{} set for both commands", names.join(", ")))
    };
    vec![send, withdraw, env, test]
}
