use super::{Selection, Selector};
use crate::{digest::Sha256Digest, identity::RecordingId};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Certainty {
    Confirmed,
    Unverified,
}
impl Certainty {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Confirmed => "confirmed",
            Self::Unverified => "unverified",
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct QuoteSource {
    pub recording: RecordingId,
    pub at_secs: f64,
    pub note: PathBuf,
    pub digest: Sha256Digest,
}
#[derive(Clone, PartialEq)]
pub struct Quote {
    pub text: String,
    pub certainty: Certainty,
    pub source: QuoteSource,
    pub selector: Selector,
}
impl std::fmt::Debug for Quote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Quote")
            .field("recording", &self.source.recording)
            .field("certainty", &self.certainty)
            .field("selector", &self.selector)
            .finish_non_exhaustive()
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceDigest {
    pub recording: RecordingId,
    pub note: PathBuf,
    pub digest: Sha256Digest,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Unresolved {
    pub flags: usize,
    pub notes_with_flags: usize,
    pub reviewed: bool,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EvidencePack {
    pub selection: Selection,
    pub items: Vec<Quote>,
    pub sources: Vec<SourceDigest>,
    pub omitted_spans: usize,
    pub unresolved: Unresolved,
}
