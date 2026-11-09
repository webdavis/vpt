use serde::{Deserialize, Serialize};
mod debug;

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ProposalDocument {
    pub schema: String,
    pub by: String,
    pub summary: Vec<Claim>,
    pub actions: Vec<Claim>,
    pub tags: Vec<Tag>,
    pub relations: Vec<Relation>,
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Claim {
    pub text: String,
    pub sources: Vec<String>,
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Tag {
    pub tag: String,
    pub reason: String,
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Relation {
    pub kind: String,
    pub target: String,
    pub reason: String,
}
