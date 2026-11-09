mod context;
mod mutation;
mod output;
mod prepare;
mod render;
mod sources;
mod upcoming;
use crate::{
    cli::{args::brief::Options, output::Outcome},
    compose::{Environment, RuntimeLedger, settings_from},
};
use std::path::Path;
use vpt_adapters::{FilesystemStores, Roots};
use vpt_application::Settings;
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(crate) fn run(environment: &Environment, config: Option<&Path>, options: &Options) -> Outcome {
    execute(environment, config, options).unwrap_or_else(|error| Outcome::Failure(*error))
}
fn execute(
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
        .map_err(|error| Box::new(crate::compose::errors::root_error(&error)))?;
    if matches!(options.target, crate::cli::args::brief::Target::Upcoming) {
        upcoming::validate(&settings)?;
        return mutation::run(&settings, &roots, options);
    }
    context::validate(&settings)?;
    if let crate::cli::args::brief::Target::Existing(id) = &options.target {
        use vpt_application::ports::OccasionLedger;
        if RuntimeLedger::read_only(&roots.state_dir)?
            .occasion(id)
            .map_err(ledger_error)?
            .is_none()
        {
            return Err(invalid());
        }
    }
    if options.dry_run || options.explain {
        let ledger = RuntimeLedger::read_only(&roots.state_dir)?;
        let prepared = match &ledger {
            RuntimeLedger::Sqlite(ledger) => prepare::run(ledger, &settings, &roots, options)?,
            RuntimeLedger::Empty(ledger) => prepare::run(ledger, &settings, &roots, options)?,
        };
        return output::success(&prepared.artifact, false, options.explain);
    }
    mutation::run(&settings, &roots, options)
}
fn stores(roots: &Roots) -> Result<FilesystemStores, Box<ErrorDocument>> {
    let paths = vpt_domain::layout::StoreKey::all().map(|key| roots.stores.get(key).to_path_buf());
    FilesystemStores::open_read_only(&paths)
        .map_err(|error| Box::new(crate::compose::errors::contained_error(error)))
}
fn invalid() -> Box<ErrorDocument> {
    Box::new(ErrorDocument::new(
        ErrorKind::Usage,
        "invalid brief request",
    ))
}
fn ledger_error(error: vpt_application::ports::LedgerError) -> Box<ErrorDocument> {
    Box::new(crate::compose::errors::ledger_error(error))
}
