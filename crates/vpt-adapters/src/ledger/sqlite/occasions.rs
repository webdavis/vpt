pub(super) mod read;
mod write;
use super::SqliteLedger;
use vpt_application::ports::{LedgerError, OccasionLedger, OccasionRecord};
use vpt_domain::occasion::{OccasionId, OccasionKey};
pub(super) use write::apply;
impl OccasionLedger for SqliteLedger {
    fn occasion(&self, id: &OccasionId) -> Result<Option<OccasionRecord>, LedgerError> {
        self.snapshot(|connection| read::one(connection, "id = ?1", id.as_str()))
    }
    fn occasion_by_key(&self, key: &OccasionKey) -> Result<Option<OccasionRecord>, LedgerError> {
        self.snapshot(|connection| read::one(connection, "provider_key = ?1", &key.encoded()))
    }
    fn occasions(&self) -> Result<Vec<OccasionRecord>, LedgerError> {
        self.snapshot(read::all)
    }
}
#[cfg(test)]
pub(super) mod tests;

#[cfg(test)]
mod durability;

#[cfg(test)]
mod corrupt;
