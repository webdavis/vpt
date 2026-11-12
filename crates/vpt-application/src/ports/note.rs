use super::StoreError;
use std::{fmt, path::Path};
use vpt_domain::digest::Sha256Digest;

#[derive(Clone, PartialEq, Eq)]
pub struct StoredNote {
    pub text: String,
    pub digest: Sha256Digest,
}

impl fmt::Debug for StoredNote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StoredNote")
            .field("bytes", &self.text.len())
            .field("digest", &self.digest)
            .finish()
    }
}

pub trait NoteFiles {
    fn note_entries(&self, root: &Path) -> Result<super::RetentionEntries, StoreError>;
    fn read_note(&self, path: &Path) -> Result<Option<StoredNote>, StoreError>;
}

pub struct NoteInputs {
    pub proposal: Option<super::AcceptedProposal>,
    pub derived_mentions: Option<super::MentionDerivation>,
    pub mention_ambiguities: Vec<vpt_domain::relations::MentionAmbiguity>,
    pub relations: Vec<super::NoteRelation>,
    pub tags: Option<Vec<vpt_domain::tags::Tag>>,
    pub record: super::RecordingRecord,
    pub predecessor: Option<super::NotePredecessor>,
    pub review: super::ReviewSnapshot,
    pub notes: Vec<super::NoteArtifact>,
}

impl NoteInputs {
    pub fn has_renderable_notes(&self) -> bool {
        self.review.accepted.is_some()
            && (self.record.stages.note != super::StageState::Expired
                || (self.record.stages.analysis != super::StageState::Expired
                    && self
                        .notes
                        .iter()
                        .any(|note| note.stage == super::NoteStage::Analysis)))
    }
}

pub trait NoteInputsLedger {
    fn note_inputs(
        &self,
        id: &vpt_domain::identity::RecordingId,
    ) -> Result<Option<NoteInputs>, super::LedgerError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteFormatError {
    CapacityExceeded,
    Frontmatter,
    Markers,
    Identity,
    Stage,
    UnsupportedSchema(u32),
}

pub trait NoteFormat {
    fn claims_digest(
        &self,
        content: &str,
        stage: super::NoteStage,
        flags: &[vpt_domain::review::ReviewFlag],
    ) -> Result<Sha256Digest, NoteFormatError>;
    fn content(&self, source: &str) -> Result<String, NoteFormatError>;
    fn clear_findings(&self, content: &str) -> Result<String, NoteFormatError>;
    fn render_findings(
        &self,
        content: &str,
        stage: super::NoteStage,
        flags: &[vpt_domain::review::ReviewFlag],
    ) -> Result<String, NoteFormatError>;

    fn recording(
        &self,
        source: &str,
    ) -> Result<Option<vpt_domain::identity::RecordingId>, NoteFormatError>;
    fn rewrite_owned(
        &self,
        source: &str,
        replacement: &str,
        id: &vpt_domain::identity::RecordingId,
        stage: super::NoteStage,
        previous_tags: &[String],
    ) -> Result<String, NoteFormatError>;
}
