use super::LedgerError;
use vpt_domain::{
    identity::RecordingId,
    tags::{State, Tag},
    time::UtcInstant,
};

pub trait TagLedger {
    fn tags(&self, recording: &RecordingId) -> Result<Option<Vec<Tag>>, LedgerError>;
    fn tag_renames(&self) -> Result<Vec<TagRename>, LedgerError>;
    fn tag_states(&self) -> Result<Vec<(String, State)>, LedgerError>;
    fn tag_names(&self) -> Result<Vec<(RecordingId, String)>, LedgerError>;
    fn learned_tags(&self) -> Result<Vec<String>, LedgerError>;
}

/// Keys of the tags synthesis appended to the known-tags file, and of those that left it.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct LearnedTagChange {
    pub learned: Vec<String>,
    pub forgotten: Vec<String>,
}

impl std::fmt::Debug for LearnedTagChange {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LearnedTagChange")
            .field("learned", &self.learned.len())
            .field("forgotten", &self.forgotten.len())
            .finish()
    }
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
