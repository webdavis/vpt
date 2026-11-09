use super::*;

pub(super) fn rewrite(
    source: &Document<'_>,
    replacement: &str,
    previous_tags: &[String],
) -> Result<String, NoteError> {
    source.check_owner(&source.id)?;
    let replacement = Document::parse(replacement)?;
    replacement.check_owner(&source.id)?;
    let header = rewrite::header(
        &source.header_view(),
        &replacement.header_view(),
        previous_tags,
    )?;
    let body = Region::parse(&source.source[source.body..])
        .map_err(format_error)?
        .rewrite(replacement.content()?)
        .map_err(format_error)?;
    let length = source.header.start + header.len() + source.body - source.header.end + body.len();
    if length > vpt_domain::note::MAX_RENDERED {
        return Err(NoteError::CapacityExceeded);
    }
    let mut updated = String::with_capacity(length);
    updated.push_str(&source.source[..source.header.start]);
    updated.push_str(&header);
    updated.push_str(&source.source[source.header.end..source.body]);
    updated.push_str(&body);
    Document::parse(&updated)?.check_owner(&source.id)?;
    Ok(updated)
}
