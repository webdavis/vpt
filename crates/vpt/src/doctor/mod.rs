mod actions;
mod checks;
mod engines;
mod helper;
mod ledger;
mod leftovers;
mod parts;
mod report;
mod source;
mod speakers;
mod state_dir;
mod stores;
mod summary;

use crate::cli::output::Outcome;
use crate::compose::Environment;
pub(crate) use checks::NAMES;
use std::path::Path;

pub(crate) fn run(environment: &Environment, config: Option<&Path>, wanted: &[String]) -> Outcome {
    report::outcome(&checks::run(environment, config, wanted))
}
