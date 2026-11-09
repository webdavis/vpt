use super::*;
use vpt_domain::relations::{self, ConfirmedTerm, Lookup, Mentions};

impl Index {
    /// Each returned mention target is an index into `self.notes()`.
    pub fn mentions(&self, words: &[&str], terms: &[String]) -> Result<Mentions, StoreError> {
        if terms.len() > 65_536 {
            return Err(capacity());
        }
        let terms = terms
            .iter()
            .map(|term| {
                let lookup = match self.matching(term) {
                    [] => Lookup::Missing,
                    [path] => Lookup::Unique(
                        self.notes
                            .binary_search_by(|note| note.path.cmp(path))
                            .map_err(|_| StoreError::Io("note index is inconsistent".into()))?,
                    ),
                    _ => Lookup::Ambiguous,
                };
                Ok(ConfirmedTerm {
                    text: term.as_str(),
                    lookup,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        relations::mentions(words, &terms).map_err(|_| capacity())
    }
}

#[cfg(test)]
mod tests;
