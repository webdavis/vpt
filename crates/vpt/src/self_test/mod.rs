mod engines;
mod plan;
mod speakers;

use crate::cli::output::Outcome;
use crate::compose::{Environment, settings_from};
use plan::Part;
use std::path::Path;

pub(crate) const NAMES: [&str; 2] = ["engines", "speakers"];

pub(crate) fn run(environment: &Environment, config: Option<&Path>, wanted: &[String]) -> Outcome {
    let settings = match settings_from(environment, config) {
        Ok(settings) => settings,
        Err(error) => return Outcome::Failure(*error),
    };
    let parts = [
        Part {
            name: "engines",
            blurb: "each selected engine transcribes a few seconds of made-up speech; nothing is kept",
            run: engines::test(&settings),
        },
        Part {
            name: "speakers",
            blurb: "whisply labels the speakers in a made-up clip of two voices; nothing is kept",
            run: speakers::test(&settings),
        },
    ];
    let parts = parts
        .into_iter()
        .filter(|part| wanted.is_empty() || wanted.iter().any(|name| name == part.name))
        .collect();
    match plan::checks(parts, !wanted.is_empty()) {
        Ok(checks) => crate::report::outcome("test", &checks, None),
        Err(error) => Outcome::Failure(*error),
    }
}

#[cfg(test)]
mod tests;
