use super::{MemoryLedger, State, with};
use crate::ledger::proposal::{self as validation, conflict};
use vpt_application::ports::*;
use vpt_domain::{identity::RecordingId, run::HistoryFact, time::UtcInstant};

impl ProposalLedger for MemoryLedger {
    fn accepted_proposal(&self, id: &RecordingId) -> Result<Option<AcceptedProposal>, LedgerError> {
        with(self, |state| {
            state
                .proposals
                .iter()
                .find(|row| row.recording == *id)
                .cloned()
        })
    }
    fn grounding_basis(&self, id: &RecordingId) -> Result<Option<GroundingBasis>, LedgerError> {
        with(self, |state| {
            let Some(accepted) = state.transcripts.iter().find(|row| row.recording == *id) else {
                return Ok(None);
            };
            let flags: Vec<_> = state
                .flags
                .iter()
                .filter(|(recording, _)| recording == id)
                .map(|(_, flag)| flag.clone())
                .collect();
            validation::basis::compute(accepted, &flags).map(Some)
        })?
    }
}

pub(super) fn apply(
    state: &mut State,
    delta: &WorkDelta,
    facts: &[(UtcInstant, HistoryFact)],
) -> Result<(), LedgerError> {
    validation::history(delta, facts)?;
    let Some(change) = &delta.proposal else {
        return Ok(());
    };
    let index = state
        .recordings
        .iter()
        .position(|row| row.id == *change.recording())
        .ok_or_else(conflict)?;
    match change {
        ProposalChange::Fail { .. } => {
            state.recordings[index].stages.synthesis = StageState::Failed
        }
        ProposalChange::Accept(accepted) => {
            let transcript = state
                .transcripts
                .iter()
                .find(|row| row.recording == accepted.recording)
                .ok_or_else(conflict)?;
            let flags: Vec<_> = state
                .flags
                .iter()
                .filter(|(id, _)| *id == accepted.recording)
                .map(|(_, flag)| flag.clone())
                .collect();
            let source = state
                .notes
                .iter()
                .find(|note| {
                    note.recording == accepted.recording && note.stage == NoteStage::Transcript
                })
                .ok_or_else(conflict)?;
            if state
                .publications
                .iter()
                .any(|entry| entry.target == source.target)
                || state
                    .intents
                    .iter()
                    .any(|(intent, done)| done.is_none() && intent.path == source.target)
            {
                return Err(conflict());
            }
            let prepared = validation::check(
                accepted,
                delta,
                transcript,
                &flags,
                source,
                state.recordings[index].stages.note,
            )?;
            super::verification::replace(
                state,
                &accepted.recording,
                NoteStage::Analysis,
                prepared.findings,
            );
            super::verification::state::save(state, prepared.verification);
            match state
                .proposals
                .iter_mut()
                .find(|row| row.recording == accepted.recording)
            {
                Some(row) => *row = accepted.as_ref().clone(),
                None => state.proposals.push(accepted.as_ref().clone()),
            }
            state.recordings[index].stages.synthesis = StageState::Pending;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
