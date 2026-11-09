use super::*;
use vpt_domain::{identity::RecordingId, note::NoteStage};
pub struct Header {
    pub title: Option<String>,
    pub date: Option<String>,
    pub source: String,
}
impl Default for Header {
    fn default() -> Self {
        Self {
            title: None,
            date: None,
            source: "redacted extract, not a verbatim record".into(),
        }
    }
}
pub fn owned_content<'a>(
    source: &'a str,
    id: &RecordingId,
    stage: NoteStage,
) -> Result<(&'a str, usize), Error> {
    let document = crate::note_document::Document::parse(source).map_err(|_| Error::InvalidText)?;
    document
        .check_owner(id, stage.as_str())
        .map_err(|_| Error::InvalidText)?;
    let length = document.content().map_err(|_| Error::InvalidText)?.len();
    let offset = source
        .split_inclusive('\n')
        .take(document.content_start_line() - 1)
        .map(str::len)
        .sum::<usize>();
    let content = source
        .get(offset..offset + length)
        .ok_or(Error::InvalidText)?;
    Ok((content, offset))
}
pub fn assemble(
    mut content: Content,
    header: &Header,
    redactor: &Redactor,
) -> Result<Content, Error> {
    if header
        .date
        .as_ref()
        .is_some_and(|date| date.len() != 10 || date.parse::<jiff::civil::Date>().is_err())
    {
        return Err(Error::InvalidText);
    }
    let mut fields = Vec::new();
    if let Some(title) = &header.title {
        if title.trim().is_empty() {
            return Err(Error::InvalidText);
        }
        fields.push(("title", title.as_str()));
    }
    if let Some(date) = &header.date {
        fields.push(("date", date.as_str()));
    }
    if !header.source.is_empty() {
        fields.push(("source", header.source.as_str()));
    }
    let mut prefix = String::new();
    let mut decoded = String::new();
    let mut candidates = Vec::new();
    if !fields.is_empty() {
        prefix.push_str("---\n");
        for (name, value) in fields {
            if value.len() > 65_536 {
                return Err(Error::Capacity);
            }
            if value.contains('\0') {
                return Err(Error::InvalidText);
            }
            let value_decoded = html_escape::decode_html_entities(value);
            candidates::header(&mut candidates, &value_decoded, prefix.lines().count() + 1)?;
            prefix.push_str(name);
            prefix.push_str(": ");
            prefix.push_str(&serde_json::to_string(value).map_err(|_| Error::InvalidText)?);
            prefix.push('\n');
            decoded.push_str(name);
            decoded.push_str(": ");
            decoded.push_str(&value_decoded);
            decoded.push('\n');
        }
        prefix.push_str("---\n\n");
    }
    if prefix.len() + content.text.len() > MAX_TEXT {
        return Err(Error::Capacity);
    }
    for block in blocks::read(&content.text)? {
        if decoded.len() + block.text.len() + 1 > MAX_TEXT {
            return Err(Error::Capacity);
        }
        decoded.push_str(&block.text);
        decoded.push('\n');
    }
    redactor.verify(&decoded)?;
    let extra = prefix.bytes().filter(|c| *c == b'\n').count();
    for line in &mut content.lines {
        line.line += extra;
    }
    if candidates.len() + content.candidates.len() > MAX_ITEMS {
        return Err(Error::Capacity);
    }
    for mut candidate in content.candidates {
        candidate.line += extra;
        candidates.push(candidate);
    }
    content.candidates = candidates;
    prefix.push_str(&content.text);
    content.text = prefix;
    Ok(content)
}
#[cfg(test)]
mod tests;
