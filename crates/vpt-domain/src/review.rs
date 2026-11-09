mod verification;
pub use verification::VerificationFlag;
mod resolution;
use crate::{
    digest::Sha256Digest,
    time::UtcInstant,
    uncertainty::{Diagnostic, LexicalFlag},
};
pub use resolution::{ResolutionKind, ResolveError, ReviewAction, valid_correction};

#[derive(Clone, Debug, PartialEq)]
pub enum FlagContent {
    Lexical(LexicalFlag),
    Diagnostic(Diagnostic),
    Verification(VerificationFlag),
}

#[derive(Clone, PartialEq, Eq)]
pub enum Resolution {
    Open,
    Confirmed { at: UtcInstant },
    Corrected { text: String, at: UtcInstant },
    Dismissed { at: UtcInstant },
}

impl std::fmt::Debug for Resolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Open => "Open",
            Self::Confirmed { .. } => "Confirmed",
            Self::Corrected { .. } => "Corrected",
            Self::Dismissed { .. } => "Dismissed",
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReviewFlag {
    pub identifier: Sha256Digest,
    pub content: FlagContent,
    pub resolution: Resolution,
}
