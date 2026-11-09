use super::*;
use vpt_domain::review::ReviewAction;

pub(super) fn parse(
    words: &mut Vec<String>,
    correction: Option<text::PrivateText>,
) -> Result<Verb, UsageError> {
    let flag = take_value(words, "--resolve")?;
    let confirm = take_flag(words, "--confirm");
    let dismiss = take_flag(words, "--dismiss");
    let actions = usize::from(confirm) + usize::from(dismiss) + usize::from(correction.is_some());
    let id = take_positional(words, "<id>")?;
    if actions == 0 && flag.is_none() {
        return Ok(Verb::Review { id });
    }
    if actions != 1 || flag.is_none() {
        return Err(UsageError(
            "resolution needs --resolve and exactly one of --confirm, --correct, --dismiss".into(),
        ));
    }
    let action = match correction {
        Some(text) => ReviewAction::Correct(text.as_str().into()),
        None if confirm => ReviewAction::Confirm,
        None => ReviewAction::Dismiss,
    };
    Ok(Verb::ReviewResolve {
        id,
        flag: flag.unwrap(),
        action,
    })
}
