use crate::{
    digest::Sha256Digest, identity::RecordingId, occasion::OccasionDetails, relations::Relation,
    review::ReviewFlag, tags::Tag, time::UtcInstant, transcript::Transcript,
};
use std::path::PathBuf;

#[derive(Clone, PartialEq, Eq)]
pub enum TermTarget {
    Missing,
    Ambiguous,
    Unique(PathBuf),
}
#[derive(Clone, PartialEq, Eq)]
pub struct Term {
    pub text: String,
    pub target: TermTarget,
}

#[derive(Clone, PartialEq)]
pub struct TranscriptSource {
    pub note: PathBuf,
    pub digest: Sha256Digest,
    pub transcript: Transcript,
    pub flags: Vec<ReviewFlag>,
    pub reviewed: bool,
}
#[derive(Clone, PartialEq)]
pub struct Candidate {
    pub recording: RecordingId,
    pub captured_at: UtcInstant,
    /// Present only when the caller has validated an eligible transcript artifact snapshot.
    pub source: Option<TranscriptSource>,
    pub tags: Vec<Tag>,
    pub relations: Vec<Relation>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    pub max_notes: usize,
    pub lookback_days: u32,
    pub max_spans_per_note: usize,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            max_notes: 12,
            lookback_days: 180,
            max_spans_per_note: 5,
        }
    }
}

pub struct Request<'a> {
    pub occasion: &'a OccasionDetails,
    /// Confirmed vocabulary with resolution against the current note index.
    pub terms: &'a [Term],
    pub candidates: &'a [Candidate],
    pub now: UtcInstant,
    pub options: Options,
}

impl std::fmt::Debug for Candidate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Candidate")
            .field("recording", &self.recording)
            .finish_non_exhaustive()
    }
}
impl std::fmt::Debug for Term {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Term").finish_non_exhaustive()
    }
}
