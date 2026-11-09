use super::{Request, changes, errors};
use crate::{
    cli::output::Outcome,
    compose::{
        errors::{contained_error, ledger_error},
        history::commit,
    },
};
use std::collections::BTreeSet;
use vpt_adapters::{FilesystemStores, Roots, SqliteLedger, SystemClock};
use vpt_application::{Settings, ports::*};
use vpt_domain::run::{HistoryFact, RunKind, RunOutcome, WorkOutcome};
use vpt_protocol::error::ErrorDocument;

pub(super) fn run(
    request: &Request<'_>,
    settings: &Settings,
    roots: &Roots,
    ledger: &SqliteLedger,
    sink: &mut impl EventSink,
    warnings: &mut BTreeSet<SinkWarning>,
    committed: &mut bool,
) -> Result<Outcome, Box<ErrorDocument>> {
    let inputs = ledger
        .note_inputs(&request.id)
        .map_err(|error| Box::new(ledger_error(error)))?
        .ok_or_else(|| errors::usage("recording not found"))?;
    let stage = inputs
        .review
        .flags
        .iter()
        .find(|flag| flag.identifier == request.flag)
        .and_then(|flag| match &flag.content {
            vpt_domain::review::FlagContent::Verification(finding) => Some(finding.artifact),
            _ => None,
        })
        .unwrap_or(vpt_application::ports::NoteStage::Transcript);
    let run = commit(
        ledger.begin_run(RunKind::Review, SystemClock.now()),
        sink,
        warnings,
    )?
    .run;
    let result = (|| {
        roots.create_leaves().map_err(|error| {
            crate::compose::config_refusal::in_run(ledger, sink, run, error, warnings)
        })?;
        let stores = FilesystemStores::open(&[
            roots.stores.transcripts.clone(),
            roots.stores.analysis.clone(),
        ])
        .map_err(|error| Box::new(contained_error(error)))?;
        let at = SystemClock.now();
        let delta = changes::prepare(request, inputs, settings, roots, &stores, at)?;
        if let Err(error) = commit(
            ledger.commit_step(run, &delta, &changes::facts(&delta, request, at)),
            sink,
            warnings,
        ) {
            *committed = error.completed.contains(&request.id.to_string());
            return Err(error);
        }
        *committed = true;
        crate::compose::publication::repair(ledger, &stores, settings, sink, warnings)?;
        super::output::success(
            ledger,
            request,
            delta
                .notes
                .iter()
                .find(|change| change.artifact.stage == stage)
                .map(|change| change.artifact.target.as_path()),
            delta.resolution.is_some(),
        )
    })();
    let at = SystemClock.now();
    let mut facts = Vec::new();
    if result.is_err() && !*committed {
        facts.push((
            at,
            HistoryFact::WorkStep {
                recording: Some(request.id.clone()),
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
