use super::*;
use std::fmt;

impl fmt::Debug for Proposal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Proposal")
            .field("summary", &self.summary.len())
            .field("actions", &self.actions.len())
            .field("tags", &self.tags.len())
            .field("relations", &self.relations.len())
            .finish()
    }
}
impl fmt::Debug for Claim {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Claim")
            .field("sources", &self.source_references.len())
            .finish()
    }
}
impl fmt::Debug for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ProposedTag")
    }
}
impl fmt::Debug for Related {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ProposedRelated")
    }
}
