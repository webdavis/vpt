use super::Kind;
use crate::{digest::Sha256Digest, identity::RecordingId};
use std::ops::Range;

#[derive(Clone, PartialEq, Eq)]
pub enum Target {
    Recording(RecordingId),
    Note(String),
}
impl Target {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Recording(id) => id.as_str(),
            Self::Note(path) => path,
        }
    }
}
impl std::fmt::Debug for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Recording(_) => "Recording",
            Self::Note(_) => "Note",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Confirmed,
    Suggested,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rule {
    ImmediatePredecessor,
    KnownTerm,
}

#[derive(Clone, PartialEq, Eq)]
pub enum Provenance {
    Manual,
    Derived(Rule),
    Proposal {
        by: String,
        reason: String,
        input: Sha256Digest,
    },
}
impl std::fmt::Debug for Provenance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Manual => f.write_str("Manual"),
            Self::Derived(rule) => f.debug_tuple("Derived").field(rule).finish(),
            Self::Proposal { .. } => f.write_str("Proposal"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relation {
    pub kind: Kind,
    pub target: Target,
    pub state: State,
    pub provenance: Provenance,
    /// Half-open positions in the immutable accepted word stream.
    pub ranges: Vec<Range<usize>>,
}
