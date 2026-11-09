use super::*;
use serde::Serialize;
#[derive(Serialize)]
struct Report<'a> {
    schema: &'static str,
    source_path: &'a str,
    source_artifact_sha256: String,
    released_content_sha256: String,
    omitted: usize,
    marked: usize,
    mappings: Vec<Mask<'a>>,
    lines: Vec<Line>,
}
#[derive(Serialize)]
struct Mask<'a> {
    class: &'static str,
    number: usize,
    label: &'a str,
    value: &'a str,
}
#[derive(Serialize)]
struct Line {
    line: usize,
    source_bytes: Option<Span>,
}
#[derive(Serialize)]
struct Span {
    start: usize,
    end: usize,
}
pub fn encode(
    content: &Content,
    source: &str,
    content_offset: usize,
    source_path: &std::path::Path,
) -> Result<String, Error> {
    if source.len() > vpt_domain::note::MAX_RENDERED
        || content.text.len() > MAX_TEXT
        || content.mappings.len() > MAX_ITEMS
        || content.lines.len() > MAX_ITEMS
        || content.omitted > MAX_ITEMS
        || content.marked > MAX_ITEMS
    {
        return Err(Error::Capacity);
    }
    let source_path =
        vpt_domain::brief::document::source_path(source_path).ok_or(Error::InvalidText)?;
    if source_path.chars().any(char::is_control) || !source.is_char_boundary(content_offset) {
        return Err(Error::InvalidText);
    }
    let mut lines = Vec::new();
    let mut spans = content.lines.iter().peekable();
    for (index, _) in content.text.lines().enumerate() {
        if index >= MAX_ITEMS {
            return Err(Error::Capacity);
        }
        let source_bytes = if spans.peek().is_some_and(|s| s.line == index + 1) {
            let span = spans.next().ok_or(Error::InvalidText)?;
            let start = content_offset
                .checked_add(span.source.start)
                .ok_or(Error::InvalidText)?;
            let end = content_offset
                .checked_add(span.source.end)
                .ok_or(Error::InvalidText)?;
            if start > end || source.get(start..end).is_none() {
                return Err(Error::InvalidText);
            }
            Some(Span { start, end })
        } else {
            None
        };
        lines.push(Line {
            line: index + 1,
            source_bytes,
        });
    }
    if spans.next().is_some() {
        return Err(Error::InvalidText);
    }
    let mut bytes = 0usize;
    let mut mappings = Vec::new();
    for (index, m) in content.mappings.iter().enumerate() {
        bytes = bytes
            .checked_add(m.value.len())
            .and_then(|b| b.checked_add(m.label.len()))
            .ok_or(Error::Capacity)?;
        if bytes > MAX_TEXT {
            return Err(Error::Capacity);
        }
        if m.number != index + 1 || m.label.is_empty() || m.value.is_empty() {
            return Err(Error::InvalidText);
        }
        mappings.push(Mask {
            class: m.class.as_str(),
            number: m.number,
            label: &m.label,
            value: &m.value,
        });
    }
    let digest = crate::ledger::brief::validate::digest;
    crate::ledger::json::encode(
        &Report {
            schema: "vpt.redaction-report/1",
            source_path,
            source_artifact_sha256: digest(source.as_bytes()).hex(),
            released_content_sha256: digest(content.text.as_bytes()).hex(),
            omitted: content.omitted,
            marked: content.marked,
            mappings,
            lines,
        },
        MAX_TEXT,
    )
    .map_err(|_| Error::Capacity)
}
#[cfg(test)]
mod tests;
