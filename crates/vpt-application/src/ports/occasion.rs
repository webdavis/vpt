use super::LedgerError;
use std::path::PathBuf;
use vpt_domain::{
    occasion::{OccasionDetails, OccasionId, OccasionKey},
    time::UtcInstant,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OccasionRecord {
    pub id: OccasionId,
    pub key: OccasionKey,
    pub details: OccasionDetails,
    pub brief_path: Option<PathBuf>,
    pub briefed_at: Option<UtcInstant>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OccasionChange {
    pub expected: Option<OccasionRecord>,
    pub occasion: OccasionRecord,
    pub at: UtcInstant,
}

pub trait OccasionLedger {
    fn occasion(&self, id: &OccasionId) -> Result<Option<OccasionRecord>, LedgerError>;
    fn occasion_by_key(&self, key: &OccasionKey) -> Result<Option<OccasionRecord>, LedgerError>;
    fn occasions(&self) -> Result<Vec<OccasionRecord>, LedgerError>;
}
