use serde::{Deserialize, Serialize};
mod debug;

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ProposalDocument {
    pub schema: String,
    pub by: String,
    pub summary: Vec<Claim>,
    pub actions: Vec<Action>,
    pub tags: Vec<Tag>,
    pub relations: Vec<Relation>,
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Claim {
    pub text: String,
    pub sources: Vec<String>,
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Action {
    pub text: String,
    pub sources: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<Owner>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due: Option<Due>,
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Owner {
    Me,
    Other(Option<String>),
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Due {
    pub date: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
    #[serde(default)]
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
