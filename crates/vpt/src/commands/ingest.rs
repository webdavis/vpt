pub(crate) mod errors;
mod observe;
mod output;
pub(crate) mod runtime;
pub(crate) mod startup_failure;
use crate::compose::startup;

use crate::cli::output::Outcome;
use crate::compose::{Environment, config_refusal, settings_from};
use std::path::Path;
use vpt_adapters::{ClonefileArchive, HelperClient, LoggedEvents, SqliteLedger, VoiceMemosStore};
use vpt_application::Mode;
use vpt_application::ports::{Clock, EventSink, RunHistory, SinkOutcome};
use vpt_domain::run::RunKind;
use vpt_protocol::error::ErrorDocument;

pub(crate) fn run(environment: &Environment, config: Option<&Path>, mode: &Mode) -> Outcome {
    execute(environment, config, mode).unwrap_or_else(|error| Outcome::Failure(*error))
}

fn execute(
    environment: &Environment,
    config: Option<&Path>,
    mode: &Mode,
) -> Result<Outcome, Box<ErrorDocument>> {
    let settings = settings_from(environment, config)?;
    let config_path = config.map(Path::to_path_buf).map_or_else(
        || {
            environment
                .config_path()
                .map_err(super::super::compose::config_error)
        },
        Ok,
    )?;
    let roots =
        vpt_adapters::config::resolve(&settings, config_path.parent().unwrap_or(Path::new(".")))
            .map_err(|error| Box::new(crate::compose::errors::root_error(&error)))?;
    let helper = HelperClient::new(settings.helper_path.clone());
    let recorder = VoiceMemosStore::open(&roots.recordings_dir);
    if mode.dry_run {
        return observe::run(recorder, &roots, &helper, &settings.source, mode);
    }
    let state = crate::compose::mutation::state(&roots)?;
    let _lock = crate::compose::mutation::lock(&state)?;
    let ledger = SqliteLedger::open(&state)
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
    let mut warnings = match sink.recover(&view) {
        SinkOutcome::Continue { warnings } => warnings,
        SinkOutcome::Stop(stop) => {
            return Err(Box::new(
                errors::document(vpt_application::IngestFailure::SinkStopped(stop.reason))
                    .diagnostics(
                        stop.warnings
                            .into_iter()
                            .map(crate::compose::events::warning_text)
                            .map(str::to_owned)
                            .collect(),
                    ),
            ));
        }
    };
    let retention = startup::repair(
        &ledger,
        &mut sink,
        &helper,
        &settings,
        &roots,
        &mut warnings,
    )
    .map_err(|mut error| {
        error.diagnostics.extend(
            warnings
                .iter()
                .copied()
                .map(crate::compose::events::warning_text)
                .map(str::to_owned),
        );
        error
    })?;
    warnings.extend(retention.retention.warnings.iter().copied());
    let work = (|| -> Result<Outcome, Box<ErrorDocument>> {
        let recorder = recorder
            .map_err(|error| startup_failure::source(&ledger, &mut sink, error, &mut warnings))?;
        roots.create_leaves().map_err(|error| {
            config_refusal::start_run(&ledger, &mut sink, RunKind::Ingest, error, &mut warnings)
        })?;
        let archive = ClonefileArchive::open(&roots.stores.audio)
            .map_err(|error| Box::new(crate::compose::errors::contained_error(error)))?;
        let recorder = recorder.with_titles_root(&state, settings.source.read_titles);
        let mut report = runtime::execute(
            &recorder,
            &archive,
            &ledger,
            &helper,
            &mut sink,
            &settings.source,
            mode,
        );
        match &mut report {
            Ok(report) => report.warnings.extend(warnings.iter().copied()),
            Err(error) => error.warnings.extend(warnings.iter().copied()),
        }
        Ok(output::outcome(report, false))
    })();
    let outcome = work.unwrap_or_else(|mut error| {
        error.diagnostics.extend(
            warnings
                .into_iter()
                .map(crate::compose::events::warning_text)
                .map(str::to_owned),
        );
        Outcome::Failure(*error)
    });
    Ok(startup::attach(outcome, retention))
}
