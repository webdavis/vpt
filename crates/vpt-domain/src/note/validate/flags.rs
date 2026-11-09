use super::*;

pub(super) fn validate(flags: &[ReviewFlag], transcript: &Transcript) -> Result<(), RenderError> {
    let words: Vec<_> = transcript
        .segments
        .iter()
        .flat_map(|segment| &segment.words)
        .collect();
    let mut ranges = Vec::new();
    let mut bytes = 0usize;
    for flag in flags {
        if let FlagContent::Lexical(lexical) = &flag.content {
            if lexical.ranges.len() > MAX_ITEMS - ranges.len() {
                return Err(RenderError::CapacityExceeded);
            }
            bytes = bytes
                .saturating_add(lexical.record_text.len())
                .saturating_add(lexical.alternative_text.as_ref().map_or(0, String::len));
            if lexical.ranges.is_empty()
                || lexical
                    .ranges
                    .iter()
                    .any(|r| r.start > r.end || r.end > words.len())
            {
                return Err(RenderError::InvalidFlags);
            }
            ranges.extend(lexical.ranges.iter().map(|range| (range, lexical)));
        }
        if let Resolution::Corrected { text, .. } = &flag.resolution {
            bytes = bytes.saturating_add(text.len());
        }
        if bytes > 2 * MAX_SOURCE {
            return Err(RenderError::CapacityExceeded);
        }
    }
    ranges.sort_by_key(|(range, _)| (range.start, range.end));
    for pair in ranges.windows(2) {
        if pair[0].0.end > pair[1].0.start || pair[0].0 == pair[1].0 {
            return Err(RenderError::InvalidFlags);
        }
    }
    for (range, lexical) in ranges {
        let expected = words[range.clone()]
            .iter()
            .enumerate()
            .flat_map(|(index, word)| {
                (index != 0)
                    .then_some(b' ')
                    .into_iter()
                    .chain(word.text.bytes())
            });
        if !expected.eq(lexical.record_text.bytes()) {
            return Err(RenderError::InvalidFlags);
        }
    }
    Ok(())
}
