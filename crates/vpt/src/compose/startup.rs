pub(crate) mod publications;
use crate::compose::errors::{contained_error, ledger_error};
use crate::compose::events::warning_text;
use vpt_adapters::{FilesystemStores, HelperClient, Roots, SqliteLedger, SystemClock};
use vpt_application::ports::{EventSink, RetentionJournal};
use vpt_application::{Retention, RetentionReport, Settings};
use vpt_domain::layout::StoreKey;
use vpt_protocol::error::ErrorDocument;

pub(crate) struct StartupReport {
    pub retention: RetentionReport,
    pub publications: Vec<String>,
}

pub(crate) fn repair<E: EventSink>(
    ledger: &SqliteLedger,
    sink: &mut E,
    helper: &HelperClient,
    settings: &Settings,
    roots: &Roots,
    warnings: &mut std::collections::BTreeSet<vpt_application::ports::SinkWarning>,
) -> Result<StartupReport, Box<ErrorDocument>> {
    let paths = StoreKey::all().map(|key| roots.stores.get(key).to_path_buf());
    let stores = FilesystemStores::open_read_only(&paths)
        .map_err(|error| Box::new(contained_error(error)))?
        .with_private_roots(&[roots.stores.get(StoreKey::EngineOutputs).to_path_buf()]);
    let publications = publications::repair(ledger, &stores, settings, roots, sink, warnings)?;
    let result = (|| {
        if ledger
            .pending_intents()
            .map_err(|error| Box::new(ledger_error(error)))?
            .is_empty()
        {
            return Ok(RetentionReport::default());
        }
        Retention {
            ledger,
            stores: &stores,
            trash: helper,
            clock: &SystemClock,
            sink,
            settings: &settings.retention,
            paths: &roots.stores,
        }
        .recover()
        .map_err(|failure| {
            let mut diagnostics = failure.report.diagnostics;
            diagnostics.extend(
                failure
                    .report
                    .warnings
                    .into_iter()
                    .map(warning_text)
                    .map(str::to_owned),
            );
            Box::new(
                crate::commands::retention::errors::document(failure.error)
                    .completed(
                        failure
                            .report
                            .completed
                            .iter()
                            .map(ToString::to_string)
                            .collect(),
                    )
                    .diagnostics(diagnostics),
            )
        })
    })();
    match result {
        Ok(retention) => Ok(StartupReport {
            retention,
            publications,
        }),
        Err(mut error) => {
            super::publication::merge_completed(&mut error, publications);
            Err(error)
        }
    }
}

pub(crate) fn attach(
    mut outcome: crate::cli::output::Outcome,
    report: StartupReport,
) -> crate::cli::output::Outcome {
    outcome.add_completed(report.publications);
    let report = report.retention;
    let completed: Vec<_> = report.completed.iter().map(ToString::to_string).collect();
    let summary = format!(
        "Recovered retention: {} moved, {} completed\n",
        report.moved.len(),
        completed.len()
    );
    outcome.add_completed(completed.iter().cloned());
    if let crate::cli::output::Outcome::Success {
        document, human, ..
    } = &mut outcome
    {
        document["retention"] = serde_json::json!({"moved": report.moved.iter().map(|row| serde_json::json!({"id": row.owner.as_str(), "store": row.store.key_name(), "path": row.path})).collect::<Vec<_>>(), "completed": completed});
        if !report.completed.is_empty() {
            human.push_str(&summary);
        }
    }
    outcome
}
