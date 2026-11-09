use super::errors;
use crate::{
    cli::output::Outcome,
    compose::{
        errors::{contained_error, ledger_error, open_error},
        history::{commit, recover},
    },
};
use std::collections::BTreeSet;
use vpt_adapters::{
    FilesystemStores, HelperClient, LoggedEvents, Roots, SqliteLedger, SystemClock,
};
use vpt_application::{Settings, ports::*};
use vpt_domain::{
    identity::RecordingId,
    run::{HistoryFact, RunKind, RunOutcome},
};
use vpt_protocol::error::ErrorDocument;

pub(super) fn run(
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
    let mut retention_report = None;
    let result = (|| {
        recover(&ledger, &mut sink, &mut warnings)?;
        let retention = crate::compose::startup::repair(
            &ledger,
            &mut sink,
            &HelperClient::new(settings.helper_path.clone()),
            settings,
            roots,
            &mut warnings,
        )?;
        warnings.extend(retention.retention.warnings.iter().copied());
        retention_report = Some(retention);
        let inputs = ledger
            .note_inputs(id)
            .map_err(|error| Box::new(ledger_error(error)))?
            .ok_or_else(errors::not_found)?;
        let run = commit(
            ledger.begin_run(RunKind::Note, SystemClock.now()),
            &mut sink,
            &mut warnings,
        )?
        .run;
        let written = (|| {
            roots.create_leaves().map_err(|error| {
                crate::compose::config_refusal::in_run(
                    &ledger,
                    &mut sink,
                    run,
                    error,
                    &mut warnings,
                )
            })?;
            let stores = FilesystemStores::open(&[
                roots.stores.transcripts.clone(),
                roots.stores.analysis.clone(),
            ])
            .map_err(|error| Box::new(contained_error(error)))?;
            let mentions = crate::compose::mentions::Context::read(settings, roots)?;
            super::write::Writer {
                mentions: &mentions,
                ledger: &ledger,
                stores: &stores,
                settings,
                options: vpt_application::note::WriteOptions {
                    analysis_content: None,
                    roots: &roots.stores,
                    regenerate_expired: true,
                },
                sink: &mut sink,
                warnings: &mut warnings,
            }
            .write(inputs, run, &mut committed)
        })();
        commit(
            ledger.commit_step(
                run,
                &WorkDelta::default(),
                &[(
                    SystemClock.now(),
                    HistoryFact::RunFinished(if written.is_ok() {
                        RunOutcome::Completed
                    } else {
                        RunOutcome::Failed
                    }),
                )],
            ),
            &mut sink,
            &mut warnings,
        )?;
        let target = written?;
        let record = ledger
            .by_id(id)
            .map_err(|error| Box::new(ledger_error(error)))?
            .ok_or_else(errors::not_found)?;
        super::output::success(&record, target)
    })();
    Ok(super::output::finish(
        result,
        warnings,
        id,
        committed,
        retention_report,
    ))
}
