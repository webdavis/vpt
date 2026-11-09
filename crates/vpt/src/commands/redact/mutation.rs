use super::write::Writer;
use crate::{
    cli::{args::redact::Options, output::Outcome},
    compose::{
        self,
        history::{commit, recover},
    },
};
use std::{collections::BTreeSet, path::Path};
use vpt_adapters::{HelperClient, LoggedEvents, Roots, SqliteLedger, SystemClock};
use vpt_application::{Settings, ports::*};
use vpt_domain::run::RunKind;
use vpt_protocol::error::ErrorDocument;
pub(super) fn run(
    settings: &Settings,
    roots: &Roots,
    options: &Options,
    destination: &Path,
) -> Result<Outcome, Box<ErrorDocument>> {
    let state = compose::mutation::state(roots)?;
    let _lock = compose::mutation::lock(&state)?;
    let ledger =
        SqliteLedger::open(&state).map_err(|e| Box::new(compose::errors::open_error(e)))?;
    let mut sink = LoggedEvents::new(
        &state,
        &ledger,
        &settings.report_command,
        &settings.alert_command,
        || SystemClock.now(),
    );
    let mut warnings = BTreeSet::new();
    let mut startup = None;
    let mut completed = false;
    let result = (|| {
        recover(&ledger, &mut sink, &mut warnings)?;
        let report = compose::startup::repair(
            &ledger,
            &mut sink,
            &HelperClient::new(settings.helper_path.clone()),
            settings,
            roots,
            &mut warnings,
        )?;
        warnings.extend(report.retention.warnings.iter().copied());
        startup = Some(report);
        let run = commit(
            ledger.begin_run(RunKind::Redact, SystemClock.now()),
            &mut sink,
            &mut warnings,
        )?
        .run;
        let result = Writer {
            ledger: &ledger,
            settings,
            roots,
            options,
            destination,
            sink: &mut sink,
            warnings: &mut warnings,
        }
        .write(run, &mut completed, false);
        let outcome = match result {
            Ok(written) => {
                compose::releases::finish(&ledger, run, Ok(()), &mut sink, &mut warnings)?;
                written
                    .expect("explicit redaction publishes or refuses")
                    .success()
            }
            Err(error) => {
                compose::releases::finish(&ledger, run, Err(error), &mut sink, &mut warnings)?;
                unreachable!("failed release retains its error")
            }
        };
        Ok(outcome)
    })();
    let mut outcome = compose::events::attach(result, &warnings);
    if completed {
        outcome.add_completed([options.id.to_string()]);
    }
    if let Some(report) = startup {
        outcome = compose::startup::attach(outcome, report);
    }
    Ok(outcome)
}
