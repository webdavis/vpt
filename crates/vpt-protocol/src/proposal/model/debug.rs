use super::*;
use std::fmt;

impl fmt::Debug for ProposalDocument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProposalDocument")
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
            .field("sources", &self.sources.len())
            .finish()
    }
}
impl fmt::Debug for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Action")
            .field("sources", &self.sources.len())
            .field("owner", &self.owner.is_some())
            .field("due", &self.due.as_ref().map(|due| due.sources.len()))
            .finish()
    }
}
impl fmt::Debug for Owner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Owner")
    }
}
impl fmt::Debug for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Tag")
    }
}
impl fmt::Debug for Relation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Relation")
    }
}
