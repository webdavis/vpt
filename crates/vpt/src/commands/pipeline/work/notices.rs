use super::*;
use crate::{
    commands::note,
    compose::{errors::ledger_error, review_notice},
};
use vpt_domain::identity::RecordingId;

pub(super) fn refresh<E: EventSink>(
    context: &mut Context<'_, E>,
    id: &RecordingId,
    before: (usize, usize),
    report: &Report,
    notices: &mut ReviewQueue,
) -> Result<(), Box<ErrorDocument>> {
    if before == (report.notes.len(), report.synthesized.len()) {
        return Ok(());
    }
    let path = if report.synthesized.last() == Some(id) {
        context
            .ledger
            .note_path(id, NoteStage::Analysis)
            .map_err(|error| Box::new(ledger_error(error)))?
            .ok_or_else(note::errors::not_found)?
    } else {
        report
            .notes
            .last()
            .filter(|(recording, _)| recording == id)
            .ok_or_else(note::errors::not_found)?
            .1
            .clone()
    };
    match review_notice::read(context.ledger, id, path)? {
        Some(notice) => notices.replace(notice).map_err(|_| {
            Box::new(ErrorDocument::new(
                ErrorKind::Ledger,
                "review alert capacity exceeded",
            ))
        })?,
        None => notices.remove(id),
    }
    Ok(())
}
