use super::*;

pub(super) fn parse_verb(words: &mut Vec<String>, text: text::Values) -> Result<Verb, UsageError> {
    if words.is_empty() {
        return Err(UsageError("a verb is required".into()));
    }
    let first = words.remove(0);
    if text.correction.is_some() && first != "review" {
        return Err(UsageError("--correct requires review".into()));
    }
    if first == "redact" {
        return redact::parse(words, text);
    }
    if first == "import" {
        return import::parse(words, text);
    }
    if first != "brief" && (text.title.is_some() || !text.participants.is_empty()) {
        return Err(UsageError("--title and --participant require brief".into()));
    }
    if first == "brief" {
        return brief::parse(words, text);
    }
    if first == "confirm" {
        return confirm::parse(words, text);
    }
    if text.term.is_some() {
        return Err(UsageError("--term requires confirm".into()));
    }
    if text.relation.is_some() {
        return Err(UsageError("--relation requires confirm".into()));
    }
    if !text.tags.is_empty() || text.reject_tag.is_some() {
        return Err(UsageError("--tag and --reject-tag require confirm".into()));
    }
    match first.as_str() {
        "tag" => match take_positional(words, "rename")?.as_str() {
            "rename" => {
                let dry_run = take_flag(words, "--dry-run");
                Ok(Verb::TagRename {
                    old: text::PrivateText::new(take_positional(words, "<old>")?),
                    new: text::PrivateText::new(take_positional(words, "<new>")?),
                    dry_run,
                })
            }
            other => Err(UsageError(format!("unknown tag subcommand {other}"))),
        },
        "occasions" => Ok(Verb::Occasions),
        "synthesize" => {
            let dry_run = take_flag(words, "--dry-run");
            Ok(Verb::Synthesize {
                id: take_positional(words, "<id>")?,
                dry_run,
            })
        }
        "verify-note" => {
            let recording = take_value(words, "--recording")?
                .ok_or_else(|| UsageError("--recording is required".into()))?;
            Ok(Verb::VerifyNote {
                path: take_positional(words, "<path>")?.into(),
                recording,
            })
        }
        "run" => Ok(Verb::Run {
            dry_run: take_flag(words, "--dry-run"),
        }),
        "--version" => Ok(Verb::Version),
        "setup" => Ok(Verb::Setup {
            force: take_flag(words, "--force"),
        }),
        "doctor" => Ok(Verb::Doctor),
        "note" => match take_positional(words, "write")?.as_str() {
            "write" => Ok(Verb::NoteWrite {
                id: take_positional(words, "<id>")?,
            }),
            other => Err(UsageError(format!("unknown note subcommand {other}"))),
        },
        "ingest" => Ok(Verb::Ingest {
            dry_run: take_flag(words, "--dry-run"),
            once: take_value(words, "--once")?.map(PathBuf::from),
        }),
        "transcribe" => {
            let language = take_value(words, "--language")?;
            let dry_run = take_flag(words, "--dry-run");
            Ok(Verb::Transcribe {
                id: take_positional(words, "<id>")?,
                language,
                dry_run,
            })
        }
        "path" => {
            let stage = take_value(words, "--stage")?
                .ok_or_else(|| UsageError("--stage is required".into()))?;
            match take_value(words, "--occasion")? {
                Some(id) => Ok(Verb::OccasionPath { id, stage }),
                None => Ok(Verb::Path {
                    id: take_positional(words, "<id>")?,
                    stage,
                }),
            }
        }
        "show" => show::parse(words),
        "review" => review::parse(words, text.correction),
        "search" => search::parse(words),
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
