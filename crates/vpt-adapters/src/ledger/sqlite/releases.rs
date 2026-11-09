use super::*;
use vpt_application::ports::*;
use vpt_domain::identity::RecordingId;
pub(super) mod read;
pub(super) fn owns(c: &rusqlite::Connection, path: &std::path::Path) -> Result<bool, LedgerError> {
    c.query_row(
        "SELECT EXISTS(SELECT 1 FROM releases WHERE destination=?1 OR report_path=?1)",
        [path.to_str().ok_or_else(crate::ledger::release::invalid)?],
        |r| r.get(0),
    )
    .map_err(map)
}
impl ReleaseLedger for SqliteLedger {
    fn release(&self, sequence: i64) -> Result<Option<ReleaseRecord>, LedgerError> {
        self.snapshot(|c| read::record(c, sequence))
    }
    fn release_payload(&self, sequence: i64) -> Result<Option<ReleasePayload>, LedgerError> {
        self.snapshot(|c| read::payload(c, sequence))
    }
    fn releases(&self, recording: &RecordingId) -> Result<Vec<i64>, LedgerError> {
        self.snapshot(|c| read::sequences(c, Some(recording)))
    }
    fn pending_releases(&self) -> Result<Vec<i64>, LedgerError> {
        self.snapshot(|c| read::sequences(c, None))
    }
}
#[cfg(test)]
pub(super) mod tests;
