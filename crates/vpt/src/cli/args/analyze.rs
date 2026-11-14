use super::*;
use vpt_domain::occasion::time::OccasionTime;
use vpt_domain::time::UtcInstant;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Scope {
    All { since: Option<UtcInstant> },
    Stdin,
}

pub(super) fn parse(words: &mut Vec<String>) -> Result<Verb, UsageError> {
    let dry_run = take_flag(words, "--dry-run");
    let all = take_flag(words, "--all");
    let stdin = take_flag(words, "--stdin");
    let resend_refused = take_flag(words, "--resend-refused");
    let since = take_value(words, "--since")?
        .map(|value| {
            OccasionTime::parse(&value)
                .map(|time| time.unix_second())
                .map_err(|_| UsageError("invalid --since RFC 3339 time".into()))
        })
        .transpose()?;
    if since.is_some() && !all {
        return Err(UsageError("--since requires --all".into()));
    }
    let scope = match (all, stdin) {
        (true, true) => return Err(UsageError("--all conflicts with --stdin".into())),
        (false, false) => {
            return Ok(Verb::Analyze {
                id: take_positional(words, "<id>")?,
                dry_run,
                resend_refused,
            });
        }
        _ if resend_refused => {
            return Err(UsageError("--resend-refused takes one <id>".into()));
        }
        (true, false) => Scope::All { since },
        (false, true) => Scope::Stdin,
    };
    if !words.is_empty() {
        return Err(UsageError("--all and --stdin take no <id>".into()));
    }
    Ok(Verb::AnalyzeBatch { scope, dry_run })
}
