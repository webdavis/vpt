use super::LedgerError;
use vpt_domain::{identity::RecordingId, tags::Tag, time::UtcInstant};

pub trait TagLedger {
    fn tags(&self, recording: &RecordingId) -> Result<Option<Vec<Tag>>, LedgerError>;
    fn tag_renames(&self) -> Result<Vec<TagRename>, LedgerError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagChange {
    pub recording: RecordingId,
    pub expected: Option<Vec<Tag>>,
    pub tags: Vec<Tag>,
    pub at: UtcInstant,
}

#[derive(Clone, PartialEq, Eq)]
pub struct TagRename {
    pub old: String,
    pub new: String,
    pub at: UtcInstant,
}

impl std::fmt::Debug for TagRename {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TagRename")
            .field("at", &self.at)
            .finish()
    }
}
