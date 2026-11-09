use super::corrupt;
use std::collections::HashSet;
use vpt_application::ports::{AcceptedTranscript, LedgerError};
use vpt_domain::uncertainty::Class;

pub(super) fn classification(accepted: &AcceptedTranscript) -> Result<(), LedgerError> {
    let flags = &accepted.uncertainty;
    if flags.lexical.len() > 65_536
        || flags.diagnostics.len() > 3
        || (!accepted.lexical_classification && !flags.lexical.is_empty())
    {
        return Err(corrupt());
    }
    let mut diagnostics = HashSet::new();
    for diagnostic in &flags.diagnostics {
        if !diagnostics.insert(diagnostic.as_str()) {
            return Err(corrupt());
        }
    }
    let words: Vec<_> = accepted
        .transcript
        .segments
        .iter()
        .flat_map(|segment| &segment.words)
        .collect();
    let mut occupied = vec![false; words.len()];
    let mut gaps = HashSet::new();
    let mut occurrences = 0usize;
    let mut bytes = 0usize;
    for flag in &flags.lexical {
        bytes = bytes
            .saturating_add(flag.record_text.len())
            .saturating_add(flag.alternative_text.as_ref().map_or(0, String::len));
        occurrences = occurrences.saturating_add(flag.ranges.len());
        let disagreement = matches!(
            flag.class,
            Class::Formatting | Class::Numeric | Class::ProperNoun | Class::Other
        );
        if bytes > 32 * 1024 * 1024
            || occurrences > 65_536
            || flag.ranges.is_empty()
            || (flag.ranges.len() != 1 && flag.class != Class::AgreedUnverified)
            || flag.alternative_text.is_some() != disagreement
            || flag
                .confidence
                .is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
        {
            return Err(corrupt());
        }
        let mut previous_end = None;
        let mut confidence = None;
        for range in &flag.ranges {
            if range.start > range.end
                || range.end > words.len()
                || previous_end.is_some_and(|end| range.start < end)
            {
                return Err(corrupt());
            }
            previous_end = Some(range.end);
            if range.is_empty() {
                if !disagreement
                    || flag.alternative_text.as_ref().is_none_or(String::is_empty)
                    || !flag.record_text.is_empty()
                    || !gaps.insert(range.start)
                {
                    return Err(corrupt());
                }
                continue;
            }
            if occupied[range.clone()].iter().any(|occupied| *occupied) {
                return Err(corrupt());
            }
            occupied[range.clone()].fill(true);
            let text = words[range.clone()]
                .iter()
                .map(|word| word.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            if text != flag.record_text {
                return Err(corrupt());
            }
            confidence = confidence
                .into_iter()
                .chain(
                    words[range.clone()]
                        .iter()
                        .filter_map(|word| word.confidence),
                )
                .reduce(f64::min);
        }
        if confidence != flag.confidence {
            return Err(corrupt());
        }
    }
    Ok(())
}
