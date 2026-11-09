use super::LedgerError;
use vpt_domain::{identity::RecordingId, relations::Relation, time::UtcInstant};

#[derive(Clone, PartialEq, Eq)]
pub struct NoteRelation {
    pub relation: Relation,
    pub path: Option<std::path::PathBuf>,
}

pub struct MentionDerivation {
    pub links: Vec<NoteRelation>,
    pub ambiguities: Vec<vpt_domain::relations::MentionAmbiguity>,
}

#[derive(Clone)]
pub struct NotePredecessor {
    pub record: super::RecordingRecord,
    pub path: Option<std::path::PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AmbiguityChange {
    pub expected: Vec<vpt_domain::relations::MentionAmbiguity>,
    pub ambiguities: Vec<vpt_domain::relations::MentionAmbiguity>,
}

pub trait RelationLedger {
    fn mention_ambiguities(
        &self,
        recording: &RecordingId,
    ) -> Result<Vec<vpt_domain::relations::MentionAmbiguity>, LedgerError>;
    /// Returns relations ordered by kind and target, with empty state for unknown recordings.
    fn relations(&self, recording: &RecordingId) -> Result<Vec<Relation>, LedgerError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationChange {
    pub ambiguities: Option<AmbiguityChange>,
    pub recording: RecordingId,
    pub expected: Vec<Relation>,
    pub relations: Vec<Relation>,
    pub at: UtcInstant,
}
