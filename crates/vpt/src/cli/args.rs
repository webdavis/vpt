pub(crate) mod analyze;
pub(crate) mod brief;
mod completion;
mod confirm;
mod examples;
mod groups;
pub(crate) mod import;
mod invocation;
pub(crate) mod ledger;
mod pipeline;
pub(crate) mod redact;
mod review;
mod root;
pub(crate) mod search;
mod show;
mod text;
mod usage;
mod values;
mod verbs;
use super::format::Format;
pub use invocation::{Invocation, Verb};
use std::ffi::OsString;
use std::path::PathBuf;
pub use usage::UsageError;

pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Invocation, UsageError> {
    let words: Vec<String> = args
        .into_iter()
        .map(|word| {
            word.into_string()
                .map_err(|_| UsageError::new("arguments must be valid UTF-8"))
        })
        .collect::<Result<_, _>>()?;
    let cli = match <root::Cli as clap::Parser>::try_parse_from(words) {
        Ok(cli) => cli,
        Err(error) => return usage::from_clap(&error),
    };
    let global = cli.global;
    let format = Format::FLAGS
        .into_iter()
        .zip([global.json, global.toon, global.yaml])
        .find_map(|((_, format), named)| named.then_some(format));
    let verb = match (cli.command, cli.version) {
        (Some(_), true) => return Err(UsageError::new("--version takes no command")),
        (Some(command), false) => command.verb().map_err(UsageError::new)?,
        (None, true) => Verb::Version,
        (None, false) => return Err(UsageError::new("a command is required")),
    };
    Ok(Invocation {
        verb,
        format,
        config: global.config,
        progress: global.progress,
    })
}

#[cfg(test)]
mod tests;
