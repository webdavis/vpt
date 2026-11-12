mod candidates;
pub mod document;
pub mod report;
mod reviewed;
pub use candidates::Candidate;
pub use reviewed::{FlagPolicy, ReviewSource, redact_reviewed};
use std::ops::Range;
use vpt_domain::redaction::{Error, Mapping, Redactor};
#[derive(Default)]
pub struct ContentOptions {
    pub deny_sections: Vec<String>,
    pub keep_timecodes: bool,
}
pub struct Content {
    pub candidates: Vec<Candidate>,
    pub omitted: usize,
    pub marked: usize,
    pub text: String,
    pub mappings: Vec<Mapping>,
    pub lines: Vec<SourceLine>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceLine {
    pub line: usize,
    pub source: Range<usize>,
}
impl std::fmt::Debug for Content {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedactedContent")
            .field("masks", &self.mappings.len())
            .field("lines", &self.lines.len())
            .finish()
    }
}
mod blocks;
mod joined;
mod timecodes;
const MAX_TEXT: usize = 16 * 1024 * 1024;
const MAX_ITEMS: usize = 65_536;
pub fn redact_content(
    source: &str,
    redactor: &mut Redactor,
    options: &ContentOptions,
) -> Result<Content, Error> {
    render(source, &[], redactor, options)
}
fn render(
    source: &str,
    markers: &[Range<usize>],
    redactor: &mut Redactor,
    options: &ContentOptions,
) -> Result<Content, Error> {
    if options.deny_sections.len() > 256
        || options
            .deny_sections
            .iter()
            .try_fold(0usize, |n, s| n.checked_add(s.len()))
            .is_none_or(|n| n > 65_536)
    {
        return Err(Error::Capacity);
    }
    let denied: std::collections::BTreeSet<_> = options
        .deny_sections
        .iter()
        .map(|s| vpt_domain::normalize::canonical(s.trim()))
        .collect();
    let mut skip = None;
    let mut output = Content {
        candidates: vec![],
        omitted: 0,
        marked: 0,
        text: String::new(),
        mappings: vec![],
        lines: vec![],
    };
    let mut line = 1;
    let mut joined = joined::Joined::default();
    for block in blocks::read_with_markers(source, markers)? {
        if let Some(level) = block.heading {
            if skip.is_some_and(|previous| level <= previous) {
                skip = None;
            }
            if skip.is_none()
                && denied.contains(&vpt_domain::normalize::canonical(block.text.trim()))
            {
                skip = Some(level);
            }
        }
        if skip.is_some() {
            continue;
        }
        let (text, protected) =
            timecodes::prepare(&block.text, &block.protected, options.keep_timecodes)?;
        joined.push(&block.text, &text, &protected)?;
        let masked = redactor.apply_with_spans(&text, &protected)?;
        let text = masked.text.trim();
        if text.is_empty() {
            continue;
        }
        if !output.text.is_empty() {
            output.text.push('\n');
            line += 1;
        }
        candidates::body(&mut output.candidates, &masked, line)?;
        for text in text.lines() {
            let escaped = vpt_domain::note::escape_text(text).map_err(|_| Error::Capacity)?;
            if output.text.len() + escaped.len() + 8 > MAX_TEXT || output.lines.len() >= MAX_ITEMS {
                return Err(Error::Capacity);
            }
            if let Some(level) = block.heading {
                output.text.push_str(&"#".repeat(level));
                output.text.push(' ');
            }
            output.text.push_str(&escaped);
            output.text.push('\n');
            output.lines.push(SourceLine {
                line,
                source: block.source.clone(),
            });
            line += 1;
        }
    }
    let mut decoded = String::new();
    for block in blocks::read(&output.text)? {
        if decoded.len() + block.text.len() + 1 > MAX_TEXT {
            return Err(Error::Capacity);
        }
        decoded.push_str(&block.text);
        decoded.push('\n');
    }
    joined.check(redactor)?;
    redactor.verify(&decoded)?;
    output.mappings = redactor.mappings().to_vec();
    Ok(output)
}
#[cfg(test)]
mod tests;
