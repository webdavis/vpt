mod context;
pub mod document;
pub mod markdown;
pub use context::{ContextStatus, ContextSummary, ContextTask};
mod evidence;
mod flags;
mod quotes;
pub use evidence::{Certainty, EvidencePack, Quote, QuoteSource, SourceDigest, Unresolved};
mod direct;
mod input;
mod output;
mod recent;
mod selection;
mod terms;
mod validate;
pub use input::{Candidate, Options, Request, Term, TermTarget, TranscriptSource};
pub use output::{Decision, Match, ParticipantMatch, Rejection, Selection, Selector, TermState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BriefError {
    InvalidInput,
    ConflictingRecording,
    CapacityExceeded,
}

pub fn select(request: &Request<'_>) -> Result<Selection, BriefError> {
    selection::select(request)
}

#[cfg(test)]
mod tests;

pub fn assemble(request: &Request<'_>) -> Result<EvidencePack, BriefError> {
    quotes::assemble(request)
}

/// Limits retained candidate text and collection entries across a streamed ledger snapshot.
pub struct InputBudget {
    bytes: usize,
    items: usize,
    candidates: usize,
}
impl Default for InputBudget {
    fn default() -> Self {
        Self {
            bytes: 64 * 1024 * 1024,
            items: 1_048_576,
            candidates: 65_536,
        }
    }
}
impl InputBudget {
    pub fn candidate(&mut self, candidate: &Candidate) -> Result<(), BriefError> {
        self.candidates = self
            .candidates
            .checked_sub(1)
            .ok_or(BriefError::CapacityExceeded)?;
        validate::candidate(candidate, &mut self.bytes, &mut self.items)
    }
}
