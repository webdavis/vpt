use super::*;

pub(crate) fn repair<E: EventSink>(
    ledger: &SqliteLedger,
    stores: &FilesystemStores,
    settings: &Settings,
    roots: &Roots,
    sink: &mut E,
    warnings: &mut std::collections::BTreeSet<vpt_application::ports::SinkWarning>,
) -> Result<Vec<String>, Box<ErrorDocument>> {
    let mut completed =
        crate::compose::publication::repair(ledger, stores, settings, sink, warnings)?;
    let releases =
        crate::compose::releases::repair(ledger, roots, sink, warnings).map_err(|mut error| {
            crate::compose::publication::merge_completed(&mut error, completed.iter().cloned());
            error
        })?;
    for id in releases {
        if !completed.contains(&id) {
            completed.push(id);
        }
    }
    Ok(completed)
}
