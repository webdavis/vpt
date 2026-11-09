mod startup;
pub(super) use startup::repair as startup;
pub(super) mod output;
use super::work::Context;
use crate::compose::{RuntimeLedger, errors::contained_error};
use vpt_adapters::{FilesystemStores, HelperClient, Roots, SystemClock};
use vpt_application::{
    ReconcileProgress, RetentionFailure, RetentionReport, Settings, ports::EventSink,
};
use vpt_domain::layout::StoreKey;
use vpt_protocol::error::ErrorDocument;

pub(super) fn run<E: EventSink>(
    context: &mut Context<'_, E>,
    stores: &FilesystemStores,
    run: vpt_domain::history::RunId,
    report: &mut super::Report,
) -> Result<(), Box<ErrorDocument>> {
    if !context.settings.retention.enabled {
        return Ok(());
    }
    let result = vpt_application::Retention {
        ledger: context.ledger,
        stores,
        trash: context.helper,
        sink: context.sink,
        clock: &SystemClock,
        settings: &context.settings.retention,
        paths: &context.roots.stores,
    }
    .run_in(run);
    let result = accept(&mut report.retention, result);
    report.retention_failed |= result.is_err();
    result
}

pub(super) fn observe(
    ledger: &RuntimeLedger,
    settings: &Settings,
    roots: &Roots,
    helper: &HelperClient,
) -> Result<RetentionReport, Box<ErrorDocument>> {
    let mut report = RetentionReport::default();
    if !settings.retention.enabled {
        return Ok(report);
    }
    let paths = StoreKey::all().map(|key| roots.stores.get(key).to_path_buf());
    let stores = FilesystemStores::open_read_only(&paths)
        .map_err(|error| Box::new(contained_error(error)))?
        .with_private_roots(std::slice::from_ref(&roots.stores.engine_outputs));
    let mut sink = crate::compose::events::Observation;
    let result = match ledger {
        RuntimeLedger::Sqlite(ledger) => crate::commands::retention::runtime::execute(
            ledger,
            &stores,
            helper,
            &mut sink,
            &settings.retention,
            &roots.stores,
            true,
            ReconcileProgress::default(),
        ),
        RuntimeLedger::Empty(ledger) => crate::commands::retention::runtime::execute(
            ledger,
            &stores,
            helper,
            &mut sink,
            &settings.retention,
            &roots.stores,
            true,
            ReconcileProgress::default(),
        ),
    };
    accept(&mut report, result)?;
    Ok(report)
}

fn accept(
    report: &mut RetentionReport,
    result: Result<RetentionReport, Box<RetentionFailure>>,
) -> Result<(), Box<ErrorDocument>> {
    let (current, error) = match result {
        Ok(current) => (current, None),
        Err(failure) => (failure.report, Some(failure.error)),
    };
    report.moved.extend(current.moved);
    report.kept.extend(current.kept);
    for id in current.completed {
        if !report.completed.contains(&id) {
            report.completed.push(id);
        }
    }
    report.warnings.extend(current.warnings);
    report.diagnostics.extend(current.diagnostics);
    match error {
        Some(error) => Err(Box::new(crate::commands::retention::errors::document(
            error,
        ))),
        None => Ok(()),
    }
}
