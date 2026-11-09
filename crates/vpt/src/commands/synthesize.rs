mod errors;
mod history;
mod mutation;
mod notification;
mod observe;
mod output;
mod prepare;
mod relations;
pub(crate) mod resume;
mod source;
mod tags;
pub(crate) mod work;
use crate::{
    cli::output::Outcome,
    compose::{Environment, settings_from},
};
use std::path::Path;
use vpt_domain::identity::RecordingId;
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(crate) fn run(
    environment: &Environment,
    config: Option<&Path>,
    id: &str,
    dry: bool,
) -> Outcome {
    execute(environment, config, id, dry).unwrap_or_else(|error| Outcome::Failure(*error))
}
fn execute(
    environment: &Environment,
    config: Option<&Path>,
    id: &str,
    dry: bool,
) -> Result<Outcome, Box<ErrorDocument>> {
    let id = RecordingId::parse(id).map_err(|_| errors::usage("invalid recording id"))?;
    let settings = settings_from(environment, config)?;
    if settings.synthesis.command.is_empty() {
        return Err(Box::new(ErrorDocument::new(
            ErrorKind::Config,
            "synthesis command is not configured",
        )));
    }
    let config = config.map(Path::to_path_buf).map_or_else(
        || {
            environment
                .config_path()
                .map_err(crate::compose::config_error)
        },
        Ok,
    )?;
    let roots = vpt_adapters::config::resolve(&settings, config.parent().unwrap_or(Path::new(".")))
        .map_err(|error| Box::new(crate::compose::errors::root_error(&error)))?;
    let ledger = crate::compose::RuntimeLedger::read_only(&roots.state_dir)?;
    let input = observe::inputs(&ledger, &id)?;
    if dry {
        return observe::plan(input, &settings, &roots);
    }
    mutation::run(&id, &settings, &roots)
}
