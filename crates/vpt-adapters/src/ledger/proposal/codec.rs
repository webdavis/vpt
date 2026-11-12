mod grounding;
mod publication;
mod raw;
mod restore;
use super::invalid;
use serde::{Deserialize, Serialize};
use vpt_application::ports::{AcceptedProposal, GroundingBasis, LedgerError};
use vpt_domain::{
    digest::Sha256Digest, identity::RecordingId, proposal::CheckedProposal, time::UtcInstant,
};

pub(in crate::ledger) const MAX_BODY: usize = 16 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Saved {
    version: u32,
    #[serde(default)]
    vocabulary_publication: Option<publication::Saved>,
    transcript_digest: String,
    lexical_flags_digest: String,
    grounding_window_secs: u32,
    document: String,
    summary: Vec<grounding::SavedGrounding>,
    actions: Vec<grounding::SavedGrounding>,
}

pub(in crate::ledger) fn encode(accepted: &AcceptedProposal) -> Result<String, LedgerError> {
    let document = raw::encode(&accepted.proposal)?;
    grounding::check(&accepted.proposal)?;
    let saved = Saved {
        version: 2,
        vocabulary_publication: accepted
            .vocabulary_publication
            .as_ref()
            .map(|entry| publication::save(entry, accepted.accepted_at))
            .transpose()?,
        transcript_digest: accepted.basis.transcript.hex(),
        lexical_flags_digest: accepted.basis.lexical_flags.hex(),
        grounding_window_secs: accepted.grounding_window_secs,
        document,
        summary: accepted
            .proposal
            .summary
            .iter()
            .map(|claim| grounding::save(&claim.grounding))
            .collect(),
        actions: accepted
            .proposal
            .actions
            .iter()
            .map(grounding::save_action)
            .collect(),
    };
    super::super::json::encode(&saved, MAX_BODY).map_err(|_| invalid())
}

pub(in crate::ledger) fn decode(
    recording: RecordingId,
    input_digest: &str,
    body: &str,
    accepted_at: UtcInstant,
) -> Result<AcceptedProposal, LedgerError> {
    let value = vpt_protocol::read_bounded(
        body.as_bytes(),
        &vpt_protocol::Limits {
            bytes: MAX_BODY,
            depth: 8,
            text_chars: MAX_BODY,
            array_len: 65_536,
        },
    )
    .map_err(|_| invalid())?;
    let saved: Saved = serde_json::from_value(value).map_err(|_| invalid())?;
    if !matches!(saved.version, 1 | 2)
        || (saved.version == 1 && saved.vocabulary_publication.is_some())
    {
        return Err(invalid());
    }
    let raw = crate::proposal::decode(saved.document.as_bytes())
        .map_err(|_| invalid())?
        .proposal;
    let summary = restore::claims(raw.summary, saved.summary)?;
    if raw.actions.len() != saved.actions.len() {
        return Err(invalid());
    }
    let actions = raw
        .actions
        .into_iter()
        .zip(saved.actions)
        .map(|(action, saved)| restore::action(action, saved))
        .collect::<Result<_, _>>()?;
    let accepted = AcceptedProposal {
        vocabulary_publication: saved
            .vocabulary_publication
            .map(|entry| publication::restore(entry, accepted_at))
            .transpose()?,
        recording,
        input_digest: digest(input_digest)?,
        basis: GroundingBasis {
            transcript: digest(&saved.transcript_digest)?,
            lexical_flags: digest(&saved.lexical_flags_digest)?,
        },
        grounding_window_secs: saved.grounding_window_secs,
        accepted_at,
        proposal: CheckedProposal {
            by: raw.by,
            summary,
            actions,
            tags: raw.tags,
            relations: raw.relations,
        },
    };
    grounding::check(&accepted.proposal)?;
    Ok(accepted)
}

fn digest(value: &str) -> Result<Sha256Digest, LedgerError> {
    Sha256Digest::from_hex(value).ok_or_else(invalid)
}

#[cfg(test)]
mod tests;
