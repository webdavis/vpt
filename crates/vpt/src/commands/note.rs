pub(crate) mod errors;
mod execute;
pub(crate) mod history;
pub(crate) mod output;
pub(crate) mod write;
use crate::{
    cli::output::Outcome,
    compose::{Environment, settings_from},
};
use std::path::Path;
use vpt_application::ports::NoteInputsLedger;
use vpt_domain::identity::RecordingId;
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(crate) fn run(environment: &Environment, config: Option<&Path>, id: &str) -> Outcome {
    prepare(environment, config, id).unwrap_or_else(|error| Outcome::Failure(*error))
}

fn prepare(
    environment: &Environment,
    config: Option<&Path>,
    id: &str,
) -> Result<Outcome, Box<ErrorDocument>> {
    let id = RecordingId::parse(id)
        .map_err(|_| Box::new(ErrorDocument::new(ErrorKind::Usage, "invalid recording id")))?;
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
        .map_err(|error| Box::new(crate::compose::errors::root_error(&error)))?;
    let ledger = vpt_adapters::SqliteLedger::open_read_only(&roots.state_dir)
        .map_err(|error| Box::new(crate::compose::errors::open_error(error)))?
        .ok_or_else(errors::not_found)?;
    let inputs = ledger
        .note_inputs(&id)
        .map_err(|error| Box::new(crate::compose::errors::ledger_error(error)))?
        .ok_or_else(errors::not_found)?;
    if inputs.review.accepted.is_none() {
        return Err(errors::document(
            vpt_application::note::NoteWriteError::NoTranscript,
        ));
    }
    execute::run(&id, &settings, &roots)
}
