use super::{MAX_BYTES, TagError, normalize};
use std::collections::HashMap;

pub struct Vocabulary {
    pub(super) spellings: HashMap<String, String>,
}

impl Vocabulary {
    pub fn new(tags: &[String]) -> Result<Self, TagError> {
        normalize::inputs(tags, 65_536)?;
        let mut spellings = HashMap::new();
        let mut remaining = MAX_BYTES;
        for tag in tags {
            let key = super::key(tag)?;
            if let std::collections::hash_map::Entry::Vacant(entry) = spellings.entry(key) {
                remaining = remaining
                    .checked_sub(entry.key().len() + tag.len())
                    .ok_or(TagError::CapacityExceeded)?;
                entry.insert(tag.clone());
            }
        }
        Ok(Self { spellings })
    }
}

impl std::fmt::Debug for Vocabulary {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Vocabulary")
            .field("entries", &self.spellings.len())
            .finish()
    }
}
