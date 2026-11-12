use super::*;
use vpt_domain::occasion::time::OccasionTime;
use vpt_domain::time::UtcInstant;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Options {
    pub file: PathBuf,
    pub title: Option<text::PrivateText>,
    pub captured_at: Option<UtcInstant>,
}

pub(super) fn parse(words: &mut Vec<String>, text: text::Values) -> Result<Verb, UsageError> {
    if text.term.is_some()
        || text.reject_tag.is_some()
        || text.relation.is_some()
        || !text.tags.is_empty()
        || !text.participants.is_empty()
    {
        return Err(UsageError("invalid import option".into()));
    }
    if text
        .title
        .as_ref()
        .is_some_and(|title| title.as_str().trim().is_empty() || title.as_str().contains('\0'))
    {
        return Err(UsageError("invalid --title".into()));
    }
    let captured_at = take_value(words, "--captured-at")?
        .map(|value| {
            OccasionTime::parse(&value)
                .map(|time| time.unix_second())
                .map_err(|_| UsageError("invalid --captured-at RFC 3339 time".into()))
        })
        .transpose()?;
    let file = take_positional(words, "<file>")?.into();
    if !words.is_empty() {
        return Err(UsageError("unexpected import argument".into()));
    }
    Ok(Verb::Import(Options {
        file,
        title: text.title,
        captured_at,
    }))
}
