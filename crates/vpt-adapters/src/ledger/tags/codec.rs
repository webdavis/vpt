use super::{MAX_BYTES, Provenance, Tag, invalid, validate};
use serde::{Deserialize, Serialize};
use vpt_application::ports::LedgerError;
use vpt_domain::{digest::Sha256Digest, tags::State, time::UtcInstant};

#[derive(Serialize)]
struct SavedTag {
    name: String,
    state: String,
    provenance: SavedProvenance,
    updated_at: Option<i64>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum SavedProvenance {
    Manual,
    Proposal {
        by: String,
        reason: String,
        input: String,
    },
}

pub(crate) fn state(value: State) -> &'static str {
    match value {
        State::Confirmed => "confirmed",
        State::Suggested => "suggested",
        State::Rejected => "rejected",
    }
}

fn saved(value: &Provenance) -> SavedProvenance {
    match value {
        Provenance::Manual => SavedProvenance::Manual,
        Provenance::Proposal { by, reason, input } => SavedProvenance::Proposal {
            by: by.clone(),
            reason: reason.clone(),
            input: input.hex(),
        },
    }
}

pub(crate) fn encode(tags: &[Tag]) -> Result<String, LedgerError> {
    validate(tags)?;
    let tags: Vec<_> = tags
        .iter()
        .map(|tag| SavedTag {
            name: tag.name.clone(),
            state: state(tag.state).into(),
            provenance: saved(&tag.provenance),
            updated_at: tag.updated_at.map(|at| at.secs),
        })
        .collect();
    crate::ledger::json::encode(&tags, MAX_BYTES).map_err(|_| invalid())
}

pub(crate) fn provenance(value: &Provenance) -> Result<String, LedgerError> {
    crate::ledger::json::encode(&saved(value), MAX_BYTES).map_err(|_| invalid())
}

pub(crate) fn decode_row(
    name: String,
    state: &str,
    provenance: &str,
    at: Option<i64>,
) -> Result<Tag, LedgerError> {
    let document = vpt_protocol::read_bounded(
        provenance.as_bytes(),
        &vpt_protocol::Limits {
            bytes: MAX_BYTES,
            ..vpt_protocol::Limits::incoming()
        },
    )
    .map_err(|_| invalid())?;
    let saved: SavedProvenance = serde_json::from_value(document).map_err(|_| invalid())?;
    Ok(Tag {
        name,
        state: decode_state(state)?,
        provenance: match saved {
            SavedProvenance::Manual => Provenance::Manual,
            SavedProvenance::Proposal { by, reason, input } => Provenance::Proposal {
                by,
                reason,
                input: Sha256Digest::from_hex(&input).ok_or_else(invalid)?,
            },
        },
        updated_at: at.map(|secs| UtcInstant { secs }),
    })
}

pub(crate) fn decode_state(state: &str) -> Result<State, LedgerError> {
    match state {
        "confirmed" => Ok(State::Confirmed),
        "suggested" => Ok(State::Suggested),
        "rejected" => Ok(State::Rejected),
        _ => Err(invalid()),
    }
}
