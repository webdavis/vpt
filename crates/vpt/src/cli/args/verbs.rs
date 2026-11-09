use super::*;

pub(super) fn parse_verb(words: &mut Vec<String>, text: text::Values) -> Result<Verb, UsageError> {
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
        "show" => Ok(Verb::Show {
            id: take_positional(words, "<id>")?,
        }),
        "list" => Ok(Verb::List {
            stage: take_value(words, "--stage")?,
        }),
        "storage" => Ok(Verb::Storage),
        "symlink" => match take_positional(words, "deploy|verify")?.as_str() {
            "deploy" => Ok(Verb::SymlinkDeploy),
            "verify" => Ok(Verb::SymlinkVerify),
            other => Err(UsageError(format!("unknown symlink subcommand {other}"))),
        },
        other => Err(UsageError(format!("unknown verb {other}"))),
    }
}
