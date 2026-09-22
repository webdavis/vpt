use std::ffi::OsString;
use std::path::PathBuf;

pub const USAGE: &str = "usage: vpt [--json] [--config <file>] <verb>\n\
verbs: setup [--force] | doctor | ingest [--dry-run] [--once <path>] | show <id> |\n\
       list [--stage <stage>] | storage | retention run [--dry-run] |\n\
       symlink deploy | symlink verify | --version\n";

#[derive(Debug, PartialEq, Eq)]
pub enum Verb {
    Version,
    Setup {
        force: bool,
    },
    Doctor,
    Ingest {
        dry_run: bool,
        once: Option<PathBuf>,
    },
    Show {
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
    for (index, word) in words.iter().enumerate() {
        if matches!(word.as_str(), "--config" | "--once" | "--stage")
            && words
                .get(index + 1)
                .is_none_or(|value| value.starts_with("--"))
        {
            return Err(UsageError(format!("{word} needs a value")));
        }
    }
    let config = take_value(&mut words, "--config")?.map(PathBuf::from);
    let json = take_flag(&mut words, "--json");
    let verb = parse_verb(&mut words)?;
    if let Some(extra) = words.first() {
        return Err(UsageError(format!("unexpected argument {extra}")));
    }
    Ok(Invocation { verb, json, config })
}

fn parse_verb(words: &mut Vec<String>) -> Result<Verb, UsageError> {
    if words.is_empty() {
        return Err(UsageError("a verb is required".into()));
    }
    let first = words.remove(0);
    match first.as_str() {
        "--version" => Ok(Verb::Version),
        "setup" => Ok(Verb::Setup {
            force: take_flag(words, "--force"),
        }),
        "doctor" => Ok(Verb::Doctor),
        "ingest" => Ok(Verb::Ingest {
            dry_run: take_flag(words, "--dry-run"),
            once: take_value(words, "--once")?.map(PathBuf::from),
        }),
        "show" => Ok(Verb::Show {
            id: take_positional(words, "<id>")?,
        }),
        "list" => Ok(Verb::List {
            stage: take_value(words, "--stage")?,
        }),
        "storage" => Ok(Verb::Storage),
        "retention" => match take_positional(words, "run")?.as_str() {
            "run" => Ok(Verb::RetentionRun {
                dry_run: take_flag(words, "--dry-run"),
            }),
            other => Err(UsageError(format!("unknown retention subcommand {other}"))),
        },
        "symlink" => match take_positional(words, "deploy|verify")?.as_str() {
            "deploy" => Ok(Verb::SymlinkDeploy),
            "verify" => Ok(Verb::SymlinkVerify),
            other => Err(UsageError(format!("unknown symlink subcommand {other}"))),
        },
        other => Err(UsageError(format!("unknown verb {other}"))),
    }
}

fn take_flag(words: &mut Vec<String>, flag: &str) -> bool {
    match words.iter().position(|word| word == flag) {
        Some(index) => {
            words.remove(index);
            true
        }
        None => false,
    }
}

fn take_value(words: &mut Vec<String>, flag: &str) -> Result<Option<String>, UsageError> {
    let Some(index) = words.iter().position(|word| word == flag) else {
        return Ok(None);
    };
    if index + 1 >= words.len() || words[index + 1].starts_with("--") {
        return Err(UsageError(format!("{flag} needs a value")));
    }
    words.remove(index);
    Ok(Some(words.remove(index)))
}

fn take_positional(words: &mut Vec<String>, name: &str) -> Result<String, UsageError> {
    if words.is_empty() || words[0].starts_with("--") {
        return Err(UsageError(format!("{name} is required")));
    }
    Ok(words.remove(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::path::PathBuf;

    fn parsed(line: &str) -> Result<Invocation, UsageError> {
        parse(std::iter::once(OsString::from("vpt")).chain(line.split(' ').map(OsString::from)))
    }

    #[test]
    fn json_and_config_are_global_flags_in_any_position() {
        let invocation = parsed("ingest --json --config /tmp/c.toml --dry-run").expect("parsed");
        assert!(invocation.json);
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
    fn a_value_flag_without_a_value_is_refused() {
        assert_eq!(
            parsed("ingest --once"),
            Err(UsageError("--once needs a value".into()))
        );
    }

    #[test]
    fn every_stage_one_verb_decodes_its_arguments() {
        for (line, verb) in [
            ("--version", Verb::Version),
            ("setup", Verb::Setup { force: false }),
            ("setup --force", Verb::Setup { force: true }),
            ("doctor", Verb::Doctor),
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
            ("ingest --once --json memo.m4a", "--once"),
            ("list --stage --json extract", "--stage"),
            ("ingest --once --config c.toml memo.m4a", "--once"),
        ] {
            assert_eq!(
                parsed(line),
                Err(UsageError(format!("{flag} needs a value")))
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
                Err(UsageError("arguments must be valid UTF-8".into()))
            );
        }
    }
}
