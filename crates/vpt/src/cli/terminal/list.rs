use super::{items, text};
use crate::style::{Summary, Verdict};
use serde_json::Value;

const STAGES: [&str; 3] = ["transcribe", "note", "analysis"];

pub(super) fn render(document: &Value, _: &[String]) -> String {
    let mut summary = Summary::default();
    for recording in items(document, "recordings") {
        let states = STAGES.map(|stage| text(&recording["stages"], stage));
        let finished = |state: &&str| matches!(*state, "succeeded" | "disabled");
        let verdict = match states {
            states if states.contains(&"failed") => Verdict::Issue,
            states if states.iter().all(finished) => Verdict::Ok,
            _ => Verdict::NotRun,
        };
        let title = recording["title"].as_str().unwrap_or("(untitled)");
        let outstanding: Vec<_> = STAGES
            .iter()
            .zip(states)
            .filter(|(_, state)| *state != "succeeded")
            .map(|(stage, state)| format!("{stage} {state}"))
            .collect();
        let detail = match outstanding.is_empty() {
            true => title.escape_debug().to_string(),
            false => format!("{} ({})", title.escape_debug(), outstanding.join(", ")),
        };
        summary.row(verdict, text(recording, "id"), &detail);
    }
    summary.finish("no recordings")
}
