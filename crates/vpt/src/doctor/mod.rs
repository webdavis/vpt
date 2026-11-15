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
use std::path::Path;

pub(crate) fn run(environment: &Environment, config: Option<&Path>) -> Outcome {
    report::outcome(&checks::run(environment, config))
}
