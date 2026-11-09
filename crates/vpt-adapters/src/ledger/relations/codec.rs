use super::{MAX_BYTES, MAX_ITEMS, invalid};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use vpt_application::ports::LedgerError;
use vpt_domain::{digest::Sha256Digest, identity::RecordingId, relations::*};

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum SavedProvenance {
    Manual,
    Derived,
    Proposal {
        by: String,
        reason: String,
        input: String,
    },
}

pub(crate) struct Row {
    pub kind: String,
    pub target: String,
    pub state: String,
    pub provenance: String,
    pub rule: Option<String>,
    pub ranges: String,
}
impl Row {
    pub fn bytes(&self) -> usize {
        self.kind.len()
            + self.target.len()
            + self.state.len()
            + self.provenance.len()
            + self.rule.as_ref().map_or(0, String::len)
            + self.ranges.len()
    }
}

pub(crate) fn encode(relations: &[Relation]) -> Result<Vec<Row>, LedgerError> {
    super::validate::relations(relations)?;
    let mut remaining = MAX_BYTES;
    let mut rows = Vec::new();
    for relation in relations {
        let (provenance, rule) = match &relation.provenance {
            Provenance::Manual => (SavedProvenance::Manual, None),
            Provenance::Derived(rule) => (
                SavedProvenance::Derived,
                Some(
                    match rule {
                        Rule::ImmediatePredecessor => "immediate_predecessor",
                        Rule::KnownTerm => "known_term",
                    }
                    .into(),
                ),
            ),
            Provenance::Proposal { by, reason, input } => (
                SavedProvenance::Proposal {
                    by: by.clone(),
                    reason: reason.clone(),
                    input: input.hex(),
                },
                None,
            ),
        };
        let row = Row {
            kind: relation.kind.as_str().into(),
            target: relation.target.as_str().into(),
            state: match relation.state {
                State::Confirmed => "confirmed",
                State::Suggested => "suggested",
                State::Rejected => "rejected",
            }
            .into(),
            provenance: crate::ledger::json::encode(&provenance, remaining)
                .map_err(|_| invalid())?,
            rule,
            ranges: super::ranges::encode(&relation.ranges, remaining)?,
        };
        remaining = remaining.checked_sub(row.bytes()).ok_or_else(invalid)?;
        rows.push(row);
    }
    Ok(rows)
}

pub(crate) fn decode(row: Row) -> Result<Relation, LedgerError> {
    let provenance = match parse::<SavedProvenance>(&row.provenance)? {
        SavedProvenance::Manual if row.rule.is_none() => Provenance::Manual,
        SavedProvenance::Derived => Provenance::Derived(match row.rule.as_deref() {
            Some("immediate_predecessor") => Rule::ImmediatePredecessor,
            Some("known_term") => Rule::KnownTerm,
            _ => return Err(invalid()),
        }),
        SavedProvenance::Proposal { by, reason, input } if row.rule.is_none() => {
            Provenance::Proposal {
                by,
                reason,
                input: Sha256Digest::from_hex(&input).ok_or_else(invalid)?,
            }
        }
        _ => return Err(invalid()),
    };
    Ok(Relation {
        kind: Kind::parse(&row.kind).ok_or_else(invalid)?,
        target: if row.target.starts_with('/') {
            Target::Note(row.target)
        } else {
            Target::Recording(RecordingId::parse(&row.target).map_err(|_| invalid())?)
        },
        state: match row.state.as_str() {
            "confirmed" => State::Confirmed,
            "suggested" => State::Suggested,
            "rejected" => State::Rejected,
            _ => return Err(invalid()),
        },
        provenance,
        ranges: super::ranges::decode(&row.ranges)?,
    })
}

fn parse<T: DeserializeOwned>(text: &str) -> Result<T, LedgerError> {
    let doc = vpt_protocol::read_bounded(
        text.as_bytes(),
        &vpt_protocol::Limits {
            bytes: MAX_BYTES,
            array_len: MAX_ITEMS,
            ..vpt_protocol::Limits::incoming()
        },
    )
    .map_err(|_| invalid())?;
    serde_json::from_value(doc).map_err(|_| invalid())
}
