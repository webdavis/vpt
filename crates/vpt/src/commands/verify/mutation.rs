use super::{errors, persist};
use crate::{
    cli::output::Outcome,
    compose::{
        errors::{ledger_error, open_error},
        history::{commit, recover},
    },
};
use std::{collections::BTreeSet, path::Path};
use vpt_adapters::{HelperClient, LoggedEvents, Roots, SqliteLedger, SystemClock};
use vpt_application::{Settings, ports::*};
use vpt_domain::{
    identity::RecordingId,
    run::{HistoryFact, RunKind, RunOutcome, WorkOutcome},
};
use vpt_protocol::error::ErrorDocument;

pub(super) fn run(
    path: &Path,
    id: &RecordingId,
    settings: &Settings,
    roots: &Roots,
) -> Result<Outcome, Box<ErrorDocument>> {
    let state = crate::compose::mutation::state(roots)?;
    let _lock = crate::compose::mutation::lock(&state)?;
    let ledger = SqliteLedger::open(&state).map_err(|error| Box::new(open_error(error)))?;
    let mut sink = LoggedEvents::new(
        &state,
        &ledger,
        &settings.report_command,
        &settings.alert_command,
        || SystemClock.now(),
    );
    let mut warnings = BTreeSet::new();
    let mut committed = false;
    let mut retention = None;
    let result = (|| {
        recover(&ledger, &mut sink, &mut warnings)?;
        let recovered = crate::compose::startup::repair(
            &ledger,
            &mut sink,
            &HelperClient::new(settings.helper_path.clone()),
            settings,
            roots,
            &mut warnings,
        )?;
        warnings.extend(recovered.retention.warnings.iter().copied());
        retention = Some(recovered);
        let input = ledger
            .note_inputs(id)
            .map_err(|error| Box::new(ledger_error(error)))?
            .ok_or_else(|| errors::usage("recording not found"))?;
        let run = commit(
            ledger.begin_run(RunKind::VerifyNote, SystemClock.now()),
            &mut sink,
            &mut warnings,
        )?
        .run;
        let result = persist::Writer {
            settings,
            roots,
            ledger: &ledger,
            sink: &mut sink,
            warnings: &mut warnings,
        }
        .run(path, input, run, &mut committed);
        let at = SystemClock.now();
        let mut facts = Vec::new();
        if result.is_err() && !committed {
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
            &mut sink,
            &mut warnings,
        )?;
        result
    })();
    Ok(crate::commands::note::output::finish(
        result, warnings, id, committed, retention,
    ))
}
