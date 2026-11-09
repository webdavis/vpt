mod manual;
mod normalize;
pub use manual::{Action, ManualEdit, ManualError};
mod select;
mod state;
mod vocabulary;
pub use state::{Provenance, State, Tag};
pub use vocabulary::Vocabulary;

const MAX_TEXT: usize = 65_536;
const MAX_BYTES: usize = 16 * 1024 * 1024;

pub fn key(tag: &str) -> Result<String, TagError> {
    normalized_key(tag, false)
}
pub fn proposed_key(tag: &str) -> Result<String, TagError> {
    normalized_key(tag, true)
}
fn normalized_key(tag: &str, allow_whitespace: bool) -> Result<String, TagError> {
    let normalized = normalize::proposal(tag)?;
    if (!allow_whitespace && tag.chars().any(char::is_whitespace)) || !normalize::valid(&normalized)
    {
        return Err(TagError::InvalidVocabulary);
    }
    Ok(normalize::key(&normalized))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewTags {
    Hold,
    Write,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    pub new_tags: NewTags,
    pub max_per_note: u32,
    pub max_suggested: u32,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            new_tags: NewTags::Hold,
            max_per_note: 5,
            max_suggested: 10,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagError {
    CapacityExceeded,
    InvalidVocabulary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rejection {
    Syntax,
    ConfirmedLimit,
    SuggestedLimit,
}

impl Rejection {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "syntax" => Some(Self::Syntax),
            "confirmed_limit" => Some(Self::ConfirmedLimit),
            "suggested_limit" => Some(Self::SuggestedLimit),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Syntax => "syntax",
            Self::ConfirmedLimit => "confirmed_limit",
            Self::SuggestedLimit => "suggested_limit",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rejected {
    pub index: usize,
    pub reason: Rejection,
}

#[derive(Clone, Default, PartialEq, Eq)]
pub struct Selection {
    pub confirmed: Vec<String>,
    pub suggested: Vec<String>,
    pub learn: Vec<String>,
    pub rejected: Vec<Rejected>,
}

impl std::fmt::Debug for Selection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Selection")
            .field("confirmed", &self.confirmed.len())
            .field("suggested", &self.suggested.len())
            .field("learn", &self.learn.len())
            .field("rejected", &self.rejected)
            .finish()
    }
}

#[cfg(test)]
mod tests;
