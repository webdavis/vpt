pub(crate) mod errors;
mod mutation;
pub(crate) mod observe;
pub(crate) mod work;
use crate::cli::output::Outcome;
use crate::compose::{Environment, settings_from};
use std::path::Path;
use vpt_domain::identity::RecordingId;
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(crate) fn run(
    environment: &Environment,
    config: Option<&Path>,
    id: &str,
    language: Option<&str>,
    dry_run: bool,
) -> Outcome {
    execute(environment, config, id, language, dry_run)
        .unwrap_or_else(|error| Outcome::Failure(*error))
}

fn execute(
    environment: &Environment,
    config: Option<&Path>,
    id: &str,
    language: Option<&str>,
    dry_run: bool,
) -> Result<Outcome, Box<ErrorDocument>> {
    let id = RecordingId::parse(id)
        .map_err(|_| Box::new(ErrorDocument::new(ErrorKind::Usage, "invalid recording id")))?;
    let mut settings = settings_from(environment, config)?;
    if let Some(language) = language {
        settings.engines.language = language.into();
    }
    settings.engines.select(None).map_err(errors::selection)?;
    let config_path = config.map(Path::to_path_buf).map_or_else(
        || {
            environment
                .config_path()
                .map_err(crate::compose::config_error)
        },
        Ok,
    )?;
    let roots =
        vpt_adapters::config::resolve(&settings, config_path.parent().unwrap_or(Path::new(".")))
            .map_err(|error| Box::new(crate::compose::errors::root_error(&error)))?;
    let ledger = crate::compose::RuntimeLedger::read_only(&roots.state_dir)?;
    let record = observe::record(&ledger, &id)?;
    let duration = observe::audio(&record, &settings, &roots)?;
    if dry_run {
        return observe::plan(&record, &settings, duration);
    }
    mutation::run(&id, &settings, &roots)
}
