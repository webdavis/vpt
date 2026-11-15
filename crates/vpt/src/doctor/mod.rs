mod actions;
mod calendar;
mod checks;
mod config;
mod engines;
mod fix;
mod helper;
mod ledger;
mod ledger_identity;
mod leftovers;
mod parts;
mod source;
mod speakers;
mod state_dir;
mod stores;
mod summary;

use crate::cli::output::Outcome;
use crate::compose::Environment;
pub(crate) use checks::NAMES;
use std::path::Path;

pub(crate) fn run(
    environment: &Environment,
    config: Option<&Path>,
    wanted: &[String],
    fix: bool,
) -> Outcome {
    let repairs = fix.then(|| fix::repair(environment, config, wanted));
    let mut checks = checks::run(environment, config, wanted);
    let fixed = repairs.map(|repairs| fix::mark(&mut checks, repairs));
    crate::report::outcome("doctor", &checks, fixed)
}
