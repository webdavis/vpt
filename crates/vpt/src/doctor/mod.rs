mod checks;
mod engines;
mod helper;
mod ledger;
mod leftovers;
mod report;
mod source;
mod state_dir;
mod stores;

use crate::cli::output::Outcome;
use crate::compose::Environment;
pub(crate) use checks::NAMES;
use std::path::Path;

pub(crate) fn run(environment: &Environment, config: Option<&Path>, wanted: &[String]) -> Outcome {
    report::outcome(&checks::run(environment, config, wanted))
}
