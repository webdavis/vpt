use crate::{alignment::AlignmentError, transcript::Transcript};
use std::ops::Range;

mod classify;
mod comparison;
mod known;
mod risk;
mod stream;

pub use classify::classify;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Class {
    Formatting,
    Numeric,
    ProperNoun,
    LowConfidence,
    AgreedUnverified,
    Other,
}

impl Class {
    pub fn surfaced(self) -> bool {
        self != Self::Formatting
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Formatting => "formatting",
            Self::Numeric => "numeric",
            Self::ProperNoun => "proper-noun",
            Self::LowConfidence => "low-confidence",
            Self::AgreedUnverified => "agreed-unverified",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Diagnostic {
    SingleEngine,
    LanguageMismatch,
    Divergent,
}

impl Diagnostic {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SingleEngine => "single-engine",
            Self::LanguageMismatch => "language-mismatch",
            Self::Divergent => "divergent",
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct LexicalFlag {
    /// Global word offsets in the accepted stream; an insertion has an empty range.
    pub ranges: Vec<Range<usize>>,
    pub class: Class,
    pub record_text: String,
    pub alternative_text: Option<String>,
    pub confidence: Option<f64>,
}

impl std::fmt::Debug for LexicalFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LexicalFlag")
            .field("class", &self.class)
            .field("ranges", &self.ranges)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Classification {
    pub lexical: Vec<LexicalFlag>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Options {
    pub confidence_floor: f64,
    pub max_divergence_ratio: f64,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            confidence_floor: 0.35,
            max_divergence_ratio: 0.35,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassificationError {
    InvalidOptions,
    InvalidConfidence,
    CapacityExceeded,
    Alignment(AlignmentError),
}

#[cfg(test)]
mod tests;
