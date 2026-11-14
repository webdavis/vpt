mod output;
mod select;
use super::mutation;
use crate::{
    cli::{args::analyze::Scope, output::Outcome},
    compose::{Environment, RuntimeLedger, actions::Counts},
};
use std::collections::BTreeSet;
use std::path::Path;
use vpt_adapters::{RootDir, Roots};
use vpt_application::{Settings, ports::*};
use vpt_domain::{identity::RecordingId, tags::State};
use vpt_protocol::error::ErrorDocument;

enum Entry {
    Planned(RecordingId),
    Succeeded(RecordingId, Change),
    Failed(Failure),
}

struct Failure {
    id: String,
    error: Box<ErrorDocument>,
}

struct Change {
    summary_changed: bool,
    tags_added: usize,
    tags_removed: usize,
    actions: Counts,
}

#[derive(Default)]
struct Snapshot {
    summary: Vec<String>,
    tags: BTreeSet<String>,
}

pub(crate) fn run(
    environment: &Environment,
    config: Option<&Path>,
    scope: &Scope,
    dry: bool,
) -> Outcome {
    execute(environment, config, scope, dry).unwrap_or_else(|error| Outcome::Failure(*error))
}

fn execute(
    environment: &Environment,
    config: Option<&Path>,
    scope: &Scope,
    dry: bool,
) -> Result<Outcome, Box<ErrorDocument>> {
    let (settings, roots) = super::configured(environment, config)?;
    let ledger = RuntimeLedger::read_only(&roots.state_dir)?;
    let candidates = select::candidates(scope, &ledger)?;
    if dry {
        let entries = candidates
            .into_iter()
            .map(|candidate| candidate.map_or_else(Entry::Failed, Entry::Planned))
            .collect();
        return Ok(output::report(entries, true, false));
    }
    let state = crate::compose::mutation::state(&roots)?;
    let _lock = crate::compose::mutation::lock(&state)?;
    let ledger = RuntimeLedger::read_only(&roots.state_dir)?;
    let entries = candidates
        .into_iter()
        .map(|candidate| match candidate {
            Ok(id) => redo(id, &settings, &roots, &state, &ledger),
            Err(failure) => Entry::Failed(failure),
        })
        .collect();
    Ok(output::report(
        entries,
        false,
        !settings.actions_command.is_empty(),
    ))
}

fn redo(
    id: RecordingId,
    settings: &Settings,
    roots: &Roots,
    state: &RootDir,
    ledger: &RuntimeLedger,
) -> Entry {
    let before = snapshot(ledger, &id);
    match mutation::locked(&id, settings, roots, state, false) {
        Outcome::Success { document, .. } => {
            let after = snapshot(ledger, &id);
            let change = Change {
                summary_changed: before.summary != after.summary,
                tags_added: after.tags.difference(&before.tags).count(),
                tags_removed: before.tags.difference(&after.tags).count(),
                actions: Counts::parse(&document["actions"]),
            };
            Entry::Succeeded(id, change)
        }
        Outcome::Failure(error)
        | Outcome::ParseFailure(error)
        | Outcome::FailedReport { error, .. } => Entry::Failed(Failure {
            id: id.to_string(),
            error: Box::new(error),
        }),
    }
}

fn snapshot(ledger: &RuntimeLedger, id: &RecordingId) -> Snapshot {
    let Ok(Some(input)) = ledger.note_inputs(id) else {
        return Snapshot::default();
    };
    Snapshot {
        summary: input
            .proposal
            .map(|accepted| {
                accepted
                    .proposal
                    .summary
                    .into_iter()
                    .map(|claim| claim.claim.text)
                    .collect()
            })
            .unwrap_or_default(),
        tags: input
            .tags
            .unwrap_or_default()
            .into_iter()
            .filter(|tag| tag.state != State::Rejected)
            .map(|tag| tag.name)
            .collect(),
    }
}
