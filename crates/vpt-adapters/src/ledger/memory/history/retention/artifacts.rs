use super::super::super::{State, retention_artifacts};
use crate::ledger::retention_artifacts::completion::{binding, conflict, fact};
use vpt_application::ports::*;
use vpt_domain::{
    history::{EventId, RunId},
    run::{EffectOutcome, HistoryFact, WorkOutcome},
    time::UtcInstant,
};

pub(super) fn complete(
    state: &mut State,
    run: RunId,
    intent: &RetentionIntent,
    completed: Option<UtcInstant>,
    effect: EventId,
    at: UtcInstant,
) -> Result<HistoryReceipt, LedgerError> {
    let expected = fact(intent)?;
    if !state
        .history
        .records
        .iter()
        .any(|row| row.id == effect && row.fact == expected)
    {
        return Err(conflict());
    }
    let mut artifact = binding(retention_artifacts::read(state)?, intent)?;
    let mut receipt = HistoryReceipt {
        run,
        records: vec![],
        completed: intent.owner.recording().cloned().into_iter().collect(),
    };
    if completed.is_some() {
        if artifact.trashed_at.is_none() {
            return Err(conflict());
        }
        return Ok(receipt);
    }
    if artifact.trashed_at.is_none() {
        artifact.trashed_at = Some(at);
        artifact.status = RetentionStatus::Expired;
        state.expirations.push(artifact);
    }
    if matches!(
        intent.kind,
        ArtifactKind::Transcript | ArtifactKind::Analysis
    ) {
        let recording = state
            .recordings
            .iter_mut()
            .find(|row| Some(&row.id) == intent.owner.recording())
            .ok_or_else(conflict)?;
        match intent.kind {
            ArtifactKind::Transcript => recording.stages.note = StageState::Expired,
            ArtifactKind::Analysis => recording.stages.synthesis = StageState::Expired,
            _ => unreachable!(),
        }
    }
    state
        .intents
        .iter_mut()
        .find(|(row, _)| row.id == intent.id)
        .ok_or_else(conflict)?
        .1 = Some(at);
    for event in [
        HistoryFact::EffectObserved {
            intent: effect,
            outcome: EffectOutcome::Completed,
        },
        HistoryFact::WorkStep {
            recording: intent.owner.recording().cloned(),
            outcome: WorkOutcome::Completed,
        },
    ] {
        receipt.records.push(state.history.append(run, at, event)?);
    }
    Ok(receipt)
}
