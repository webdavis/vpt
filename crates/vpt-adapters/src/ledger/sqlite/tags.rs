use super::SqliteLedger;
use vpt_application::ports::{LedgerError, TagLedger, TagRename};
use vpt_domain::{
    identity::RecordingId,
    tags::{State, Tag},
};
mod learned;
mod read;
mod write;
pub(super) use learned::apply as apply_learned;
pub(super) use read::recording as read;
pub(super) use write::{apply, apply_rename};

impl TagLedger for SqliteLedger {
    fn tags(&self, id: &RecordingId) -> Result<Option<Vec<Tag>>, LedgerError> {
        self.snapshot(|connection| read(connection, id))
    }

    fn tag_renames(&self) -> Result<Vec<TagRename>, LedgerError> {
        self.snapshot(read::renames)
    }

    fn tag_states(&self) -> Result<Vec<(String, State)>, LedgerError> {
        self.snapshot(read::states)
    }

    fn tag_names(&self) -> Result<Vec<(RecordingId, String)>, LedgerError> {
        self.snapshot(learned::names)
    }

    fn learned_tags(&self) -> Result<Vec<String>, LedgerError> {
        self.snapshot(learned::keys)
    }
}

#[cfg(test)]
pub(super) mod tests;
