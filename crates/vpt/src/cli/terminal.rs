mod actions;
mod analyze;
mod confirm;
mod ingest;
mod list;
mod note;
mod redact;
mod review;
mod run;
mod search;
mod show;
mod storage;
mod tag;
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
        Verb::List { .. } => Some(list::render),
        Verb::Storage => Some(storage::render),
        Verb::Search(_) => Some(search::render),
        Verb::Show { stage: None, .. } => Some(show::render),
        Verb::Review { .. } => Some(review::render),
        Verb::ConfirmTerm { .. } | Verb::ConfirmTag { .. } | Verb::ConfirmRelation { .. } => {
            Some(confirm::render)
        }
        Verb::TagRename { .. } => Some(tag::render),
        Verb::NoteWrite { .. } => Some(note::write),
        Verb::VerifyNote { .. } => Some(note::verify),
        Verb::Redact(_) => Some(redact::render),
        _ => None,
    }
}

fn items<'a>(document: &'a Value, key: &str) -> impl Iterator<Item = &'a Value> {
    document[key].as_array().into_iter().flatten()
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().unwrap_or_default()
}
