use super::*;
use crate::ledger::contract::{notes, record};
pub(crate) fn brief_input_stream_keeps_published_notes_and_stops_on_visitor_failure(
    ledger: &(impl BriefInputsLedger + RecordingLedger + RunHistory + NoteLedger + PublicationJournal),
) {
    notes::note_stage_completion_commits_with_publication_history(ledger);
    ledger
        .commit(&LedgerCommit {
            recordings: vec![record(82)],
            ..Default::default()
        })
        .unwrap();
    let before = ledger.pending_exports().unwrap();
    let mut inputs = Vec::new();
    ledger
        .visit_brief_inputs(&mut |row| {
            inputs.push(row);
            Ok(())
        })
        .unwrap();
    assert_eq!(inputs.len(), 2);
    let owned = inputs
        .iter()
        .find(|row| row.recording.id == record(81).id)
        .unwrap();
    assert_eq!(owned.note, Some(notes::artifact()));
    let pending = inputs
        .iter()
        .find(|row| row.recording.id == record(82).id)
        .unwrap();
    assert!(pending.note.is_none());
    let mut visits = 0;
    let result = ledger.visit_brief_inputs(&mut |_| {
        visits += 1;
        Err(LedgerError::Busy)
    });
    assert_eq!(result, Err(LedgerError::Busy));
    assert_eq!(visits, 1);
    assert_eq!(ledger.pending_exports().unwrap(), before);
    assert!(ledger.pending_publications().unwrap().is_empty());
}
