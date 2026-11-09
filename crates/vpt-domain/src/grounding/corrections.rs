use super::{GroundingError, MAX_ITEMS, MAX_TEXT_BYTES};
use crate::{
    review::{FlagContent, Resolution, ReviewFlag, valid_correction},
    transcript::Word,
};

pub(super) struct EvidenceWord<'a> {
    pub text: &'a str,
    pub start: f64,
    pub end: f64,
}

pub(super) fn project<'a>(
    words: &[&'a Word],
    flags: &'a [ReviewFlag],
) -> Result<Vec<EvidenceWord<'a>>, GroundingError> {
    let mut corrections = Vec::new();
    for flag in flags {
        let Resolution::Corrected { text, .. } = &flag.resolution else {
            continue;
        };
        if matches!(flag.content, FlagContent::Verification(_)) {
            continue;
        }
        let FlagContent::Lexical(lexical) = &flag.content else {
            return Err(GroundingError::InvalidFlags);
        };
        if !lexical.class.surfaced() || !valid_correction(text) {
            return Err(GroundingError::InvalidFlags);
        }
        corrections.extend(lexical.ranges.iter().map(|range| (range, text.as_str())));
    }
    corrections.sort_by_key(|(range, _)| (range.start, range.end));
    if corrections
        .windows(2)
        .any(|pair| pair[0].0.end > pair[1].0.start || pair[0].0 == pair[1].0)
    {
        return Err(GroundingError::InvalidFlags);
    }
    let mut projected = Vec::new();
    let mut bytes = 0;
    let mut cursor = 0;
    for (range, text) in corrections {
        for word in &words[cursor..range.start] {
            push(
                &mut projected,
                &mut bytes,
                word.text.as_str(),
                word.start,
                word.end,
            )?;
        }
        let start = words
            .get(range.start)
            .map_or(words[words.len() - 1].end, |word| word.start);
        let end = words[range.clone()]
            .iter()
            .fold(start, |end, word| end.max(word.end));
        push(&mut projected, &mut bytes, text, start, end)?;
        cursor = range.end;
    }
    for word in &words[cursor..] {
        push(
            &mut projected,
            &mut bytes,
            word.text.as_str(),
            word.start,
            word.end,
        )?;
    }
    Ok(projected)
}

fn push<'a>(
    words: &mut Vec<EvidenceWord<'a>>,
    bytes: &mut usize,
    text: &'a str,
    start: f64,
    end: f64,
) -> Result<(), GroundingError> {
    *bytes += text.len();
    if *bytes > MAX_TEXT_BYTES || words.len() == 2 * MAX_ITEMS {
        return Err(GroundingError::CapacityExceeded);
    }
    words.push(EvidenceWord { text, start, end });
    Ok(())
}
