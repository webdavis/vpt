mod check;
mod errors;
mod inspect;
mod mutation;
mod output;
pub(crate) mod persist;
use crate::{
    cli::output::Outcome,
    compose::{
        Environment,
        errors::{ledger_error, open_error, root_error},
        settings_from,
    },
};
use std::path::Path;
use vpt_adapters::SqliteLedger;
use vpt_application::ports::NoteInputsLedger;
use vpt_domain::identity::RecordingId;
use vpt_protocol::error::ErrorDocument;

pub(crate) fn run(
    environment: &Environment,
    config: Option<&Path>,
    path: &Path,
    id: &str,
) -> Outcome {
    prepare(environment, config, path, id).unwrap_or_else(|error| Outcome::Failure(*error))
}

fn prepare(
    environment: &Environment,
    config: Option<&Path>,
    path: &Path,
    id: &str,
) -> Result<Outcome, Box<ErrorDocument>> {
    let id = RecordingId::parse(id).map_err(|_| errors::usage("invalid recording id"))?;
    let path = std::path::absolute(path).map_err(|_| errors::read())?;
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
        .map_err(|error| Box::new(root_error(&error)))?;
    let ledger = SqliteLedger::open_read_only(&roots.state_dir)
        .map_err(|error| Box::new(open_error(error)))?
        .ok_or_else(|| errors::usage("recording not found"))?;
    let input = ledger
        .note_inputs(&id)
        .map_err(|error| Box::new(ledger_error(error)))?
        .ok_or_else(|| errors::usage("recording not found"))?;
    let source = inspect::read(&path, &input, &roots)?;
    if input.review.accepted.is_none() {
        return Err(errors::usage("recording has no accepted transcript"));
    }
    if source.stage.is_none() {
        return Ok(output::success(
            &path,
            false,
            &check::run(&source, &input, settings.synthesis.grounding_window_secs)?.report,
        ));
    }
    mutation::run(&path, &id, &settings, &roots)
}
