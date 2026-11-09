use super::{MAX_RENDERED, RenderError, writer::Writer};
use std::ops::Range;

pub const LINKS_START: &str = "<!-- vpt:links start -->";
pub const LINKS_END: &str = "<!-- vpt:links end -->";
pub const CONTENT_START: &str = "<!-- vpt:content start -->";
pub const CONTENT_END: &str = "<!-- vpt:content end -->";
const MARKERS: [&str; 4] = [LINKS_START, LINKS_END, CONTENT_START, CONTENT_END];

pub struct Regions<'a> {
    source: &'a str,
    links: Range<usize>,
    content: Range<usize>,
}

impl<'a> Regions<'a> {
    pub fn parse(source: &'a str) -> Result<Self, RenderError> {
        if source.len() > MAX_RENDERED {
            return Err(RenderError::CapacityExceeded);
        }
        let links_start = marker(source, LINKS_START)?;
        let links_end = marker(source, LINKS_END)?;
        let content_start = marker(source, CONTENT_START)?;
        let content_end = marker(source, CONTENT_END)?;
        if links_start.end > links_end.start
            || content_start.end > content_end.start
            || !(links_end.end <= content_start.start || content_end.end <= links_start.start)
        {
            return Err(RenderError::Markers);
        }
        Ok(Self {
            source,
            links: links_start.end..links_end.start,
            content: content_start.end..content_end.start,
        })
    }

    pub fn content(&self) -> &'a str {
        &self.source[self.content.clone()]
    }

    pub fn links(&self) -> &'a str {
        &self.source[self.links.clone()]
    }

    pub fn rewrite(&self, links: &str, content: &str) -> Result<String, RenderError> {
        for value in [links, content] {
            if value.len() > MAX_RENDERED {
                return Err(RenderError::CapacityExceeded);
            }
            if MARKERS.iter().any(|marker| value.contains(marker)) {
                return Err(RenderError::Markers);
            }
        }
        let mut replacements = [(&self.links, links), (&self.content, content)];
        replacements.sort_by_key(|(range, _)| range.start);
        let mut output = Writer::default();
        let mut cursor = 0;
        for (range, replacement) in replacements {
            output.literal(&self.source[cursor..range.start])?;
            output.literal(replacement)?;
            if !replacement.is_empty() && !replacement.ends_with('\n') {
                output.literal("\n")?;
            }
            cursor = range.end;
        }
        output.literal(&self.source[cursor..])?;
        Ok(output.finish())
    }
}

pub(crate) fn marker(source: &str, token: &str) -> Result<Range<usize>, RenderError> {
    let mut occurrences = source.match_indices(token);
    let (start, _) = occurrences.next().ok_or(RenderError::Markers)?;
    if occurrences.next().is_some() || (start > 0 && source.as_bytes()[start - 1] != b'\n') {
        return Err(RenderError::Markers);
    }
    let end = start + token.len();
    let suffix = &source[end..];
    let newline = if suffix.starts_with("\r\n") {
        2
    } else if suffix.starts_with('\n') {
        1
    } else if suffix.is_empty() {
        0
    } else {
        return Err(RenderError::Markers);
    };
    Ok(start..end + newline)
}

#[cfg(test)]
mod tests;
