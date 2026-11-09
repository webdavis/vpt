use super::{LedgerError, NoteArtifact, RecordingRecord, ReviewSnapshot};
use vpt_domain::{relations::Relation, tags::Tag};

pub struct BriefInput {
    pub recording: RecordingRecord,
    pub note: Option<NoteArtifact>,
    pub review: ReviewSnapshot,
    pub tags: Vec<Tag>,
    pub relations: Vec<Relation>,
}

pub trait BriefInputsLedger {
    /// Streams at most 65,536 recording inputs from one consistent ledger snapshot.
    fn visit_brief_inputs(
        &self,
        visitor: &mut dyn FnMut(BriefInput) -> Result<(), LedgerError>,
    ) -> Result<(), LedgerError>;
}
