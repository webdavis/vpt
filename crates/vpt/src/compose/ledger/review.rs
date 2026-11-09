use super::RuntimeLedger;
use vpt_application::ports::{LedgerError, ReviewLedger, ReviewSnapshot};
use vpt_domain::identity::RecordingId;

impl ReviewLedger for RuntimeLedger {
    fn review_snapshot(&self, id: &RecordingId) -> Result<Option<ReviewSnapshot>, LedgerError> {
        match self {
            Self::Sqlite(ledger) => ledger.review_snapshot(id),
            Self::Empty(ledger) => ledger.review_snapshot(id),
        }
    }
}
