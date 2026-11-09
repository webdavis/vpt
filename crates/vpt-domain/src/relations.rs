mod ambiguity;
pub use ambiguity::MentionAmbiguity;
mod state;
pub use state::{Provenance, Relation, Rule, State, Target};
mod continues;
pub use continues::{Capture, continues};
mod mentions;
pub use mentions::{AmbiguousMention, ConfirmedTerm, Lookup, Mention, Mentions, mentions};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Continues,
    Mentions,
    Related,
    Supersedes,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Continues => "continues",
            Self::Mentions => "mentions",
            Self::Related => "related",
            Self::Supersedes => "supersedes",
        }
    }
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "continues" => Some(Self::Continues),
            "mentions" => Some(Self::Mentions),
            "related" => Some(Self::Related),
            "supersedes" => Some(Self::Supersedes),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    pub session_gap_minutes: u32,
    pub max_suggested: u32,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            session_gap_minutes: 60,
            max_suggested: 5,
        }
    }
}
