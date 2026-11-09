pub(crate) mod errors;
pub(super) mod output;
pub(super) mod runtime;

use crate::cli::output::Outcome;
use crate::compose::{Environment, RuntimeLedger, settings_from};
use std::path::Path;
use vpt_adapters::config::resolve;
use vpt_adapters::{FilesystemStores, HelperClient, LoggedEvents};
use vpt_application::ReconcileProgress;
use vpt_application::ports::{Clock, EventSink, RunHistory};
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(crate) fn run(environment: &Environment, config: Option<&Path>, dry_run: bool) -> Outcome {
    match execute(environment, config, dry_run) {
        Ok(outcome) => outcome,
        Err(error) => Outcome::Failure(*error),
    }
}

fn execute(
    environment: &Environment,
    config: Option<&Path>,
    dry_run: bool,
) -> Result<Outcome, Box<ErrorDocument>> {
    let settings = settings_from(environment, config)?;
    if !settings.retention.enabled {
        return Err(Box::new(ErrorDocument::new(
            ErrorKind::Config,
            "retention is disabled",
        )));
    }
    let config_path = config.map(Path::to_path_buf).map_or_else(
        || {
            environment.config_path().map_err(|_| {
                Box::new(ErrorDocument::new(
                    ErrorKind::Config,
                    "invalid configuration path",
                ))
            })
        },
        Ok,
    )?;
    let roots = resolve(&settings, config_path.parent().unwrap_or(Path::new(".")))
        .map_err(|error| Box::new(crate::compose::errors::root_error(&error)))?;
    let paths = vpt_domain::layout::StoreKey::all().map(|key| roots.stores.get(key).to_path_buf());
    let stores = FilesystemStores::open_read_only(&paths)
        .map_err(|error| Box::new(crate::compose::errors::contained_error(error)))?
        .with_private_roots(&[roots
            .stores
            .get(vpt_domain::layout::StoreKey::EngineOutputs)
            .to_path_buf()]);
    let helper = HelperClient::new(settings.helper_path.clone());
    if dry_run {
        let ledger = RuntimeLedger::read_only(&roots.state_dir)?;
        let mut sink = crate::compose::events::Observation;
        let result = match &ledger {
            RuntimeLedger::Sqlite(ledger) => runtime::execute(
                ledger,
                &stores,
                &helper,
                &mut sink,
                &settings.retention,
                &roots.stores,
                true,
                ReconcileProgress::default(),
            ),
            RuntimeLedger::Empty(ledger) => runtime::execute(
                ledger,
                &stores,
                &helper,
                &mut sink,
                &settings.retention,
                &roots.stores,
                true,
                ReconcileProgress::default(),
            ),
        };
        return Ok(output::outcome(result, true));
    }
    let state = crate::compose::mutation::state(&roots)?;
    let _lock = crate::compose::mutation::lock(&state)?;
    let ledger = vpt_adapters::SqliteLedger::open(&state)
        .map_err(|error| Box::new(crate::compose::errors::open_error(error)))?;
    let mut sink = LoggedEvents::new(
        &state,
        &ledger,
        &settings.report_command,
        &settings.alert_command,
        || vpt_adapters::SystemClock.now(),
    );
    let view = ledger
        .recovery_view()
        .map_err(|error| Box::new(crate::compose::errors::ledger_error(error)))?;
    let mut progress = match sink.recover(&view) {
        vpt_application::ports::SinkOutcome::Continue { warnings } => ReconcileProgress {
            warnings,
            ..Default::default()
        },
        vpt_application::ports::SinkOutcome::Stop(stop) => {
            let report = vpt_application::RetentionReport {
                warnings: stop.warnings,
                ..Default::default()
            };
            return Ok(output::outcome(
                Err(Box::new(vpt_application::RetentionFailure {
                    error: vpt_application::RetentionError::SinkStopped(stop.reason),
                    report,
                    committed: None,
                })),
                false,
            ));
        }
    };
    let completed = crate::compose::startup::publications::repair(
        &ledger,
        &stores,
        &settings,
        &roots,
        &mut sink,
        &mut progress.warnings,
    )
    .map_err(|error| crate::compose::events::attach_error(error, &progress.warnings))?;
    let mut outcome = output::outcome(
        runtime::execute(
            &ledger,
            &stores,
            &helper,
            &mut sink,
            &settings.retention,
            &roots.stores,
            false,
            progress,
        ),
        false,
    );
    outcome.add_completed(completed);
    Ok(outcome)
}
