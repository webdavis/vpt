use crate::{digest::Sha256Digest, time::UtcInstant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Confirmed,
    Suggested,
    Rejected,
}

#[derive(Clone, PartialEq, Eq)]
pub enum Provenance {
    Manual,
    Proposal {
        by: String,
        reason: String,
        input: Sha256Digest,
    },
}

impl std::fmt::Debug for Provenance {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Manual => "Manual",
            Self::Proposal { .. } => "Proposal",
        })
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Tag {
    pub name: String,
    pub state: State,
    pub provenance: Provenance,
    pub updated_at: Option<UtcInstant>,
}

impl std::fmt::Debug for Tag {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Tag")
            .field("state", &self.state)
            .field("provenance", &self.provenance)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}
