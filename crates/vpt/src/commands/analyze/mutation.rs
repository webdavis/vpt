use super::{observe, output, resume, work};
use crate::{
    cli::output::Outcome,
    commands::note,
    compose::{
        errors::open_error,
        history::{commit, recover},
    },
};
use std::collections::BTreeSet;
use vpt_adapters::{HelperClient, LoggedEvents, RootDir, Roots, SqliteLedger, SystemClock};
use vpt_application::{Settings, ports::*};
use vpt_domain::{
    identity::RecordingId,
    run::{HistoryFact, RunKind},
};
use vpt_protocol::error::ErrorDocument;

pub(super) fn run(
    id: &RecordingId,
    settings: &Settings,
    roots: &Roots,
) -> Result<Outcome, Box<ErrorDocument>> {
    let state = crate::compose::mutation::state(roots)?;
    let _lock = crate::compose::mutation::lock(&state)?;
    Ok(locked(id, settings, roots, &state))
}

pub(super) fn locked(
    id: &RecordingId,
    settings: &Settings,
    roots: &Roots,
    state: &RootDir,
) -> Outcome {
    let ledger = match SqliteLedger::open(state) {
        Ok(ledger) => ledger,
        Err(error) => return Outcome::Failure(open_error(error)),
    };
    let mut sink = LoggedEvents::new(
        state,
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
        let pending = resume::pending(&ledger, id)?;
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
        let input = observe::inputs(&ledger, id)?;
        if pending && resume::current(&input, settings, roots)? {
            committed = true;
            return output::success(&ledger, id, &[]);
        }
        let run = commit(
            ledger.begin_run(RunKind::Analyze, SystemClock.now()),
            &mut sink,
            &mut warnings,
        )?
        .run;
        let result = work::Writer {
            mentions: None,
            settings,
            roots,
            ledger: &ledger,
            sink: &mut sink,
            warnings: &mut warnings,
        }
        .run(input, run, &mut committed);
        let finished = (|| {
            let at = SystemClock.now();
            let mut facts = Vec::new();
            if result.is_ok() {
                facts.extend(super::notification::fact(&ledger, id, at)?);
            }
            facts.push((
                at,
                HistoryFact::RunFinished(crate::compose::run_outcome::from_result(&result)),
            ));
            commit(
                ledger.commit_step(run, &WorkDelta::default(), &facts),
                &mut sink,
                &mut warnings,
            )
        })();
        finished.map_err(|mut error| {
            match &result {
                Ok(Outcome::Success { diagnostics, .. }) => {
                    error.diagnostics.extend(diagnostics.iter().cloned());
                }
                Err(original) => {
                    error
                        .diagnostics
                        .extend(original.diagnostics.iter().cloned());
                    crate::compose::publication::merge_completed(
                        &mut error,
                        original.completed.clone(),
                    );
                }
                _ => {}
            }
            error
        })?;
        result
    })();
    note::output::finish(result, warnings, id, committed, retention)
}
