use super::{Entry, Failure};
use crate::{cli::output::Outcome, compose::actions::Counts};
use serde_json::{Value, json};
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(super) fn report(entries: Vec<Entry>, dry_run: bool, actions: bool) -> Outcome {
    let total = entries.len();
    let rows: Vec<_> = entries.iter().filter_map(row).collect();
    let human: String = entries.iter().map(line).collect();
    let mut completed = Vec::new();
    let mut failures = Vec::new();
    let mut counts = Counts::default();
    for entry in entries {
        match entry {
            Entry::Succeeded(id, change) => {
                completed.push(id.to_string());
                counts += change.actions;
            }
            Entry::Failed(failure) => failures.push(failure),
            Entry::Planned(_) => {}
        }
    }
    if failures.is_empty() {
        let mut document = vpt_protocol::result::document(
            "analyze",
            json!({"dry_run": dry_run, "recordings": rows}),
        );
        if actions {
            document["actions"] = counts.json();
        }
        return Outcome::Success {
            document,
            human: if human.is_empty() {
                "no recordings\n".into()
            } else {
                human
            },
            completed,
            diagnostics: Vec::new(),
        };
    }
    Outcome::Failure(
        ErrorDocument::new(
            ErrorKind::Command,
            format!("{} of {total} recordings failed", failures.len()),
        )
        .diagnostics(failures.iter().map(detail).collect())
        .ids(failures.into_iter().map(|failure| failure.id).collect())
        .completed(completed),
    )
}

fn row(entry: &Entry) -> Option<Value> {
    match entry {
        Entry::Planned(id) => Some(json!({"id": id.as_str(), "status": "planned"})),
        Entry::Succeeded(id, change) => Some(json!({
            "id": id.as_str(), "status": "succeeded", "summary_changed": change.summary_changed,
            "tags_added": change.tags_added, "tags_removed": change.tags_removed,
        })),
        Entry::Failed(_) => None,
    }
}

fn line(entry: &Entry) -> String {
    match entry {
        Entry::Planned(id) => format!("Would analyze {id}\n"),
        Entry::Succeeded(id, change) => format!(
            "Analyzed {id}; summary {}; {} tag(s) added, {} removed\n",
            if change.summary_changed {
                "changed"
            } else {
                "unchanged"
            },
            change.tags_added,
            change.tags_removed
        ),
        Entry::Failed(_) => String::new(),
    }
}

fn detail(failure: &Failure) -> String {
    format!(
        "{}: {}: {}",
        failure.id,
        failure.error.kind.name(),
        failure.error.message
    )
}
