use crate::{
    cli::output::Outcome,
    compose::{errors::open_error, events::attach, history::recover},
};
use std::collections::BTreeSet;
use vpt_adapters::{HelperClient, LoggedEvents, Roots, SqliteLedger, SystemClock};
use vpt_application::{Settings, ports::Clock};
use vpt_protocol::error::ErrorDocument;

pub(super) fn run(
    settings: &Settings,
    roots: &Roots,
    request: &super::Request<'_>,
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
    let mut retention = None;
    let mut committed = false;
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
        match request {
            super::Request::Term(term) => {
                super::execute::run(&ledger, &mut sink, &mut warnings, settings, term)
            }
            super::Request::Tag(_) | super::Request::Relation(_) => super::record::run(
                request,
                settings,
                roots,
                &ledger,
                &mut sink,
                &mut warnings,
                &mut committed,
            ),
        }
    })();
    if let Some(id) = request.recording() {
        return Ok(crate::commands::note::output::finish(
            result, warnings, id, committed, retention,
        ));
    }
    let outcome = attach(result, &warnings);
    Ok(match retention {
        Some(retention) => crate::compose::startup::attach(outcome, retention),
        None => outcome,
    })
}
