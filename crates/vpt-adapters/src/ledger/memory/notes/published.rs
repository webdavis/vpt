use super::*;
use vpt_application::ports::StageState;

pub(in crate::ledger) fn apply(state: &mut State, target: &Path) -> Result<(), LedgerError> {
    let Some(note) = state.notes.iter().find(|note| {
        note.target == target
            && state
                .publications
                .iter()
                .any(|entry| entry.target == target && entry.intended == note.digest)
    }) else {
        return Ok(());
    };
    let next = match note.stage {
        NoteStage::Transcript => StageState::Succeeded,
        NoteStage::Analysis => {
            let id = &note.recording;
            let flags: Vec<_> = state
                .flags
                .iter()
                .filter(|(recording, _)| recording == id)
                .map(|(_, flag)| flag.clone())
                .collect();
            if crate::ledger::proposal::is_current(
                state.proposals.iter().find(|row| &row.recording == id),
                state.transcripts.iter().find(|row| &row.recording == id),
                state
                    .notes
                    .iter()
                    .find(|row| &row.recording == id && row.stage == NoteStage::Transcript),
                &flags,
            )? {
                StageState::Succeeded
            } else {
                StageState::Pending
            }
        }
    };
    if let Some(record) = state
        .recordings
        .iter_mut()
        .find(|record| record.id == note.recording)
    {
        match note.stage {
            NoteStage::Transcript => record.stages.note = next,
            NoteStage::Analysis => {
                if !matches!(
                    record.stages.analysis,
                    StageState::Failed | StageState::Disabled
                ) {
                    record.stages.analysis = next;
                }
            }
        }
    }
    Ok(())
}
