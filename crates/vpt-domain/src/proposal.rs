mod action_id;
mod checked;
pub use action_id::action_ids;
mod debug;
pub use checked::{CheckedAction, CheckedClaim, CheckedDue, CheckedProposal};

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
    pub due: Option<Due>,
}

#[derive(Clone, PartialEq, Eq)]
pub enum Owner {
    Me,
    Other(Option<String>),
}

#[derive(Clone, PartialEq, Eq)]
pub struct Due {
    pub date: String,
    pub time: Option<String>,
    pub source_references: Vec<String>,
}

impl From<Claim> for Action {
    fn from(claim: Claim) -> Self {
        Self {
            claim,
            owner: None,
            due: None,
        }
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
