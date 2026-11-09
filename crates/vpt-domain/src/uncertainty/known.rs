use super::ClassificationError;
use crate::normalize::TermMatcher;

pub(super) fn coverage(words: &[&str], terms: &[&str]) -> Result<Vec<bool>, ClassificationError> {
    if words.len() > 65_536 {
        return Err(ClassificationError::CapacityExceeded);
    }
    let matcher = TermMatcher::new(terms).map_err(|_| ClassificationError::CapacityExceeded)?;
    let mut ends = vec![0; words.len()];
    matcher
        .scan(words, |_, range| {
            ends[range.start] = ends[range.start].max(range.end);
            Ok(())
        })
        .map_err(|_| ClassificationError::CapacityExceeded)?;
    let mut until = 0;
    Ok(ends
        .into_iter()
        .enumerate()
        .map(|(index, end)| {
            until = until.max(end);
            index < until
        })
        .collect())
}
