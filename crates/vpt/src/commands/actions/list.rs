use super::rows::{self, Row};
use crate::{cli::output::Outcome, style};
use vpt_application::ports::{ActionLedger, ProposalLedger, SendState};
use vpt_protocol::result::document;

pub(crate) fn run(ledger: &(impl ActionLedger + ProposalLedger), id: Option<&str>) -> Outcome {
    let listed =
        rows::recording(id).and_then(|only| rows::read(ledger, only.as_ref(), &SendState::ALL));
    match listed {
        Ok(rows) => Outcome::Success {
            completed: Vec::new(),
            diagnostics: Vec::new(),
            human: human(&rows),
            document: document(
                "actions list",
                serde_json::json!({"actions": rows.iter().map(Row::json).collect::<Vec<_>>()}),
            ),
        },
        Err(error) => Outcome::Failure(*error),
    }
}

pub(super) fn human(rows: &[Row]) -> String {
    if rows.is_empty() {
        return "no follow-ups\n".into();
    }
    let mut text = String::new();
    for (index, row) in rows.iter().enumerate() {
        if index == 0 || rows[index - 1].recording != row.recording {
            let count = rows
                .iter()
                .filter(|other| other.recording == row.recording)
                .count();
            text.push_str(&style::heading(row.recording.as_str(), &follow_ups(count)));
        }
        let shown = row
            .text
            .as_deref()
            .unwrap_or(&row.id)
            .escape_debug()
            .to_string();
        text.push_str(&style::row(verdict(row.state), row.state.name(), &shown));
    }
    text
}

fn follow_ups(count: usize) -> String {
    match count {
        1 => "1 follow-up".into(),
        count => format!("{count} follow-ups"),
    }
}

fn verdict(state: SendState) -> style::Verdict {
    match state {
        SendState::Sent => style::Verdict::Ok,
        SendState::Pending => style::Verdict::NotRun,
        SendState::Refused => style::Verdict::Issue,
        SendState::Withdrawing => style::Verdict::Warning,
    }
}
