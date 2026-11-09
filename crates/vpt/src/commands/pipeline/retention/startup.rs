use super::*;
use vpt_domain::history::RunId;

pub(in crate::commands::pipeline) fn repair<E: EventSink>(
    context: &mut Context<'_, E>,
    run: RunId,
    report: &mut super::super::Report,
) -> Result<Vec<String>, Box<ErrorDocument>> {
    let paths = StoreKey::all().map(|key| context.roots.stores.get(key).to_path_buf());
    let stores = FilesystemStores::open_read_only(&paths)
        .map_err(|error| Box::new(contained_error(error)))?
        .with_private_roots(std::slice::from_ref(&context.roots.stores.engine_outputs));
    let publications = crate::compose::startup::publications::repair(
        context.ledger,
        &stores,
        context.settings,
        context.roots,
        context.sink,
        context.warnings,
    )?;
    let result = vpt_application::reconcile_intents(
        run,
        context.ledger,
        &stores,
        context.helper,
        &SystemClock,
        &context.roots.stores,
        context.sink,
    )
    .map(RetentionReport::from)
    .map_err(|failure| {
        Box::new(RetentionFailure {
            error: failure.error,
            report: RetentionReport::from(failure.progress),
            committed: failure.committed,
        })
    });
    let result = accept(&mut report.retention, result);
    report.retention_failed |= result.is_err();
    result.map_err(|mut error| {
        crate::compose::publication::merge_completed(&mut error, publications.iter().cloned());
        error
    })?;
    Ok(publications)
}
