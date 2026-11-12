mod checked;
mod debug;
pub use checked::{CheckedAction, CheckedClaim, CheckedProposal};

#[derive(Clone, PartialEq, Eq)]
pub struct Proposal {
    pub by: String,
    pub summary: Vec<Claim>,
    pub actions: Vec<Action>,
    pub tags: Vec<Tag>,
    pub relations: Vec<Related>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Claim {
    pub text: String,
    pub source_references: Vec<String>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Action {
    pub claim: Claim,
    pub owner: Option<Owner>,
}

#[derive(Clone, PartialEq, Eq)]
pub enum Owner {
    Me,
    Other(Option<String>),
}

impl From<Claim> for Action {
    fn from(claim: Claim) -> Self {
        Self { claim, owner: None }
    }
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
