use super::{observe, source};
use crate::compose::errors::{contained_error, ledger_error};
use vpt_adapters::{FilesystemStores, Roots, SqliteLedger};
use vpt_application::{Settings, ports::*};
use vpt_domain::identity::RecordingId;
use vpt_protocol::error::ErrorDocument;

pub(super) fn pending(ledger: &SqliteLedger, id: &RecordingId) -> Result<bool, Box<ErrorDocument>> {
    let input = ledger
        .note_inputs(id)
        .map_err(|error| Box::new(ledger_error(error)))?;
    let pending = ledger
        .pending_publications()
        .map_err(|error| Box::new(ledger_error(error)))?;
    Ok(input.is_some_and(|input| {
        input.proposal.as_ref().is_some_and(|proposal| {
            proposal
                .vocabulary_publication
                .as_ref()
                .is_some_and(|entry| pending.contains(entry))
                || input.notes.iter().any(|note| {
                    pending
                        .iter()
                        .any(|entry| entry.target == note.target && entry.intended == note.digest)
                })
        })
    }))
}

pub(crate) fn current(
    input: &NoteInputs,
    settings: &Settings,
    roots: &Roots,
) -> Result<bool, Box<ErrorDocument>> {
    if input.record.stages.analysis != StageState::Succeeded {
        return Ok(false);
    }
    let stores = FilesystemStores::open_read_only(std::slice::from_ref(&roots.stores.transcripts))
        .map_err(|error| Box::new(contained_error(error)))?;
    if !input_matches(input, settings, &stores)? {
        return Ok(false);
    }
    let source = input
        .notes
        .iter()
        .find(|note| note.stage == NoteStage::Transcript)
        .ok_or_else(super::errors::invalid)?;
    source::check(&stores, source)?;
    Ok(true)
}

pub(crate) fn input_matches(
    input: &NoteInputs,
    settings: &Settings,
    stores: &impl Stores,
) -> Result<bool, Box<ErrorDocument>> {
    let (Some(proposal), Some(accepted), Some(source)) = (
        &input.proposal,
        &input.review.accepted,
        input
            .notes
            .iter()
            .find(|note| note.stage == NoteStage::Transcript),
    ) else {
        return Ok(false);
    };
    if proposal.grounding_window_secs != settings.analysis.grounding_window_secs
        || proposal.input_digest != source.content_digest
        || proposal.basis
            != vpt_adapters::grounding_basis_for(accepted, &input.review.flags)
                .map_err(|error| Box::new(ledger_error(error)))?
    {
        return Ok(false);
    }
    Ok(stores.digest_bytes(observe::content(input, settings)?.as_bytes()) == proposal.input_digest)
}
