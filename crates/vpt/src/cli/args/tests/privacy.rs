use super::super::{root::Cli, usage::from_clap};
use super::*;
use clap::CommandFactory;
use clap::error::{ContextKind, ContextValue, ErrorKind};
use std::collections::HashSet;

const MARK: &str = "§";
const TYPED: [&str; 3] = ["§ private", "-§private", "--§private"];

fn invocations(command: &clap::Command, path: &[String], out: &mut Vec<Vec<String>>) {
    let with = |tail: Vec<String>| [path.to_vec(), tail].concat();
    for typed in TYPED.map(String::from) {
        out.push(with(vec![typed.clone()]));
        out.push(with(vec![typed.clone(), "x".into()]));
        out.push(with(vec!["x".into(), typed.clone()]));
        out.push(with(vec![typed.clone(), typed.clone(), typed.clone()]));
        for long in command.get_arguments().filter_map(clap::Arg::get_long) {
            let flag = format!("--{long}");
            out.push(with(vec![format!("{flag}={typed}")]));
            out.push(with(vec![flag.clone(), typed.clone()]));
            out.push(with(vec![flag.clone(), typed.clone(), flag, typed.clone()]));
        }
    }
    for sub in command.get_subcommands() {
        let path = [path.to_vec(), vec![sub.get_name().to_string()]].concat();
        invocations(sub, &path, out);
    }
}

#[test]
fn no_usage_error_repeats_typed_text_for_any_flag_positional_or_error_kind() {
    let mut all = vec![];
    invocations(&Cli::command(), &[], &mut all);
    let mut kinds = HashSet::new();
    for words in all {
        let words = [vec!["vpt".to_string()], words].concat();
        if let Err(error) = Cli::command().try_get_matches_from(&words) {
            kinds.insert(error.kind());
            if let Err(usage) = from_clap(&error) {
                assert!(!format!("{usage:?}").contains(MARK), "{words:?} {usage:?}");
            }
        }
        if let Err(usage) = parse(words.iter().map(OsString::from)) {
            assert!(!format!("{usage:?}").contains(MARK), "{words:?} {usage:?}");
        }
    }
    for kind in [
        ErrorKind::UnknownArgument,
        ErrorKind::InvalidSubcommand,
        ErrorKind::TooManyValues,
        ErrorKind::ValueValidation,
        ErrorKind::ArgumentConflict,
        ErrorKind::MissingRequiredArgument,
    ] {
        assert!(kinds.contains(&kind), "{kind:?} never produced");
    }
}

#[test]
fn every_error_kind_is_described_without_its_raw_text_or_typed_context() {
    for kind in [
        ErrorKind::InvalidValue,
        ErrorKind::UnknownArgument,
        ErrorKind::InvalidSubcommand,
        ErrorKind::NoEquals,
        ErrorKind::ValueValidation,
        ErrorKind::TooManyValues,
        ErrorKind::TooFewValues,
        ErrorKind::WrongNumberOfValues,
        ErrorKind::ArgumentConflict,
        ErrorKind::MissingRequiredArgument,
        ErrorKind::MissingSubcommand,
        ErrorKind::InvalidUtf8,
        ErrorKind::DisplayVersion,
        ErrorKind::Io,
        ErrorKind::Format,
    ] {
        let mut error = clap::Error::raw(kind, MARK);
        error.insert(ContextKind::InvalidValue, ContextValue::String(MARK.into()));
        error.insert(
            ContextKind::Suggested,
            ContextValue::StyledStrs(vec![MARK.into()]),
        );
        if matches!(
            kind,
            ErrorKind::UnknownArgument | ErrorKind::InvalidSubcommand
        ) {
            error.insert(ContextKind::InvalidArg, ContextValue::String(MARK.into()));
            error.insert(
                ContextKind::InvalidSubcommand,
                ContextValue::String(MARK.into()),
            );
        }
        let usage = from_clap(&error).expect_err("a usage error");
        assert!(!format!("{usage:?}").contains(MARK), "{kind:?} {usage:?}");
    }
}
