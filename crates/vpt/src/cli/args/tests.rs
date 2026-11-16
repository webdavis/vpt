use super::*;
use std::ffi::OsString;
use std::path::PathBuf;
mod analyze;
mod confirm;
mod preview;
mod privacy;
mod relation;
mod verify;

fn parsed(line: &str) -> Result<Invocation, UsageError> {
    parse(std::iter::once(OsString::from("vpt")).chain(line.split(' ').map(OsString::from)))
}

fn refused(line: &str) -> String {
    parsed(line).expect_err(line).message
}

const UNKNOWN_FLAG: &str =
    "unknown flag, or a value that starts with '-'; put '--' before such a value";

#[test]
fn json_and_config_are_global_flags_in_any_position() {
    let invocation = parsed("ingest --json --config /tmp/c.toml --preview").expect("parsed");
    assert_eq!(invocation.format, Some(Format::Json));
    assert_eq!(invocation.config, Some(PathBuf::from("/tmp/c.toml")));
    assert_eq!(
        invocation.verb,
        Verb::Ingest {
            dry_run: true,
            once: None
        }
    );
}

#[test]
fn an_unknown_verb_is_a_usage_error_naming_it() {
    let error = parsed("transcode").unwrap_err();
    assert_eq!(error.message, "unknown command");
    assert_eq!(error.tips, ["tip: did you mean 'transcribe'?"]);
}

#[test]
fn review_accepts_global_options_and_requires_exactly_one_identity() {
    for line in [
        "review list memo --json",
        "--json review list memo",
        "review list --config custom.toml memo",
    ] {
        assert_eq!(
            parsed(line).unwrap().verb,
            Verb::Review { id: "memo".into() }
        );
    }
    for line in [
        "review list",
        "review list --json",
        "review list memo extra",
    ] {
        assert!(parsed(line).is_err(), "{line}");
    }
}

#[test]
fn a_trailing_unknown_argument_is_refused() {
    assert_eq!(refused("doctor --loud"), UNKNOWN_FLAG);
}

#[test]
fn a_value_flag_without_a_value_is_refused() {
    assert_eq!(
        refused("ingest --once"),
        "a value is required for '--once <PATH>'"
    );
}

#[test]
fn every_stage_one_verb_decodes_its_arguments() {
    for (line, verb) in [
        ("--version", Verb::Version),
        (
            "setup",
            Verb::Setup {
                overwrite: false,
                engine: None,
            },
        ),
        (
            "setup --overwrite --engine whisply",
            Verb::Setup {
                overwrite: true,
                engine: Some("whisply".into()),
            },
        ),
        (
            "doctor",
            Verb::Doctor {
                checks: vec![],
                fix: false,
            },
        ),
        (
            "doctor stores --fix",
            Verb::Doctor {
                checks: vec!["stores".into()],
                fix: true,
            },
        ),
        (
            "ingest",
            Verb::Ingest {
                dry_run: false,
                once: None,
            },
        ),
        (
            "ingest --once memo.m4a",
            Verb::Ingest {
                dry_run: false,
                once: Some("memo.m4a".into()),
            },
        ),
        (
            "show memo",
            Verb::Show {
                id: "memo".into(),
                stage: None,
                redact: false,
            },
        ),
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
            "retention run --preview",
            Verb::RetentionRun { dry_run: true },
        ),
        ("symlink deploy", Verb::SymlinkDeploy),
        ("symlink verify", Verb::SymlinkVerify),
    ] {
        assert_eq!(parsed(line).expect(line).verb, verb);
    }
    let invocation = parsed("--config c.toml --json doctor").expect("global flags first");
    assert_eq!(invocation.format, Some(Format::Json));
    assert_eq!(invocation.config, Some("c.toml".into()));
    assert_eq!(
        invocation.verb,
        Verb::Doctor {
            checks: vec![],
            fix: false,
        }
    );
}

