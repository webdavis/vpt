use super::*;
use crate::compose::{
    events,
    history::{commit, finish, recover},
    publication, startup,
};
use std::collections::BTreeSet;
use vpt_adapters::{HelperClient, LoggedEvents, SqliteLedger, SystemClock};
use vpt_application::ports::*;
use vpt_domain::run::RunKind;
pub(super) fn run(
    settings: &Settings,
    roots: &Roots,
    options: &Options,
) -> Result<Outcome, Box<ErrorDocument>> {
    let state = crate::compose::mutation::state(roots)?;
    let _lock = crate::compose::mutation::lock(&state)?;
    let ledger = SqliteLedger::open(&state)
        .map_err(|error| Box::new(crate::compose::errors::open_error(error)))?;
    let mut sink = LoggedEvents::new(
        &state,
        &ledger,
        &settings.report_command,
        &settings.alert_command,
        || SystemClock.now(),
    );
    let mut warnings = BTreeSet::new();
    let mut startup_report = None;
    let mut completed = Vec::new();
    let result = (|| {
        recover(&ledger, &mut sink, &mut warnings)?;
        let recovered = startup::repair(
            &ledger,
            &mut sink,
            &HelperClient::new(settings.helper_path.clone()),
            settings,
            roots,
            &mut warnings,
        )?;
        warnings.extend(recovered.retention.warnings.iter().copied());
        startup_report = Some(recovered);
        let run = commit(
            ledger.begin_run(RunKind::Brief, SystemClock.now()),
            &mut sink,
            &mut warnings,
        )?
        .run;
        let result = (|| {
            if matches!(options.target, crate::cli::args::brief::Target::Upcoming) {
                return super::upcoming::write(
                    &ledger,
                    settings,
                    roots,
                    run,
                    &mut sink,
                    &mut warnings,
                    &mut completed,
                );
            }
            let prepared = prepare::run(&ledger, settings, roots, options)?;
            roots.create_leaves().map_err(|error| {
                crate::compose::config_refusal::in_run(
                    &ledger,
                    &mut sink,
                    run,
                    error,
                    &mut warnings,
                )
            })?;
            commit(
                ledger.commit_step(run, &prepared.delta, &prepared.facts),
                &mut sink,
                &mut warnings,
            )?;
            completed =
                publication::repair(&ledger, &stores(roots)?, settings, &mut sink, &mut warnings)?;
            output::success(&prepared.artifact, true, false)
        })();
        finish(&ledger, run, result, &mut sink, &mut warnings)
    })();
    let mut outcome = events::attach(result, &warnings);
    outcome.add_completed(completed);
    if let Some(report) = startup_report {
        outcome = startup::attach(outcome, report);
    }
    Ok(outcome)
}
