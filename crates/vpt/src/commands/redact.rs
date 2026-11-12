mod errors;
mod mutation;
pub(crate) mod output;
pub(crate) mod plan;
pub(crate) mod prepare;
pub(crate) mod source;
mod unchanged;
pub(crate) mod write;
use crate::{
    cli::{args::redact::Options, output::Outcome},
    compose::{Environment, settings_from},
};
use std::path::Path;
use vpt_application::ports::NoteInputsLedger;
use vpt_protocol::error::{ErrorDocument, ErrorKind};
pub(crate) fn run(environment: &Environment, config: Option<&Path>, options: &Options) -> Outcome {
    configure(environment, config, options).unwrap_or_else(|e| Outcome::Failure(*e))
}
fn configure(
    environment: &Environment,
    config: Option<&Path>,
    options: &Options,
) -> Result<Outcome, Box<ErrorDocument>> {
    let settings = settings_from(environment, config)?;
    let config = config.map(Path::to_path_buf).map_or_else(
        || {
            environment
                .config_path()
                .map_err(crate::compose::config_error)
        },
        Ok,
    )?;
    let roots = vpt_adapters::config::resolve(&settings, config.parent().unwrap_or(Path::new(".")))
        .map_err(|e| Box::new(crate::compose::errors::root_error(&e)))?;
    let ledger = vpt_adapters::SqliteLedger::open_read_only(&roots.state_dir)
        .map_err(|e| Box::new(crate::compose::errors::open_error(e)))?
        .ok_or_else(errors::missing)?;
    let input = ledger
        .note_inputs(&options.id)
        .map_err(errors::ledger)?
        .ok_or_else(errors::missing)?;
    if !input.notes.iter().any(|n| n.stage == options.stage) || input.review.accepted.is_none() {
        return Err(errors::missing());
    }
    let destination = options
        .destination
        .as_deref()
        .unwrap_or(&roots.stores.released);
    let destination = if destination.is_absolute() {
        destination.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|_| {
                Box::new(ErrorDocument::new(
                    ErrorKind::Store,
                    "cannot resolve release destination",
                ))
            })?
            .join(destination)
    };
    mutation::run(&settings, &roots, options, &destination)
}
