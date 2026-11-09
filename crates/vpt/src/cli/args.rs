mod confirm;
mod options;
mod review;
mod text;
mod verbs;
use options::{take_flag, take_positional, take_value};
use std::ffi::OsString;
use std::path::PathBuf;
use verbs::parse_verb;

pub const USAGE: &str = "usage: vpt [--json] [--config <file>] <verb>\n\
verbs: setup [--force] | doctor | ingest [--dry-run] [--once <path>] | show <id> |\n\
       transcribe <id> [--language <tag>] [--dry-run] | review <id> [--resolve <flag> --confirm|--correct <text>|--dismiss] | confirm --term <text> | confirm <id> --tag <tag>|--reject-tag <tag>|--relation <kind>:<target> |\n\
       synthesize <id> [--dry-run] | note write <id> | list [--stage <stage>] | storage | retention run [--dry-run] |\n\
       symlink deploy | symlink verify | --version\n";

#[derive(Debug, PartialEq, Eq)]
pub enum Verb {
    Synthesize {
        id: String,
        dry_run: bool,
    },
    ReviewResolve {
        id: String,
        flag: String,
        action: vpt_domain::review::ReviewAction,
    },
    ConfirmTerm {
        term: text::PrivateText,
    },
    ConfirmRelation {
        id: String,
        relation: text::PrivateText,
    },
    ConfirmTag {
        id: String,
        tag: text::PrivateText,
        action: vpt_domain::tags::Action,
    },
    Version,
    Setup {
        force: bool,
    },
    Doctor,
    NoteWrite {
        id: String,
    },
    Transcribe {
        id: String,
        language: Option<String>,
        dry_run: bool,
    },
    Ingest {
        dry_run: bool,
        once: Option<PathBuf>,
    },
    Show {
        id: String,
    },
    Review {
        id: String,
    },
    List {
        stage: Option<String>,
    },
    Storage,
    RetentionRun {
        dry_run: bool,
    },
    SymlinkDeploy,
    SymlinkVerify,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Invocation {
    pub verb: Verb,
    pub json: bool,
    pub config: Option<PathBuf>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct UsageError(pub String);

pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Invocation, UsageError> {
    let mut words: Vec<String> = args
        .into_iter()
        .skip(1)
        .map(|word| {
            word.into_string()
                .map_err(|_| UsageError("arguments must be valid UTF-8".into()))
        })
        .collect::<Result<_, _>>()?;
    let text = text::take(&mut words)?;
    for (index, word) in words.iter().enumerate() {
        if matches!(
            word.as_str(),
                | "--config"
                | "--once"
                | "--stage"
                | "--language"
                | "--resolve"
        ) && words
            .get(index + 1)
            .is_none_or(|value| value.starts_with("--"))
        {
            return Err(UsageError(format!("{word} needs a value")));
        }
    }
    let config = take_value(&mut words, "--config")?.map(PathBuf::from);
    let json = take_flag(&mut words, "--json");
    let verb = parse_verb(&mut words, text)?;
    if let Some(extra) = words.first() {
        return Err(UsageError(format!("unexpected argument {extra}")));
    }
    Ok(Invocation { verb, json, config })
}

#[cfg(test)]
mod tests;
