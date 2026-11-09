use crate::{
    commands::note,
    compose::{errors::ledger_error, review_notice},
};
use vpt_application::ports::*;
use vpt_domain::{identity::RecordingId, run::HistoryFact, time::UtcInstant};
use vpt_protocol::error::ErrorDocument;

pub(super) fn fact(
    ledger: &(impl NoteLedger + ReviewLedger),
    id: &RecordingId,
    at: UtcInstant,
) -> Result<Option<(UtcInstant, HistoryFact)>, Box<ErrorDocument>> {
    let path = ledger
        .note_path(id, NoteStage::Analysis)
        .map_err(|error| Box::new(ledger_error(error)))?
        .ok_or_else(note::errors::not_found)?;
    Ok(review_notice::read(ledger, id, path)?
        .map(|notice| (at, HistoryFact::Event(notice.event(at)))))
}
