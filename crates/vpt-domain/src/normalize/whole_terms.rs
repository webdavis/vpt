use std::collections::BTreeMap;
use std::ops::Range;
mod bounded;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchError {
    CapacityExceeded,
}

#[derive(Default)]
struct Node {
    children: BTreeMap<String, usize>,
    terminal: Option<usize>,
}

pub struct TermMatcher {
    nodes: Vec<Node>,
}

impl std::fmt::Debug for TermMatcher {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TermMatcher")
            .field("nodes", &self.nodes.len())
            .finish()
    }
}

impl TermMatcher {
    pub fn new(terms: &[&str]) -> Result<Self, MatchError> {
        let mut nodes = vec![Node::default()];
        for (index, term) in bounded::normalize(terms)?.iter().enumerate() {
            let mut at = 0;
            for token in term.split_whitespace() {
                let next = match nodes[at].children.get(token) {
                    Some(next) => *next,
                    None => {
                        if nodes.len() == 65_537 {
                            return Err(MatchError::CapacityExceeded);
                        }
                        let next = nodes.len();
                        nodes.push(Node::default());
                        nodes[at].children.insert(token.into(), next);
                        next
                    }
                };
                at = next;
            }
            if at != 0 {
                nodes[at].terminal.get_or_insert(index);
            }
        }
        Ok(Self { nodes })
    }
    pub fn scan(
        &self,
        words: &[&str],
        visit: impl FnMut(usize, Range<usize>) -> Result<(), MatchError>,
    ) -> Result<(), MatchError> {
        self.scan_with(words, visit, 8_388_608)
    }
    fn scan_with(
        &self,
        words: &[&str],
        visit: impl FnMut(usize, Range<usize>) -> Result<(), MatchError>,
        mut remaining: usize,
    ) -> Result<(), MatchError> {
        self.scan_budgeted(words, visit, &mut remaining)
    }
    pub(crate) fn scan_budgeted(
        &self,
        words: &[&str],
        mut visit: impl FnMut(usize, Range<usize>) -> Result<(), MatchError>,
        remaining: &mut usize,
    ) -> Result<(), MatchError> {
        let words = bounded::normalize(words)?;
        for start in 0..words.len() {
            let mut at = 0;
            for (index, word) in words.iter().enumerate().skip(start) {
                *remaining = remaining
                    .checked_sub(1)
                    .ok_or(MatchError::CapacityExceeded)?;
                let Some(next) = self.nodes[at].children.get(word) else {
                    break;
                };
                at = *next;
                if let Some(term) = self.nodes[at].terminal {
                    visit(term, start..index + 1)?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
