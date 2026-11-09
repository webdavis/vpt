use super::RuntimeLedger;
use vpt_application::ports::{LedgerError, OccasionLedger, OccasionRecord};
use vpt_domain::occasion::{OccasionId, OccasionKey};
impl RuntimeLedger {
    fn occasion_port(&self) -> &dyn OccasionLedger {
        match self {
            Self::Sqlite(ledger) => ledger,
            Self::Empty(ledger) => ledger,
        }
    }
}
impl OccasionLedger for RuntimeLedger {
    fn occasion(&self, id: &OccasionId) -> Result<Option<OccasionRecord>, LedgerError> {
        self.occasion_port().occasion(id)
    }
    fn occasion_by_key(&self, key: &OccasionKey) -> Result<Option<OccasionRecord>, LedgerError> {
        self.occasion_port().occasion_by_key(key)
    }
    fn occasions(&self) -> Result<Vec<OccasionRecord>, LedgerError> {
        self.occasion_port().occasions()
    }
}
