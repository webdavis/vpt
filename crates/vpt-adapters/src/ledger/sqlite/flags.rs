mod read;
pub(super) mod resolve;
mod write;
use super::SqliteLedger;
use vpt_application::ports::{FlagLedger, LedgerError, ReviewLedger, ReviewSnapshot};
use vpt_domain::{identity::RecordingId, review::ReviewFlag};

pub(super) use read::{all, read};
pub(super) use write::replace;

impl FlagLedger for SqliteLedger {
    fn transcript_flags(&self, id: &RecordingId) -> Result<Vec<ReviewFlag>, LedgerError> {
        self.snapshot(|connection| Ok(super::transcript::read_with_flags(connection, id)?.flags))
    }
}

impl ReviewLedger for SqliteLedger {
    fn review_snapshot(&self, id: &RecordingId) -> Result<Option<ReviewSnapshot>, LedgerError> {
        self.snapshot(|connection| {
            let exists: bool = connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM recordings WHERE id=?1)",
                    [id.as_str()],
                    |row| row.get(0),
                )
                .map_err(super::map)?;
            if !exists {
                return Ok(None);
            }
            let review = super::verification::state::review(connection, id)?;
            Ok(Some(review))
        })
    }
}

#[cfg(test)]
pub(super) mod tests;
