use super::{Grounder, GroundingError, MAX_ITEMS, MAX_TEXT_BYTES};
use crate::{
    review::{FlagContent, ReviewFlag},
    transcript::Word,
};

impl<'a> Grounder<'a> {
    pub fn new(
        words: impl IntoIterator<Item = &'a Word>,
        flags: &'a [ReviewFlag],
        duration: f64,
        window_secs: u32,
    ) -> Result<Self, GroundingError> {
        if !duration.is_finite() || !(0.0..=9_007_199_254_740_992.0).contains(&duration) {
            return Err(GroundingError::InvalidTranscript);
        }
        let mut accepted = Vec::new();
        let mut bytes = 0;
        let mut previous_start = 0.0;
        for word in words {
            if accepted.len() == MAX_ITEMS
                || word.text.len() > MAX_ITEMS * 4
                || word.text.chars().count() > MAX_ITEMS
            {
                return Err(GroundingError::CapacityExceeded);
            }
            bytes += word.text.len();
            if bytes > MAX_TEXT_BYTES {
                return Err(GroundingError::CapacityExceeded);
            }
            if !word.start.is_finite()
                || !word.end.is_finite()
                || word.start < previous_start
                || word.end < word.start
                || word.end > duration
                || word.text.trim().is_empty()
            {
                return Err(GroundingError::InvalidTranscript);
            }
            previous_start = word.start;
            accepted.push(word);
        }
        if accepted.is_empty() {
            return Err(GroundingError::InvalidTranscript);
        }
        validate_flags(flags, accepted.len())?;
        let evidence = super::corrections::project(&accepted, flags)?;
        Ok(Self {
            words: accepted,
            evidence,
            labels: Vec::new(),
            flags,
            duration,
            window_secs,
            remaining: 64 * 1024 * 1024,
        })
    }
}

fn validate_flags(flags: &[ReviewFlag], words: usize) -> Result<(), GroundingError> {
    if flags.len() > MAX_ITEMS + 3 {
        return Err(GroundingError::CapacityExceeded);
    }
    let mut ranges = 0usize;
    for flag in flags {
        if let FlagContent::Lexical(flag) = &flag.content {
            ranges = ranges
                .checked_add(flag.ranges.len())
                .ok_or(GroundingError::CapacityExceeded)?;
            if ranges > MAX_ITEMS {
                return Err(GroundingError::CapacityExceeded);
            }
            if flag.ranges.is_empty()
                || flag
                    .ranges
                    .iter()
                    .any(|range| range.start > range.end || range.end > words)
            {
                return Err(GroundingError::InvalidFlags);
            }
        }
    }
    Ok(())
}
