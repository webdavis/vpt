mod checked;
mod debug;
pub use checked::{CheckedClaim, CheckedProposal};

#[derive(Clone, PartialEq, Eq)]
pub struct Proposal {
    pub by: String,
    pub summary: Vec<Claim>,
    pub actions: Vec<Claim>,
    pub tags: Vec<Tag>,
    pub relations: Vec<Related>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Claim {
    pub text: String,
    pub source_references: Vec<String>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Tag {
    pub tag: String,
    pub reason: String,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Related {
    pub target: String,
    pub reason: String,
}
