use super::SqliteLedger;
use vpt_application::ports::{LedgerError, TagLedger};
use vpt_domain::{identity::RecordingId, tags::Tag};
mod read;
mod write;
pub(super) use read::recording as read;
pub(super) use write::apply;

impl TagLedger for SqliteLedger {
    fn tags(&self, id: &RecordingId) -> Result<Option<Vec<Tag>>, LedgerError> {
        self.snapshot(|connection| read(connection, id))
    }
}

#[cfg(test)]
pub(super) mod tests;
