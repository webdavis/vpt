use super::super::State;
use crate::ledger::verification::state::{self, VerifiedArtifact};
use vpt_application::ports::{LedgerError, ReviewSnapshot, StageState, WorkDelta};
use vpt_domain::identity::RecordingId;

pub(in crate::ledger::memory) fn save(state: &mut State, value: VerifiedArtifact) {
    state
        .verification
        .retain(|row| row.recording != value.recording || row.artifact != value.artifact);
    state.verification.push(value);
}

pub(in crate::ledger::memory) fn review(
    state: &State,
    id: &RecordingId,
) -> Result<ReviewSnapshot, LedgerError> {
    let mut review = ReviewSnapshot {
        accepted: state
            .transcripts
            .iter()
            .find(|row| row.recording == *id)
            .cloned(),
        flags: state
            .flags
            .iter()
            .filter(|(recording, _)| recording == id)
            .map(|(_, flag)| flag.clone())
            .collect(),
        ..Default::default()
    };
    let saved: Vec<_> = state
        .verification
        .iter()
        .filter(|row| row.recording == *id)
        .cloned()
        .collect();
    state::partition(&mut review, &saved, |stage, flags| {
        state
            .notes
            .iter()
            .find(|note| note.recording == *id && note.stage == stage)
            .map(|note| state::claims(note, flags))
            .transpose()
    })?;
    Ok(review)
}

pub(in crate::ledger::memory) fn refresh(
    state: &mut State,
    delta: &WorkDelta,
) -> Result<(), LedgerError> {
    for id in state::affected(delta) {
        let Some(proposal) = state.proposals.iter().find(|row| row.recording == *id) else {
            continue;
        };
        let review = review(state, id)?;
        let basis = review
            .accepted
            .as_ref()
            .map(|accepted| crate::ledger::proposal::basis::compute(accepted, &review.flags))
            .transpose()?;
        if Some(proposal.basis) != basis
            && let Some(record) = state
                .recordings
                .iter_mut()
                .find(|row| row.id == *id && row.stages.analysis == StageState::Succeeded)
        {
            record.stages.analysis = StageState::Pending;
        }
    }
    Ok(())
}
