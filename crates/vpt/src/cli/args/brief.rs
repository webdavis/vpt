use super::*;
use vpt_domain::occasion::{OccasionDetails, OccasionId, Participant, time::OccasionTime};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Target {
    Existing(OccasionId),
    Manual(OccasionDetails),
    Upcoming,
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Options {
    pub target: Target,
    pub dry_run: bool,
    pub explain: bool,
}
pub(super) fn parse(words: &mut Vec<String>, text: text::Values) -> Result<Verb, UsageError> {
    if text.term.is_some()
        || text.correction.is_some()
        || text.reject_tag.is_some()
        || text.relation.is_some()
    {
        return Err(UsageError("invalid brief option".into()));
    }
    let dry_run = take_flag(words, "--dry-run");
    let explain = take_flag(words, "--explain");
    let upcoming = take_flag(words, "--upcoming");
    let at = take_value(words, "--at")?;
    let duration = take_value(words, "--duration-minutes")?;
    let target = if let Some(title) = text.title {
        if upcoming {
            return Err(UsageError(
                "--upcoming cannot use manual occasion options".into(),
            ));
        }
        let at = at.ok_or_else(|| UsageError("--at is required with --title".into()))?;
        let at = OccasionTime::parse(&at)
            .map_err(|_| UsageError("invalid --at RFC 3339 time".into()))?;
        let duration_secs = duration
            .map(|value| {
                value
                    .parse::<u64>()
                    .ok()
                    .and_then(|v| v.checked_mul(60))
                    .ok_or_else(|| UsageError("invalid --duration-minutes".into()))
            })
            .transpose()?;
        let details = OccasionDetails {
            at,
            title: title.as_str().into(),
            duration_secs,
            participants: text
                .participants
                .into_iter()
                .map(|p| Participant {
                    name: p.as_str().into(),
                    email: None,
                })
                .collect(),
            tags: text
                .tags
                .into_iter()
                .map(|tag| tag.as_str().into())
                .collect(),
        };
        if !details.is_valid() {
            return Err(UsageError("invalid manual occasion".into()));
        }
        Target::Manual(details)
    } else {
        if at.is_some()
            || duration.is_some()
            || !text.tags.is_empty()
            || !text.participants.is_empty()
        {
            return Err(UsageError("manual occasion options require --title".into()));
        }
        if upcoming {
            if dry_run || explain {
                return Err(UsageError(
                    "--upcoming cannot use --dry-run or --explain".into(),
                ));
            }
            Target::Upcoming
        } else {
            let id = take_positional(words, "<occasion-id>")?;
            Target::Existing(
                OccasionId::parse(&id).map_err(|_| UsageError("invalid occasion id".into()))?,
            )
        }
    };
    if !words.is_empty() {
        return Err(UsageError("unexpected brief argument".into()));
    }
    Ok(Verb::Brief(Options {
        target,
        dry_run,
        explain,
    }))
}
