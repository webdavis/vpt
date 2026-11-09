use super::ResolutionError;
use vpt_domain::{
    review::{FlagContent, Resolution},
    uncertainty::Class,
};

pub(super) fn addition(
    content: &FlagContent,
    resolution: &Resolution,
) -> Result<Option<String>, ResolutionError> {
    let text = match (content, resolution) {
        (FlagContent::Lexical(_), Resolution::Corrected { text, .. }) => text,
        (FlagContent::Lexical(flag), Resolution::Confirmed { .. })
            if matches!(flag.class, Class::ProperNoun | Class::AgreedUnverified) =>
        {
            &flag.record_text
        }
        _ => return Ok(None),
    };
    if text.len() > 262_144
        || text.chars().count() > 65_536
        || text
            .chars()
            .any(|ch| ch.is_control() && !ch.is_whitespace())
    {
        return Err(ResolutionError::InvalidTerm);
    }
    let term = text.split_whitespace().collect::<Vec<_>>().join(" ");
    Ok((!term.is_empty()).then_some(term))
}
