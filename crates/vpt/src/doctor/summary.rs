use super::checks::Context;
use super::parts::{command, not_configured, not_tested};
use crate::report::Row;

pub(super) fn rows(context: &Context) -> Vec<Row> {
    let configured = &context.settings.analysis.command;
    if configured.is_empty() {
        return vec![not_configured("command", "analysis.command")];
    }
    vec![
        command(
            "command",
            configured,
            "analysis.command",
            "it receives the full transcript, and whatever it calls sees it",
        ),
        not_tested("summary"),
    ]
}
