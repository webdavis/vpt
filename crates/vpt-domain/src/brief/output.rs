use crate::identity::RecordingId;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Selector {
    Participant,
    Term,
    Tag,
    Recent,
}
impl Selector {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Participant => "participant",
            Self::Term => "term",
            Self::Tag => "tag",
            Self::Recent => "recent",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TermState {
    Unconfirmed,
    MissingNote,
    AmbiguousNote,
    Confirmed,
}
impl TermState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Confirmed => "confirmed",
            Self::Unconfirmed => "unconfirmed",
            Self::MissingNote => "missing-note",
            Self::AmbiguousNote => "ambiguous-note",
        }
    }
}
#[derive(Clone, PartialEq, Eq)]
pub struct ParticipantMatch {
    pub participant: usize,
    pub term: TermState,
    pub matched_note: Option<PathBuf>,
    pub confirmation: Option<String>,
}
#[derive(Clone, PartialEq, Eq)]
pub struct Match {
    pub selector: Selector,
    pub values: Vec<String>,
    pub anchors: Vec<RecordingId>,
}
impl std::fmt::Debug for Match {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Match")
            .field("selector", &self.selector)
            .finish_non_exhaustive()
    }
}
impl std::fmt::Debug for ParticipantMatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParticipantMatch")
            .field("participant", &self.participant)
            .field("term", &self.term)
            .finish_non_exhaustive()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rejection {
    MissingTranscript,
    NoMatch,
    OutsideLookback,
    FutureCapture,
}
impl Rejection {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MissingTranscript => "missing-transcript",
            Self::NoMatch => "no-confirmed-match",
            Self::OutsideLookback => "outside-lookback",
            Self::FutureCapture => "future-capture",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decision {
    pub recording: RecordingId,
    pub matched: Option<Match>,
    pub rejected: Option<Rejection>,
    pub selected: bool,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    pub participants: Vec<ParticipantMatch>,
    /// Candidate indices in selector rank, capture time descending, then identity order.
    pub selected: Vec<usize>,
    pub decisions: Vec<Decision>,
    pub omitted_notes: usize,
    pub rejected_notes: usize,
}
