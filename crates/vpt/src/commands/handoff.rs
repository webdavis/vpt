mod brief;
pub(crate) mod errors;
mod file;
mod output;
mod private;
mod released;
use crate::{
    cli::{
        args::handoff::{Options, Target},
        output::Outcome,
    },
    compose::{Environment, settings_from},
};
use std::path::Path;
use vpt_adapters::SqliteLedger;
use vpt_domain::handoff::Kind;
use vpt_protocol::error::ErrorDocument;
pub(crate) fn run(environment: &Environment, config: Option<&Path>, options: &Options) -> Outcome {
    execute(environment, config, options).unwrap_or_else(|e| Outcome::Failure(*e))
}
fn execute(
    environment: &Environment,
    config: Option<&Path>,
    options: &Options,
) -> Result<Outcome, Box<ErrorDocument>> {
    let settings = settings_from(environment, config)?;
    if options.kind != Kind::Released && !settings.handoff_allow_private {
        return Err(errors::private());
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
        .map_err(|e| Box::new(crate::compose::errors::root_error(&e)))?;
    let ledger = SqliteLedger::open_read_only(&roots.state_dir)
        .map_err(|e| Box::new(crate::compose::errors::open_error(e)))?
        .ok_or_else(errors::missing)?;
    match &options.target {
        Target::Recording(id) if options.kind == Kind::Released => {
            released::run(&ledger, &roots, id)
        }
        Target::Recording(id) => private::run(&ledger, &roots, id, options.kind),
        Target::Occasion(id) => brief::run(&ledger, &roots, id),
    }
}
