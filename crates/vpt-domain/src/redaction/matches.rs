use super::*;
use std::{collections::BTreeMap, ops::Range};
pub(super) struct Match {
    pub range: Range<usize>,
    pub class: Class,
}
pub(super) fn push(found: &mut Vec<Match>, range: Range<usize>, class: Class) -> Result<(), Error> {
    if found.len() >= MAX_ITEMS {
        return Err(Error::Capacity);
    }
    found.push(Match { range, class });
    Ok(())
}
pub(super) fn select(mut found: Vec<Match>) -> Vec<Match> {
    found.sort_by_key(|m| (std::cmp::Reverse(m.range.len()), m.class, m.range.start));
    let mut selected = BTreeMap::<usize, Match>::new();
    for candidate in found {
        if selected
            .range(..=candidate.range.start)
            .next_back()
            .is_some_and(|(_, m)| m.range.end > candidate.range.start)
            || selected
                .range(candidate.range.start..)
                .next()
                .is_some_and(|(&start, _)| start < candidate.range.end)
        {
            continue;
        }
        selected.insert(candidate.range.start, candidate);
    }
    selected.into_values().collect()
}
