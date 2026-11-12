pub(crate) mod brief;
mod confirm;
pub(crate) mod handoff;
pub(crate) mod import;
mod options;
pub(crate) mod redact;
mod review;
mod text;
mod verbs;
use super::format::Format;
use options::{require_values, take_flag, take_format, take_positional, take_value};
use std::ffi::OsString;
use std::path::PathBuf;
use verbs::parse_verb;

pub const USAGE: &str = "usage: vpt [--json|--toon|--yaml] [--config <file>] <verb>\n\
verbs: run [--dry-run] | setup [--force] | doctor | ingest [--dry-run] [--once <path>] |\n\
       import <file> [--title <text>] [--captured-at <RFC3339>] | show <id> |\n\
       transcribe <id> [--language <tag>] [--dry-run] | review <id> [--resolve <flag> --confirm|--correct <text>|--dismiss] | confirm --term <text> | confirm <id> --tag <tag>|--reject-tag <tag>|--relation <kind>:<target> |\n\
       synthesize <id> [--dry-run] | path <id> --stage transcript|analysis | note write <id> | verify-note <path> --recording <id> | occasions | path --occasion <id> --stage brief | list [--stage <stage>] | storage | retention run [--dry-run] |\n\
       brief <occasion-id> [--dry-run] [--explain] | brief --title <title> --at <RFC3339> [--duration-minutes <n>] [--participant <name>]... [--tag <tag>]... | brief --upcoming |\n\
       handoff <id> --stage released|transcript|analysis|brief | redact <id> [--stage transcript|analysis] [--to <dir>] [--title <title>] | symlink deploy | symlink verify | --version\n";

#[derive(Debug, PartialEq, Eq)]
pub enum Verb {
    Brief(brief::Options),
    Redact(redact::Options),
    Import(import::Options),
    Handoff(handoff::Options),
    Occasions,
    OccasionPath {
        id: String,
        stage: String,
    },
    Synthesize {
        id: String,
        dry_run: bool,
    },
    VerifyNote {
        path: PathBuf,
        recording: String,
    },
    Run {
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
    Path {
        id: String,
        stage: String,
    },
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
    pub format: Option<Format>,
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
    require_values(&words)?;
    let invocation = Invocation {
        config: take_value(&mut words, "--config")?.map(PathBuf::from),
        format: take_format(&mut words)?,
        verb: parse_verb(&mut words, text)?,
    };
    if let Some(extra) = words.first() {
        return Err(UsageError(format!("unexpected argument {extra}")));
    }
    Ok(invocation)
}

#[cfg(test)]
mod tests;
