use crate::normalize::{MatchError, TermMatcher};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lookup {
    Missing,
    Unique(usize),
    Ambiguous,
}

pub struct ConfirmedTerm<'a> {
    pub text: &'a str,
    pub lookup: Lookup,
}
impl std::fmt::Debug for ConfirmedTerm<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfirmedTerm")
            .field("lookup", &self.lookup)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mention {
    pub target: usize,
    pub ranges: Vec<Range<usize>>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AmbiguousMention {
    pub term: usize,
    pub ranges: Vec<Range<usize>>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Mentions {
    pub links: Vec<Mention>,
    pub ambiguous: Vec<AmbiguousMention>,
}

pub fn mentions(words: &[&str], terms: &[ConfirmedTerm<'_>]) -> Result<Mentions, MatchError> {
    if terms.len() > 65_536 {
        return Err(MatchError::CapacityExceeded);
    }
    let names: Vec<_> = terms.iter().map(|term| term.text).collect();
    let matcher = TermMatcher::new(&names)?;
    let mut links: BTreeMap<usize, BTreeSet<(usize, usize)>> = BTreeMap::new();
    let mut ambiguous: BTreeMap<usize, BTreeSet<(usize, usize)>> = BTreeMap::new();
    let mut remaining = 65_536usize;
    matcher.scan(words, |term, range| {
        let ranges = match terms[term].lookup {
            Lookup::Missing => return Ok(()),
            Lookup::Unique(target) => links.entry(target).or_default(),
            Lookup::Ambiguous => ambiguous.entry(term).or_default(),
        };
        let range = (range.start, range.end);
        if !ranges.contains(&range) {
            remaining = remaining
                .checked_sub(1)
                .ok_or(MatchError::CapacityExceeded)?;
            ranges.insert(range);
        }
        Ok(())
    })?;
    let ranges = |items: BTreeSet<(usize, usize)>| {
        items.into_iter().map(|(start, end)| start..end).collect()
    };
    Ok(Mentions {
        links: links
            .into_iter()
            .map(|(target, items)| Mention {
                target,
                ranges: ranges(items),
            })
            .collect(),
        ambiguous: ambiguous
            .into_iter()
            .map(|(term, items)| AmbiguousMention {
                term,
                ranges: ranges(items),
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests;
