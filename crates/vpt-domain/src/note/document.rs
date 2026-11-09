use super::{RenderError, managed, writer::Writer};
use crate::{
    identity::RecordingId,
    time::{UtcInstant, UtcOffset},
};
mod frontmatter;
pub(crate) mod yaml;

pub struct RecordingNote<'a> {
    pub stage: super::NoteStage,
    pub recording: &'a RecordingId,
    pub filename: &'a str,
    pub captured_at: UtcInstant,
    pub offset: UtcOffset,
    pub duration_secs: u64,
    pub language: &'a str,
    pub engines: &'a [String],
    pub readability: bool,
    pub open_flags: usize,
    pub diagnostics: usize,
    pub tags: &'a [String],
    pub suggested_tags: &'a [String],
    pub profile: Profile<'a>,
    pub links: &'a str,
    pub content: &'a str,
}

pub enum Profile<'a> {
    Portable,
    Obsidian {
        title: Option<&'a str>,
        reference: &'a str,
        hub: &'a str,
        first_written: UtcInstant,
        write_offset: UtcOffset,
    },
}

pub fn create(note: &RecordingNote<'_>) -> Result<String, RenderError> {
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
    if note.filename.len() > 4 * 65_536 {
        return Err(RenderError::CapacityExceeded);
    }
    if note.offset.secs.unsigned_abs() >= 86_400
        || crate::identity::local_timestamp(note.captured_at, note.offset)
            != note.recording.local_timestamp()
        || note.language.trim().is_empty()
    {
        return Err(RenderError::InvalidMetadata);
    }
    let mut output = Writer::default();
    frontmatter::write(&mut output, note)?;
    output.literal("# ")?;
    output.text(heading)?;
    output.literal(&format!("\n\n{}\n", managed::LINKS_START))?;
    output.literal(note.links)?;
    if !note.links.is_empty() && !note.links.ends_with('\n') {
        output.literal("\n")?;
    }
    output.literal(&format!(
        "{}\n\n{}\n",
        managed::LINKS_END,
        managed::CONTENT_START
    ))?;
    output.literal(note.content)?;
    if !note.content.is_empty() && !note.content.ends_with('\n') {
        output.literal("\n")?;
    }
    output.literal(managed::CONTENT_END)?;
    output.literal("\n")?;
    let document = output.finish();
    managed::Regions::parse(&document)?;
    Ok(document)
}

#[cfg(test)]
mod tests;
