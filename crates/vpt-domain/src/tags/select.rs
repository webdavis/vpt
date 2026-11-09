use super::{
    MAX_BYTES, NewTags, Options, Rejected, Rejection, Selection, TagError, Vocabulary, normalize,
};
use std::collections::HashSet;

impl Vocabulary {
    pub fn select(&self, proposals: &[String], options: Options) -> Result<Selection, TagError> {
        normalize::inputs(proposals, 256)?;
        let mut selection = Selection::default();
        let mut seen = HashSet::new();
        let mut remaining = MAX_BYTES;
        for (index, proposal) in proposals.iter().enumerate() {
            let normalized = normalize::proposal(proposal)?;
            if !normalize::valid(&normalized) {
                selection.rejected.push(Rejected {
                    index,
                    reason: Rejection::Syntax,
                });
                continue;
            }
            let key = normalize::key(&normalized);
            if !seen.insert(key.clone()) {
                continue;
            }
            remaining = remaining
                .checked_sub(key.len())
                .ok_or(TagError::CapacityExceeded)?;
            let known = self.spellings.get(&key);
            let confirmed = known.is_some() || options.new_tags == NewTags::Write;
            let (target, limit, reason) = if confirmed {
                (
                    &mut selection.confirmed,
                    options.max_per_note,
                    Rejection::ConfirmedLimit,
                )
            } else {
                (
                    &mut selection.suggested,
                    options.max_suggested,
                    Rejection::SuggestedLimit,
                )
            };
            if target.len() as u64 >= u64::from(limit) {
                selection.rejected.push(Rejected { index, reason });
                continue;
            }
            let value = known.unwrap_or(&normalized);
            let learn = confirmed && known.is_none();
            remaining = remaining
                .checked_sub(value.len() * if learn { 2 } else { 1 })
                .ok_or(TagError::CapacityExceeded)?;
            target.push(value.clone());
            if learn {
                selection.learn.push(value.clone());
            }
        }
        Ok(selection)
    }
}
