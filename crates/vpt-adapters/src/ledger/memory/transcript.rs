use super::{MemoryLedger, State, with};
use vpt_application::ports::{
    AcceptedTranscript, LedgerError, StageState, TranscriptLedger, TranscriptionChange,
};
use vpt_domain::identity::RecordingId;

impl TranscriptLedger for MemoryLedger {
    fn accepted_transcript(
        &self,
        id: &RecordingId,
    ) -> Result<Option<AcceptedTranscript>, LedgerError> {
        with(self, |state| {
            state
                .transcripts
                .iter()
                .find(|row| row.recording == *id)
                .cloned()
        })
    }
}

pub(super) fn apply(state: &mut State, change: &TranscriptionChange) -> Result<(), LedgerError> {
    if let TranscriptionChange::Accept(accepted) = change {
        super::super::transcript::encode(accepted)?;
        super::flags::replace(state, accepted)?;
    }
    let row = state
        .recordings
        .iter_mut()
        .find(|row| row.id == *change.recording())
        .ok_or_else(|| LedgerError::Conflict("unknown transcription recording".into()))?;
    match change {
        TranscriptionChange::Accept(accepted) => {
            row.stages.transcribe = StageState::Succeeded;
            if row.stages.note != StageState::Expired {
                row.stages.note = StageState::Pending;
            }
            match state
                .transcripts
                .iter_mut()
                .find(|saved| saved.recording == accepted.recording)
            {
                Some(saved) => *saved = accepted.as_ref().clone(),
                None => state.transcripts.push(accepted.as_ref().clone()),
            }
        }
        TranscriptionChange::Fail(_) => row.stages.transcribe = StageState::Failed,
    }
    Ok(())
}

#[cfg(test)]
mod tests;
