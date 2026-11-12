use crate::{
    cli::{args::search::Options, output::Outcome},
    compose::{Environment, errors::ledger_error, ledger_from},
};
use serde_json::{Value, json};
use std::{collections::HashMap, ops::ControlFlow, path::Path};
use vpt_adapters::note_document::Document;
use vpt_application::ports::{NoteLedger, RecordingLedger};
use vpt_domain::search::{Query, moments, timecode};
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(crate) fn run(environment: &Environment, config: Option<&Path>, options: &Options) -> Outcome {
    execute(environment, config, options).unwrap_or_else(|error| Outcome::Failure(*error))
}

fn execute(
    environment: &Environment,
    config: Option<&Path>,
    options: &Options,
) -> Result<Outcome, Box<ErrorDocument>> {
    let query = Query::new(&options.words)
        .ok_or_else(|| Box::new(ErrorDocument::new(ErrorKind::Usage, "<word> is required")))?;
    let ledger = ledger_from(environment, config)?;
    let titles: HashMap<String, Option<String>> = ledger
        .recordings()
        .map_err(|e| Box::new(ledger_error(e)))?
        .into_iter()
        .map(|record| (record.id.as_str().to_owned(), record.title))
        .collect();
    let mut hits = Vec::new();
    let mut truncated = false;
    let mut failure = None;
    ledger
        .visit_notes(&mut |note| {
            if !on_disk(&note.target) {
                return ControlFlow::Continue(());
            }
            let content =
                match Document::parse(&note.text).and_then(|d| d.content().map(str::to_owned)) {
                    Ok(content) => content,
                    Err(_) => {
                        failure = Some(ErrorDocument::new(
                            ErrorKind::Ledger,
                            "saved note does not parse",
                        ));
                        return ControlFlow::Break(());
                    }
                };
            for moment in moments(note.stage, &content).filter(|m| query.matches(&m.quote)) {
                if hits.len() == options.limit {
                    truncated = true;
                    return ControlFlow::Break(());
                }
                let id = note.recording.as_str();
                let at = moment.seconds.map(timecode);
                hits.push(json!({
                    "moment": at.as_ref().map(|at| format!("{id}@{at}")),
                    "recording": id,
                    "title": titles.get(id).cloned().flatten(),
                    "stage": note.stage.as_str(),
                    "seconds": moment.seconds,
                    "timecode": at,
                    "quote": moment.quote,
                }));
            }
            ControlFlow::Continue(())
        })
        .map_err(|e| Box::new(ledger_error(e)))?;
    if let Some(error) = failure {
        return Err(Box::new(error));
    }
    Ok(success(options, hits, truncated))
}

fn on_disk(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_file())
}

fn printable(text: &str) -> String {
    text.chars()
        .map(|c| match c.is_control() {
            true => c.escape_debug().to_string(),
            false => c.to_string(),
        })
        .collect()
}

fn success(options: &Options, hits: Vec<Value>, truncated: bool) -> Outcome {
    let human: String = hits
        .iter()
        .map(|hit| {
            format!(
                "{}  {}  {}  {}\n",
                hit["moment"]
                    .as_str()
                    .or(hit["recording"].as_str())
                    .unwrap_or_default(),
                printable(hit["title"].as_str().unwrap_or("(untitled)")),
                hit["stage"].as_str().unwrap_or_default(),
                printable(hit["quote"].as_str().unwrap_or_default()),
            )
        })
        .collect();
    Outcome::Success {
        completed: Vec::new(),
        diagnostics: Vec::new(),
        human: if human.is_empty() {
            "no matches\n".into()
        } else {
            human
        },
        document: vpt_protocol::result::document(
            "search",
            json!({"words": options.words, "limit": options.limit, "truncated": truncated, "hits": hits}),
        ),
    }
}
