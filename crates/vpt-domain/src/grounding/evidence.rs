use super::corrections::EvidenceWord;
use super::{GroundingError, SourceRange, facts, sources::intersects, spend};
use std::collections::BTreeSet;

pub(super) fn collect(
    words: &[EvidenceWord<'_>],
    sources: &[SourceRange],
    window_secs: u32,
    remaining: &mut usize,
) -> Result<BTreeSet<String>, GroundingError> {
    let mut evidence = BTreeSet::new();
    for (start, end) in windows(sources, window_secs) {
        let mut text = String::new();
        for word in words {
            spend(remaining, 1)?;
            let selected = intersects(word.start, word.end, start, end)
                || (window_secs > 0 && word.end >= start && word.start <= end);
            if selected {
                spend(remaining, word.text.len() + 1)?;
                text.push_str(word.text);
                text.push(' ');
            } else if !text.is_empty() {
                extend(&mut evidence, &text, remaining)?;
                text.clear();
            }
        }
        extend(&mut evidence, &text, remaining)?;
    }
    Ok(evidence)
}

fn extend(
    evidence: &mut BTreeSet<String>,
    text: &str,
    remaining: &mut usize,
) -> Result<(), GroundingError> {
    for fact in facts::extract(text, remaining)? {
        if evidence.len() == super::MAX_ITEMS && !evidence.contains(&fact.key) {
            return Err(GroundingError::CapacityExceeded);
        }
        evidence.insert(fact.key);
    }
    Ok(())
}

fn windows(sources: &[SourceRange], window_secs: u32) -> Vec<(f64, f64)> {
    let mut ranges: Vec<_> = sources
        .iter()
        .map(|source| {
            (
                (source.start_secs as f64 - f64::from(window_secs)).max(0.0),
                source.end_secs as f64 + f64::from(window_secs),
            )
        })
        .collect();
    ranges.sort_by(|left, right| left.0.total_cmp(&right.0));
    let mut merged: Vec<(f64, f64)> = Vec::new();
    for (start, end) in ranges {
        if let Some(previous) = merged.last_mut()
            && start <= previous.1
        {
            previous.1 = previous.1.max(end);
        } else {
            merged.push((start, end));
        }
    }
    merged
}
