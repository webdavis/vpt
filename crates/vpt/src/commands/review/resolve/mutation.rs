use super::Request;
use crate::{
    cli::output::Outcome,
    compose::{errors::open_error, history::recover},
};
use std::collections::BTreeSet;
use vpt_adapters::{HelperClient, LoggedEvents, Roots, SqliteLedger, SystemClock};
use vpt_application::{Settings, ports::Clock};
use vpt_protocol::error::ErrorDocument;

pub(super) fn run(
    request: &Request<'_>,
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
        super::execute::run(
            request,
            settings,
            roots,
            &ledger,
            &mut sink,
            &mut warnings,
            &mut committed,
        )
    })();
    Ok(crate::commands::note::output::finish(
        result,
        warnings,
        &request.id,
        committed,
        retention,
    ))
}
