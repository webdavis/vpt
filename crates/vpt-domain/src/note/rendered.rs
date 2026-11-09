use super::*;
use crate::digest::Sha256Digest;
use std::ops::Range;
#[derive(Clone, PartialEq, Eq)]
pub struct OpenSpan {
    pub flag: Sha256Digest,
    pub occurrence: usize,
    pub bytes: Range<usize>,
    pub accepted: String,
}
impl std::fmt::Debug for OpenSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenSpan")
            .field("flag", &self.flag)
            .field("occurrence", &self.occurrence)
            .field("bytes", &self.bytes)
            .finish_non_exhaustive()
    }
}
#[derive(Clone, PartialEq, Eq)]
pub struct RenderedTranscript {
    pub text: String,
    pub open_spans: Vec<OpenSpan>,
}
impl std::fmt::Debug for RenderedTranscript {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RenderedTranscript")
            .field("spans", &self.open_spans.len())
            .finish_non_exhaustive()
    }
}
pub fn transcript_with_spans(
    transcript: &Transcript,
    flags: &[ReviewFlag],
    readability: bool,
) -> Result<RenderedTranscript, RenderError> {
    validate::inputs(transcript, flags)?;
    let projected = projection::project(transcript);
    let spans = annotations::spans(&projected, flags)?;
    let mut output = writer::Writer::default();
    let mut open_spans = Vec::new();
    output.literal("## Transcript\n")?;
    for ((segment, text), spans) in transcript
        .segments
        .iter()
        .zip(&projected.segments)
        .zip(spans)
    {
        output.literal("\n")?;
        let seconds = segment.start as u64;
        output.literal(&format!("[{:02}:{:02}] ", seconds / 60, seconds % 60))?;
        open_spans.extend(annotations::render(&mut output, text, &spans, readability)?);
        output.literal("\n")?;
    }
    Ok(RenderedTranscript {
        text: output.finish(),
        open_spans,
    })
}
#[cfg(test)]
mod tests;
