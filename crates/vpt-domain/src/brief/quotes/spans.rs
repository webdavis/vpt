use super::super::*;
use crate::normalize::TermMatcher;
use std::{collections::BTreeMap, ops::Range};

pub(super) struct Span {
    pub occurrences: Vec<(usize, Range<usize>)>,
}
pub(super) fn collect(
    source: &TranscriptSource,
    matched: &Match,
    remaining: &mut usize,
) -> Result<Vec<Span>, BriefError> {
    let names: Vec<_> = matched.values.iter().map(String::as_str).collect();
    let matcher = if matches!(matched.selector, Selector::Participant | Selector::Term) {
        Some(TermMatcher::new(&names).map_err(|_| BriefError::CapacityExceeded)?)
    } else {
        None
    };
    let mut unique = BTreeMap::new();
    let mut spans: Vec<Span> = vec![];
    let mut offset = 0;
    for (index, segment) in source.transcript.segments.iter().enumerate() {
        let words = offset..offset + segment.words.len();
        offset = words.end;
        if segment.text.trim().is_empty() {
            continue;
        }
        let mut matches = matcher.is_none();
        if let Some(matcher) = &matcher {
            let words: Vec<_> = segment.text.split_whitespace().collect();
            matcher
                .scan_budgeted(
                    &words,
                    |_, _| {
                        matches = true;
                        Ok(())
                    },
                    remaining,
                )
                .map_err(|_| BriefError::CapacityExceeded)?;
        }
        if !matches {
            continue;
        }
        let bits = |value: f64| if value == 0.0 { 0 } else { value.to_bits() };
        let key = (
            bits(segment.start),
            bits(segment.end),
            segment.text.as_str(),
        );
        let next = spans.len();
        let position = *unique.entry(key).or_insert_with(|| {
            spans.push(Span {
                occurrences: vec![],
            });
            next
        });
        spans[position].occurrences.push((index, words));
    }
    Ok(spans)
}
