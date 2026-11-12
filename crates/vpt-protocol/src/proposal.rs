mod diagnostics;
mod model;
mod validate;
use crate::LimitViolation;
pub use model::{Action, Claim, Due, Owner, ProposalDocument, Relation, Tag};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProposalDecodeError {
    Limit(LimitViolation),
    UnsupportedVersion(u64),
    Invalid(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedProposal {
    pub document: ProposalDocument,
    pub diagnostics: Vec<String>,
}

pub fn decode(bytes: &[u8]) -> Result<DecodedProposal, ProposalDecodeError> {
    let value = crate::limits::read_proposal(bytes).map_err(ProposalDecodeError::Limit)?;
    let schema = value
        .get("schema")
        .and_then(serde_json::Value::as_str)
        .ok_or(ProposalDecodeError::Invalid("schema"))?;
    let version = schema
        .strip_prefix("vpt.proposal/")
        .and_then(|version| version.parse::<u64>().ok())
        .ok_or(ProposalDecodeError::Invalid("schema"))?;
    if version != 1 {
        return Err(ProposalDecodeError::UnsupportedVersion(version));
    }
    if schema != "vpt.proposal/1" {
        return Err(ProposalDecodeError::Invalid("schema"));
    }
    let diagnostics = diagnostics::unknown_fields(&value);
    let document =
        serde_json::from_value(value).map_err(|_| ProposalDecodeError::Invalid("document"))?;
    validate::validate(&document)?;
    Ok(DecodedProposal {
        document,
        diagnostics,
    })
}

#[cfg(test)]
mod tests;
