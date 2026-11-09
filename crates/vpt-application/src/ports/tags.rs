use super::LedgerError;
use vpt_domain::{identity::RecordingId, tags::Tag, time::UtcInstant};

pub trait TagLedger {
    fn tags(&self, recording: &RecordingId) -> Result<Option<Vec<Tag>>, LedgerError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagChange {
    pub recording: RecordingId,
    pub expected: Option<Vec<Tag>>,
    pub tags: Vec<Tag>,
    pub at: UtcInstant,
}
