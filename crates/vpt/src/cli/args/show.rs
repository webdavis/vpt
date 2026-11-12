use super::*;
use vpt_domain::handoff::Kind;

pub(super) fn parse(words: &mut Vec<String>) -> Result<Verb, UsageError> {
    let stage = take_value(words, "--stage")?
        .map(|stage| {
            Kind::parse(&stage).ok_or_else(|| {
                UsageError("show --stage must be released, transcript, analysis, or brief".into())
            })
        })
        .transpose()?;
    Ok(Verb::Show {
        id: take_positional(words, "<id>")?,
        stage,
    })
}
