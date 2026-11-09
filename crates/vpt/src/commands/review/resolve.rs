mod changes;
mod errors;
mod execute;
mod mutation;
mod output;
use crate::{
    cli::output::Outcome,
    compose::{Environment, settings_from},
};
use std::path::Path;
use vpt_adapters::{Roots, SqliteLedger, SystemClock};
use vpt_application::ports::{Clock, NoteInputsLedger};
use vpt_domain::{digest::Sha256Digest, identity::RecordingId, review::ReviewAction};
use vpt_protocol::error::ErrorDocument;

pub(super) struct Request<'a> {
    id: RecordingId,
    flag: Sha256Digest,
    action: &'a ReviewAction,
}

pub(crate) fn run(
    environment: &Environment,
    config: Option<&Path>,
    id: &str,
    flag: &str,
    action: &ReviewAction,
) -> Outcome {
    prepare(environment, config, id, flag, action).unwrap_or_else(|error| Outcome::Failure(*error))
}

fn prepare(
    environment: &Environment,
    config: Option<&Path>,
    id: &str,
    flag: &str,
    action: &ReviewAction,
) -> Result<Outcome, Box<ErrorDocument>> {
    let request = Request {
        id: RecordingId::parse(id).map_err(|_| errors::usage("invalid recording id"))?,
        flag: Sha256Digest::from_hex(flag).ok_or_else(|| errors::usage("invalid flag id"))?,
        action,
    };
    if matches!(action, ReviewAction::Correct(text) if !vpt_domain::review::valid_correction(text))
    {
        return Err(errors::usage("correction must contain bounded, valid text"));
    }
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
    preflight(&request, &roots)?;
    mutation::run(&request, &settings, &roots)
}

fn preflight(request: &Request<'_>, roots: &Roots) -> Result<(), Box<ErrorDocument>> {
    let ledger = SqliteLedger::open_read_only(&roots.state_dir)
        .map_err(|error| Box::new(crate::compose::errors::open_error(error)))?
        .ok_or_else(|| errors::usage("recording not found"))?;
    let mut inputs = ledger
        .note_inputs(&request.id)
        .map_err(|error| Box::new(crate::compose::errors::ledger_error(error)))?
        .ok_or_else(|| errors::usage("recording not found"))?;
    vpt_application::review::resolve::prepare(
        &mut inputs.review,
        &request.id,
        request.flag,
        request.action,
        SystemClock.now(),
    )
    .map_err(errors::document)?;
    Ok(())
}
