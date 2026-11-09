use super::*;
use std::collections::{HashMap, HashSet};
mod additions;
mod metadata;
pub(super) use metadata::{View, rewrite as header};
pub(super) mod counts;
mod edits;
mod properties;
mod remove;
mod values;
use edits::Edit;

pub(super) fn rewrite(
    source: &Document<'_>,
    replacement: &str,
    previous_tags: &[String],
) -> Result<String, NoteError> {
    source.check_owner(&source.recording, &source.stage)?;
    let replacement = Document::parse(replacement)?;
    replacement.check_owner(&source.recording, &source.stage)?;
    let header = metadata::rewrite(
        &View::from(source),
        &View::from(&replacement),
        previous_tags,
    )?;
    let current_body = vpt_domain::note::managed::Regions::parse(&source.source[source.body..])
        .map_err(|_| NoteError::Markers)?;
    let next_body =
        vpt_domain::note::managed::Regions::parse(&replacement.source[replacement.body..])
            .map_err(|_| NoteError::Markers)?;
    let body = current_body
        .rewrite(next_body.links(), next_body.content())
        .map_err(|_| NoteError::Markers)?;
    if source.header.start + header.len() + source.body - source.header.end + body.len()
        > vpt_domain::note::MAX_RENDERED
    {
        return Err(NoteError::CapacityExceeded);
    }
    let result = format!(
        "{}{}{}{}",
        &source.source[..source.header.start],
        header,
        &source.source[source.header.end..source.body],
        body
    );
    Document::parse(&result)?.check_owner(&source.recording, &source.stage)?;
    Ok(result)
}
