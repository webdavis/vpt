use super::{MAX_ITEMS, MAX_SOURCE, RenderError};
use crate::{
    review::{FlagContent, Resolution, ReviewFlag},
    transcript::Transcript,
};
mod flags;

pub(crate) fn inputs(transcript: &Transcript, flags: &[ReviewFlag]) -> Result<(), RenderError> {
    if transcript.segments.len() > MAX_ITEMS || flags.len() > MAX_ITEMS + 3 {
        return Err(RenderError::CapacityExceeded);
    }
    if transcript.segments.is_empty() {
        return Err(RenderError::InvalidTranscript);
    }
    let mut bytes = 0usize;
    let mut words = 0usize;
    let mut previous = 0.0;
    for segment in &transcript.segments {
        bytes = bytes.saturating_add(segment.text.len());
        words = words.saturating_add(segment.words.len());
        if words > MAX_ITEMS || bytes > MAX_SOURCE {
            return Err(RenderError::CapacityExceeded);
        }
        if !range(segment.start, segment.end, previous, u64::MAX as f64)
            || segment.end >= u64::MAX as f64
        {
            return Err(RenderError::InvalidTranscript);
        }
        previous = segment.start;
        let mut last_word = segment.start;
        for word in &segment.words {
            bytes = bytes.saturating_add(word.text.len());
            if bytes > MAX_SOURCE {
                return Err(RenderError::CapacityExceeded);
            }
            if word.text.trim().is_empty() || !range(word.start, word.end, last_word, segment.end) {
                return Err(RenderError::InvalidTranscript);
            }
            last_word = word.start;
        }
    }
    flags::validate(flags, transcript)
}

fn range(start: f64, end: f64, lower: f64, upper: f64) -> bool {
    start.is_finite() && end.is_finite() && lower <= start && start <= end && end <= upper
}
