mod actions;
mod analyze;
mod ingest;
mod run;
mod transcribe;

use super::{args::Verb, format::Format, output::Outcome};
use serde_json::Value;
use std::io::IsTerminal;

type Render = fn(&Value, &[String]) -> String;

/// Replaces a successful command's plain text with its styled form when standard output is a
/// terminal, so a pipe or a file keeps receiving the plain lines it always has.
pub(crate) fn styled(outcome: Outcome, verb: &Verb, format: Option<Format>) -> Outcome {
    let Some(render) = render(verb) else {
        return outcome;
    };
    match outcome {
        Outcome::Success {
            document,
            completed,
            diagnostics,
            ..
        } if format.is_none() && std::io::stdout().is_terminal() => Outcome::Success {
            human: render(&document, &diagnostics),
            document,
            completed,
            diagnostics,
        },
        outcome => outcome,
    }
}

fn render(verb: &Verb) -> Option<Render> {
    match verb {
        Verb::Run { .. } => Some(run::render),
        Verb::Ingest { .. } | Verb::Import(_) => Some(ingest::render),
        Verb::Transcribe { .. } => Some(transcribe::render),
        Verb::Analyze { .. } => Some(analyze::one),
        Verb::AnalyzeBatch { .. } => Some(analyze::batch),
        _ => None,
    }
}

fn items<'a>(document: &'a Value, key: &str) -> impl Iterator<Item = &'a Value> {
    document[key].as_array().into_iter().flatten()
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().unwrap_or_default()
}
