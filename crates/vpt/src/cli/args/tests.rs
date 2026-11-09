use super::*;
use std::ffi::OsString;
use std::path::PathBuf;

fn parsed(line: &str) -> Result<Invocation, UsageError> {
    parse(std::iter::once(OsString::from("vpt")).chain(line.split(' ').map(OsString::from)))
}

#[test]
fn an_unknown_verb_is_a_usage_error_naming_it() {
    assert_eq!(
        parsed("transcode"),
        Err(UsageError("unknown verb transcode".into()))
    );
}

#[test]
fn a_trailing_unknown_argument_is_refused() {
    assert_eq!(
        parsed("doctor --loud"),
        Err(UsageError("unexpected argument --loud".into()))
    );
}

#[test]
fn every_stage_one_verb_decodes_its_arguments() {
    for (line, verb) in [
        ("--version", Verb::Version),
        ("setup", Verb::Setup { force: false }),
        ("setup --force", Verb::Setup { force: true }),
        ("doctor", Verb::Doctor),
        ("show memo", Verb::Show { id: "memo".into() }),
        ("list", Verb::List { stage: None }),
        (
            "list --stage extract",
            Verb::List {
                stage: Some("extract".into()),
            },
        ),
        ("storage", Verb::Storage),
        ("retention run", Verb::RetentionRun { dry_run: false }),
        (
            "retention run --dry-run",
            Verb::RetentionRun { dry_run: true },
        ),
        ("symlink deploy", Verb::SymlinkDeploy),
        ("symlink verify", Verb::SymlinkVerify),
    ] {
        assert_eq!(parsed(line).expect(line).verb, verb);
    }
    let invocation = parsed("--config c.toml --json doctor").expect("global flags first");
    assert!(invocation.json);
    assert_eq!(invocation.config, Some("c.toml".into()));
    assert_eq!(invocation.verb, Verb::Doctor);
}

#[test]
fn missing_positionals_subcommands_and_flag_values_are_refused() {
    assert_eq!(
        parse([OsString::from("vpt")]),
        Err(UsageError("a verb is required".into()))
    );
    for (line, message) in [
        ("show", "<id> is required"),
        ("show --force", "<id> is required"),
        ("retention", "run is required"),
        ("retention erase", "unknown retention subcommand erase"),
        ("symlink", "deploy|verify is required"),
        ("symlink erase", "unknown symlink subcommand erase"),
        ("--config --json doctor", "--config needs a value"),
        ("list --stage --force", "--stage needs a value"),
    ] {
        assert_eq!(parsed(line), Err(UsageError(message.into())));
    }
}

#[test]
fn global_flags_cannot_fill_missing_value_slots() {
    for (line, flag) in [
        ("list --stage --json extract", "--stage"),
    ] {
        assert_eq!(
            parsed(line),
            Err(UsageError(format!("{flag} needs a value")))
        );
    }
}
