use super::{brief, errors, recording, released};
use crate::compose::{Environment, settings_from};
use std::path::Path;
use vpt_adapters::{Roots, SqliteLedger};
use vpt_application::Settings;
use vpt_domain::{copy::Kind, identity::RecordingId, occasion::OccasionId};
use vpt_protocol::error::ErrorDocument;

pub(crate) enum Target {
    Recording(RecordingId),
    Occasion(OccasionId),
}

pub(crate) fn open(
    environment: &Environment,
    config: Option<&Path>,
) -> Result<(Settings, Roots, SqliteLedger), Box<ErrorDocument>> {
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
    let ledger = SqliteLedger::open_read_only(&roots.state_dir)
        .map_err(|e| Box::new(crate::compose::errors::open_error(e)))?
        .ok_or_else(errors::missing)?;
    Ok((settings, roots, ledger))
}

pub(crate) fn read(
    ledger: &SqliteLedger,
    roots: &Roots,
    target: &Target,
    kind: Kind,
) -> Result<serde_json::Value, Box<ErrorDocument>> {
    match target {
        Target::Recording(id) if kind == Kind::Released => released::run(ledger, roots, id),
        Target::Recording(id) => recording::run(ledger, roots, id, kind),
        Target::Occasion(id) => brief::run(ledger, roots, id),
    }
}
