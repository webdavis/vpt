mod body;
mod header;
mod region;
use super::{
    document::{Document, DocumentError},
    *,
};
use crate::note::{
    RenderError,
    document::Profile,
    links::{self, LinkStyle},
    writer::Writer,
};
pub use region::Region;
use std::path::Path;

pub struct Note<'a> {
    pub document: Document<'a>,
    pub filename: &'a str,
    pub origin: &'a Path,
    pub link_style: LinkStyle,
    pub wiki_root: Option<&'a Path>,
    pub profile: Profile<'a>,
}

pub const START: &str = "<!-- vpt:brief start -->";
pub const END: &str = "<!-- vpt:brief end -->";

pub fn create(note: &Note<'_>) -> Result<String, RenderError> {
    note.document.validate().map_err(|error| match error {
        DocumentError::CapacityExceeded => RenderError::CapacityExceeded,
        DocumentError::Invalid => RenderError::InvalidMetadata,
    })?;
    let heading = note
        .filename
        .strip_suffix(".md")
        .ok_or(RenderError::InvalidName)?;
    if heading.is_empty()
        || note
            .filename
            .chars()
            .any(|ch| ch.is_control() || matches!(ch, '/' | '\\'))
    {
        return Err(RenderError::InvalidName);
    }
    if note.filename.len() > 4 * 65_536 || note.filename.chars().count() > 65_536 {
        return Err(RenderError::CapacityExceeded);
    }
    for path in std::iter::once(note.origin).chain(note.wiki_root) {
        if path != Path::new("/") && document::source_path(path).is_none() {
            return Err(RenderError::InvalidName);
        }
    }
    let mut output = Writer::default();
    header::write(&mut output, note)?;
    output.literal("# ")?;
    output.text(heading)?;
    output.literal(&format!("\n\n{START}\n"))?;
    body::write(&mut output, note)?;
    output.literal(&format!("{END}\n"))?;
    Ok(output.finish())
}

pub fn filename(document: &Document<'_>, maximum: usize) -> Result<String, RenderError> {
    let slug = crate::note::filename::sanitized_title(&document.occasion.title, maximum)?;
    Ok(format!(
        "{}-{slug}-{}.md",
        &document.occasion.at.as_str()[..10],
        &document.id.hash12()[..8]
    ))
}

fn reference(note: &Note<'_>, path: &Path) -> Result<String, RenderError> {
    let label = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or(RenderError::InvalidName)?;
    links::reference(note.link_style, label, path, note.origin, note.wiki_root)
}
