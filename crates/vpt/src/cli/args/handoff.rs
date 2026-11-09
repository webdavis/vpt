use super::*;
use vpt_domain::{handoff::Kind, identity::RecordingId, occasion::OccasionId};
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Target {
    Recording(RecordingId),
    Occasion(OccasionId),
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Options {
    pub target: Target,
    pub kind: Kind,
}
pub(super) fn parse(words: &mut Vec<String>) -> Result<Verb, UsageError> {
    let stage =
        take_value(words, "--stage")?.ok_or_else(|| UsageError("--stage is required".into()))?;
    let kind = Kind::parse(&stage).ok_or_else(|| {
        UsageError("handoff --stage must be released, transcript, analysis, or brief".into())
    })?;
    let id = take_positional(words, "<id>")?;
    let target = if kind == Kind::Brief {
        Target::Occasion(
            OccasionId::parse(&id).map_err(|_| UsageError("invalid occasion id".into()))?,
        )
    } else {
        Target::Recording(
            RecordingId::parse(&id).map_err(|_| UsageError("invalid recording id".into()))?,
        )
    };
    if !words.is_empty() {
        return Err(UsageError("unexpected handoff argument".into()));
    }
    Ok(Verb::Handoff(Options { target, kind }))
}
