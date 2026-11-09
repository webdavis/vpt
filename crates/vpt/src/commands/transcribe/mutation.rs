use super::{observe, work};
use crate::cli::output::Outcome;
use crate::compose::{
    errors::{contained_error, open_error},
    events::attach_error,
};
use std::collections::BTreeSet;
use vpt_adapters::{HelperClient, LoggedEvents, Roots, SqliteLedger, SystemClock};
use vpt_application::{
    Settings,
    ports::{Clock, RunHistory},
};
use vpt_domain::identity::RecordingId;
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
    crate::compose::history::recover(&ledger, &mut sink, &mut warnings)
        .map_err(|error| attach_error(error, &warnings))?;
    let helper = HelperClient::new(settings.helper_path.clone());
    let retention = crate::compose::startup::repair(
        &ledger,
        &mut sink,
        &helper,
        settings,
        roots,
        &mut warnings,
    )
    .map_err(|error| attach_error(error, &warnings))?;
    warnings.extend(retention.retention.warnings.iter().copied());
    let result = (|| {
        let record = observe::record(&ledger, id)?;
        let duration = observe::audio(&record, settings, roots)?;
        let run = crate::compose::history::commit(
            ledger.begin_run(vpt_domain::run::RunKind::Transcribe, SystemClock.now()),
            &mut sink,
            &mut warnings,
        )?
        .run;
        roots.create_leaves().map_err(|error| {
            crate::compose::config_refusal::finish_run(
                &ledger,
                &mut sink,
                run,
                error,
                &mut warnings,
            )
        })?;
        let stores = vpt_adapters::FilesystemStores::open(&[
            roots.stores.engine_outputs.clone(),
            roots.stores.transcripts.clone(),
            roots.stores.analysis.clone(),
        ])
        .map_err(|error| Box::new(contained_error(error)))?
        .with_private_roots(std::slice::from_ref(&roots.stores.engine_outputs));
        work::run(
            work::Context {
                mentions: None,
                notices: None,
                ledger: &ledger,
                stores: &stores,
                settings,
                roots,
                sink: &mut sink,
                warnings: &mut warnings,
            },
            record,
            duration,
            run,
        )
    })();
    let outcome = crate::compose::events::attach(result, &warnings);
    Ok(crate::compose::startup::attach(outcome, retention))
}
