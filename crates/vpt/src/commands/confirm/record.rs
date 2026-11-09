use super::Request;
mod changes;
use crate::{
    cli::output::Outcome,
    commands::note,
    compose::{
        errors::{contained_error, ledger_error},
        history::commit,
        publication,
    },
};
use std::collections::BTreeSet;
use vpt_adapters::{FilesystemStores, Roots, SqliteLedger, SystemClock};
use vpt_application::{Settings, ports::*};
use vpt_domain::run::*;
use vpt_protocol::error::ErrorDocument;

pub(in crate::commands::confirm) fn run(
    request: &Request<'_>,
    settings: &Settings,
    roots: &Roots,
    ledger: &SqliteLedger,
    sink: &mut impl EventSink,
    warnings: &mut BTreeSet<SinkWarning>,
    committed: &mut bool,
) -> Result<Outcome, Box<ErrorDocument>> {
    let id = request
        .recording()
        .ok_or_else(|| super::usage("recording is required"))?;
    let inputs = ledger
        .note_inputs(id)
        .map_err(|error| Box::new(ledger_error(error)))?
        .ok_or_else(note::errors::not_found)?;
    let run = commit(
        ledger.begin_run(RunKind::Confirm, SystemClock.now()),
        sink,
        warnings,
    )?
    .run;
    let result = (|| {
        let render = inputs.has_renderable_notes();
        let stores = if render {
            roots.create_leaves().map_err(|error| {
                crate::compose::config_refusal::in_run(ledger, sink, run, error, warnings)
            })?;
            FilesystemStores::open(&[
                roots.stores.transcripts.clone(),
                roots.stores.analysis.clone(),
            ])
        } else {
            FilesystemStores::open_read_only(&[])
        }
        .map_err(|error| Box::new(contained_error(error)))?;
        let at = SystemClock.now();
        let prepared = changes::prepare(request, inputs, settings, roots, &stores, ledger, at)?;
        let delta = prepared.delta;
        let mut facts = note::history::facts(&delta, id, at);
        if let Some(saved) = &delta.vocabulary {
            facts.push((
                at,
                HistoryFact::EffectIntent {
                    kind: EffectKind::Publication,
                    reference: EffectReference::PublicationTarget(saved.entry.target.clone()),
                },
            ));
        }
        if let Err(error) = commit(ledger.commit_step(run, &delta, &facts), sink, warnings) {
            *committed = error.completed.contains(&id.to_string());
            return Err(error);
        }
        *committed = true;
        publication::repair(ledger, &stores, settings, sink, warnings)?;
        let record = ledger
            .by_id(id)
            .map_err(|error| Box::new(ledger_error(error)))?
            .ok_or_else(note::errors::not_found)?;
        let record = crate::documents::record_json(&record).map_err(Box::new)?;
        Ok(Outcome::Success {
            completed: Vec::new(),
            diagnostics: Vec::new(),
            document: vpt_protocol::result::document(
                "confirm",
                serde_json::json!({"record": record, "confirmed": prepared.confirmed, "index_diagnostics": prepared.diagnostics}),
            ),
            human: format!("{} for {id}\n", prepared.message),
        })
    })();
    let at = SystemClock.now();
    let mut facts = vec![];
    if result.is_err() && !*committed {
        facts.push((
            at,
            HistoryFact::WorkStep {
                recording: Some(id.clone()),
                outcome: WorkOutcome::Failed,
            },
        ));
    }
    facts.push((
        at,
        HistoryFact::RunFinished(if result.is_ok() {
            RunOutcome::Completed
        } else {
            RunOutcome::Failed
        }),
    ));
    commit(
        ledger.commit_step(run, &WorkDelta::default(), &facts),
        sink,
        warnings,
    )?;
    result
}