#[test]
fn missing_positionals_subcommands_and_flag_values_are_refused() {
    assert_eq!(
        parse([OsString::from("vpt"), OsString::from("--json")]),
        Err(UsageError::new("a command is required"))
    );
    for (line, message) in [
        ("show", "<ID> is required"),
        ("show --force", UNKNOWN_FLAG),
        ("setup --force", UNKNOWN_FLAG),
        ("retention erase", "unknown command"),
        ("symlink erase", "unknown command"),
        (
            "--config --json doctor",
            "a value is required for '--config <FILE>'",
        ),
        ("list --stage --force", UNKNOWN_FLAG),
        ("--version doctor", "--version takes no command"),
    ] {
        assert_eq!(refused(line), message, "{line}");
    }
}

#[test]
fn global_flags_cannot_fill_missing_value_slots() {
    for (line, flag) in [
        ("ingest --once --json memo.m4a", "--once <PATH>"),
        ("list --stage --json extract", "--stage <STAGE>"),
        ("ingest --once --config c.toml memo.m4a", "--once <PATH>"),
    ] {
        assert_eq!(
            refused(line),
            format!("a value is required for '{flag}'"),
            "{line}"
        );
    }
}

#[test]
fn non_utf8_paths_are_refused_without_substitution() {
    use std::os::unix::ffi::OsStringExt;
    for flag in ["--config", "--once"] {
        let args = [
            "vpt".into(),
            "ingest".into(),
            flag.into(),
            OsString::from_vec(vec![b'a', 0xff]),
        ];
        assert_eq!(
            parse(args),
            Err(UsageError::new("arguments must be valid UTF-8"))
        );
    }
}

#[test]
fn transcribe_accepts_language_and_dry_run_flags_around_the_identity() {
    for line in [
        "transcribe --language fr-CA id --preview",
        "transcribe id --preview --language fr-CA",
    ] {
        assert_eq!(
            parsed(line).unwrap().verb,
            Verb::Transcribe {
                id: "id".into(),
                language: Some("fr-CA".into()),
                dry_run: true
            }
        );
    }
    for line in [
        "transcribe id --language",
        "transcribe --language --json id",
        "transcribe",
    ] {
        assert!(parsed(line).is_err());
    }
}

#[test]
fn path_requires_one_identity_and_an_explicit_stage_with_global_options() {
    for line in [
        "path memo --stage transcript --json",
        "--json path --stage transcript memo",
    ] {
        assert_eq!(
            parsed(line).unwrap().verb,
            Verb::Path {
                id: "memo".into(),
                stage: "transcript".into()
            }
        );
    }
    for line in [
        "path memo",
        "path --stage transcript",
        "path memo --stage",
        "path memo --stage transcript extra",
    ] {
        assert!(parsed(line).is_err(), "{line}");
    }
}

mod review;

#[test]
fn run_accepts_preview_and_global_flags_but_no_recording_or_resend() {
    assert_eq!(
        parsed("run --preview --json").unwrap().verb,
        Verb::Run { dry_run: true }
    );
    for line in [
        "run memo",
        "run --once memo",
        "run --language fr",
        "run --preview --preview",
        "run --resend-refused",
    ] {
        assert!(parsed(line).is_err(), "{line}");
    }
}

#[test]
fn actions_list_and_resend_take_an_optional_recording() {
    for (line, verb) in [
        ("actions list", Verb::ActionsList { id: None }),
        (
            "actions list memo --json",
            Verb::ActionsList {
                id: Some("memo".into()),
            },
        ),
        (
            "actions resend",
            Verb::ActionsResend {
                id: None,
                preview: false,
            },
        ),
        (
            "actions resend memo -n",
            Verb::ActionsResend {
                id: Some("memo".into()),
                preview: true,
            },
        ),
    ] {
        assert_eq!(parsed(line).expect(line).verb, verb);
    }
    assert!(
        parsed("actions resend -n")
            .unwrap()
            .verb
            .reads_ledger_only()
    );
    assert!(!parsed("actions resend").unwrap().verb.reads_ledger_only());
    let Verb::Print(help) = parsed("actions").unwrap().verb else {
        panic!("help");
    };
    assert!(help.contains("resend"), "{help}");
    for line in [
        "actions list one two",
        "actions send",
        "actions resend --all",
    ] {
        assert!(parsed(line).is_err(), "{line}");
    }
}
