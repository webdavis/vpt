use crate::{
    commands::note,
    compose::{errors::ledger_error, history::commit, review_notice},
};
use vpt_adapters::{SqliteLedger, SystemClock};
use vpt_application::ports::*;
use vpt_domain::{
    history::{Event, EventKind, RunId},
    identity::RecordingId,
    run::HistoryFact,
    time::UtcInstant,
};
use vpt_protocol::error::ErrorDocument;

pub(super) fn analyzed(
    ledger: &SqliteLedger,
    sink: &mut impl EventSink,
    warnings: &mut std::collections::BTreeSet<SinkWarning>,
    run: RunId,
    id: &RecordingId,
) -> Result<(), Box<ErrorDocument>> {
    let note = ledger
        .note_path(id, NoteStage::Analysis)
        .map_err(|error| Box::new(ledger_error(error)))?;
    let at = SystemClock.now();
    let event = Event::recording_done(EventKind::Analyzed, id.clone(), note, at);
    commit(
        ledger.commit_step(
            run,
            &WorkDelta::default(),
            &[(at, HistoryFact::Event(event))],
        ),
        sink,
        warnings,
    )
    .map(|_| ())
}

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
