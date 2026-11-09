use super::SqliteLedger;
use vpt_application::ports::{LedgerError, RelationLedger};
use vpt_domain::{identity::RecordingId, relations::Relation};
pub(super) mod ambiguities;
mod read;
mod write;
pub(super) use read::recording as read;
pub(super) use write::apply;
impl RelationLedger for SqliteLedger {
    fn mention_ambiguities(
        &self,
        id: &RecordingId,
    ) -> Result<Vec<vpt_domain::relations::MentionAmbiguity>, LedgerError> {
        self.snapshot(|connection| ambiguities::read(connection, id))
    }
    fn relations(&self, id: &RecordingId) -> Result<Vec<Relation>, LedgerError> {
        self.snapshot(|connection| read(connection, id))
    }
}
#[cfg(test)]
pub(crate) mod tests;
