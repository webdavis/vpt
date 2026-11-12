use super::*;
use vpt_domain::copy::Kind;

pub(super) fn parse(words: &mut Vec<String>) -> Result<Verb, UsageError> {
    let redact = take_flag(words, "--redact");
    let stage = take_value(words, "--stage")?
        .map(|stage| {
            Kind::parse(&stage).ok_or_else(|| {
                UsageError("show --stage must be released, transcript, analysis, or brief".into())
            })
        })
        .transpose()?;
    match (redact, stage) {
        (true, None) => return Err(UsageError("--redact requires --stage".into())),
        (true, Some(Kind::Brief)) => {
            return Err(UsageError(
                crate::commands::show::stage::BRIEF_REDACT.into(),
            ));
        }
        (true, Some(Kind::Released)) => {
            return Err(UsageError(
                "--redact does not apply to --stage released, which is already redacted".into(),
            ));
        }
        _ => (),
    }
    Ok(Verb::Show {
        id: take_positional(words, "<id>")?,
        stage,
        redact,
    })
}
