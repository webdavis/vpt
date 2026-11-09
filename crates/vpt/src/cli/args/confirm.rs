use super::*;
use vpt_domain::tags::Action;

pub(super) fn parse(words: &mut Vec<String>, mut text: text::Values) -> Result<Verb, UsageError> {
    if text.tags.len() > 1 {
        return Err(UsageError("--tag may only appear once".into()));
    }
    if usize::from(text.term.is_some())
        + usize::from(!text.tags.is_empty())
        + usize::from(text.reject_tag.is_some())
        + usize::from(text.relation.is_some())
        != 1
    {
        return Err(UsageError(
            "confirm requires exactly one of --term, --tag, --reject-tag, --relation".into(),
        ));
    }
    if let Some(term) = text.term {
        return Ok(Verb::ConfirmTerm { term });
    }
    if let Some(relation) = text.relation {
        return Ok(Verb::ConfirmRelation {
            id: take_positional(words, "<id>")?,
            relation,
        });
    }
    let (tag, action) = match (text.tags.pop(), text.reject_tag) {
        (Some(tag), _) => (tag, Action::Confirm),
        (_, Some(tag)) => (tag, Action::Reject),
        _ => unreachable!("one confirmation action was checked"),
    };
    Ok(Verb::ConfirmTag {
        id: take_positional(words, "<id>")?,
        tag,
        action,
    })
}
