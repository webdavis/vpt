mod rendered;
mod stage;
use crate::{review::ReviewFlag, transcript::Transcript};
pub use rendered::{OpenSpan, RenderedTranscript, corrected_segments, transcript_with_spans};
pub use stage::NoteStage;
mod analysis;
mod annotations;
pub use analysis::{
    FINDING_CORRECTION_MARKER, analysis_content, finding_annotation, resolved_finding_annotation,
};
pub mod document;
pub mod filename;
pub mod links;
pub mod managed;
mod projection;
pub(crate) mod validate;
pub(crate) mod writer;

const MAX_ITEMS: usize = 65_536;
const MAX_SOURCE: usize = 16 * 1024 * 1024;
pub const MAX_RENDERED: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderError {
    InvalidTranscript,
    InvalidProposal,
    InvalidFlags,
    Markers,
    InvalidName,
    InvalidMetadata,
    CapacityExceeded,
}

pub fn escape_text(text: &str) -> Result<String, RenderError> {
    let mut output = writer::Writer::default();
    output.text(text)?;
    Ok(output.finish())
}

pub fn transcript_content(
    transcript: &Transcript,
    flags: &[ReviewFlag],
    readability: bool,
) -> Result<String, RenderError> {
    transcript_with_spans(transcript, flags, readability).map(|value| value.text)
}

#[cfg(test)]
mod tests;
