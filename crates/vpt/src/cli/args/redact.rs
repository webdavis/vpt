use super::*;
use vpt_domain::{identity::RecordingId, note::NoteStage};
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Options {
    pub id: RecordingId,
    pub stage: NoteStage,
    pub destination: Option<PathBuf>,
    pub title: Option<text::PrivateText>,
}
pub(super) fn parse(words: &mut Vec<String>, text: text::Values) -> Result<Verb, UsageError> {
    if text.term.is_some()
        || text.correction.is_some()
        || text.reject_tag.is_some()
        || text.relation.is_some()
        || !text.tags.is_empty()
        || !text.participants.is_empty()
    {
        return Err(UsageError("invalid redact option".into()));
    }
    if text.title.as_ref().is_some_and(|t| {
        t.as_str().trim().is_empty() || t.as_str().len() > 65_536 || t.as_str().contains('\0')
    }) {
        return Err(UsageError("invalid --title".into()));
    }
    let stage = take_value(words, "--stage")?
        .map(|s| {
            NoteStage::parse(&s)
                .ok_or_else(|| UsageError("redact --stage must be transcript or analysis".into()))
        })
        .transpose()?
        .unwrap_or(NoteStage::Transcript);
    let destination = take_value(words, "--to")?.map(PathBuf::from);
    let id = RecordingId::parse(&take_positional(words, "<id>")?)
        .map_err(|_| UsageError("invalid recording id".into()))?;
    if !words.is_empty() {
        return Err(UsageError("unexpected redact argument".into()));
    }
    Ok(Verb::Redact(Options {
        id,
        stage,
        destination,
        title: text.title,
    }))
}
