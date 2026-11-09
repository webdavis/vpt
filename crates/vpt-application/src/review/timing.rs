use super::{Occurrence, Range, ReviewError};
use vpt_domain::transcript::Word;

pub(super) fn occurrence(
    range: &Range<usize>,
    words: &[&Word],
    duration: f64,
) -> Result<Occurrence, ReviewError> {
    if range.start > range.end || range.end > words.len() || !duration.is_finite() || duration < 0.0
    {
        return Err(ReviewError::InvalidFlags);
    }
    let (start, end) = if range.is_empty() {
        let at = words
            .get(range.start)
            .map(|word| word.start)
            .or_else(|| words.last().map(|word| word.end))
            .unwrap_or(0.0);
        (at, at)
    } else {
        let selected = &words[range.clone()];
        if selected.iter().any(|word| {
            !word.start.is_finite()
                || !word.end.is_finite()
                || word.start < 0.0
                || word.end < word.start
                || word.end > duration
        }) {
            return Err(ReviewError::InvalidFlags);
        }
        (
            selected[0].start,
            selected.iter().map(|word| word.end).fold(0.0, f64::max),
        )
    };
    if !start.is_finite() || !end.is_finite() || start < 0.0 || end < start || end > duration {
        return Err(ReviewError::InvalidFlags);
    }
    Ok(Occurrence {
        words: range.clone(),
        start,
        end,
    })
}
