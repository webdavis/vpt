mod inputs;
use super::SqliteLedger;
mod read;
mod write;
use std::path::Path;
use vpt_application::ports::{BriefArtifact, BriefLedger, LedgerError};
use vpt_domain::occasion::OccasionId;
pub(super) use write::apply;

impl BriefLedger for SqliteLedger {
    fn brief(&self, id: &OccasionId) -> Result<Option<BriefArtifact>, LedgerError> {
        self.snapshot(|connection| read::one(connection, "occasion", id.as_str()))
    }
    fn brief_at(&self, target: &Path) -> Result<Option<BriefArtifact>, LedgerError> {
        self.snapshot(|connection| {
            read::one(
                connection,
                "target",
                target.to_str().ok_or_else(crate::ledger::brief::invalid)?,
            )
        })
    }
}

#[cfg(test)]
pub(super) mod tests;

pub(super) fn owns(
    connection: &rusqlite::Connection,
    target: &std::path::Path,
) -> Result<bool, LedgerError> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM owned_briefs WHERE target=?1)",
            [target.to_str().ok_or_else(crate::ledger::brief::invalid)?],
            |row| row.get(0),
        )
        .map_err(super::map)
}
