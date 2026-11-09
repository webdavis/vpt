use super::*;
use crate::ports::{RetentionIntent, TrashError};
use crate::retention::{RetentionCommit, history};
use vpt_domain::{
    history::EventId,
    run::{EffectOutcome, HistoryFact, WorkOutcome},
};

impl<L, S, T, C, E> Reconciler<'_, L, S, T, C, E>
where
    L: RetentionJournal + RetentionHistory + RunHistory + crate::ports::RetentionArtifacts,
    S: Stores,
    T: Trash,
    C: Clock,
    E: EventSink,
{
    pub(super) fn failed_effect(
        &mut self,
        intent: &RetentionIntent,
        effect: EventId,
        error: &TrashError,
    ) -> Result<(), Box<ReconcileFailure>> {
        let (effect_outcome, work_outcome) = match error {
            TrashError::Unknown(_) => (EffectOutcome::Unknown, WorkOutcome::Unknown),
            _ => (EffectOutcome::Failed, WorkOutcome::Failed),
        };
        let at = self.clock.now();
        history::append(
            self.ledger,
            self.sink,
            self.run,
            &[
                (
                    at,
                    HistoryFact::EffectObserved {
                        intent: effect,
                        outcome: effect_outcome,
                    },
                ),
                (
                    at,
                    HistoryFact::WorkStep {
                        recording: intent.owner.recording().cloned(),
                        outcome: work_outcome,
                    },
                ),
            ],
            &mut self.progress.warnings,
        )
        .map_err(|failure| self.failed(failure.error, failure.committed))?;
        Ok(())
    }

    pub(super) fn missing_history(&mut self, intent_id: i64) -> Result<(), Box<ReconcileFailure>> {
        let observed_id = u64::try_from(intent_id)
            .map_err(|_| self.failed(RetentionError::MissingHistory { intent_id }, None))?;
        let receipt = history::append(
            self.ledger,
            self.sink,
            self.run,
            &[(
                self.clock.now(),
                HistoryFact::MissingRetentionHistory {
                    intent_id: observed_id,
                },
            )],
            &mut self.progress.warnings,
        )
        .map_err(|failure| self.failed(failure.error, failure.committed))?;
        Err(self.failed(
            RetentionError::MissingHistory { intent_id },
            Some(RetentionCommit::Step(receipt)),
        ))
    }

    pub(super) fn completed(&mut self, id: &crate::ports::ArtifactOwner) {
        if !self.progress.completed.contains(id) {
            self.progress.completed.push(id.clone());
        }
    }

    pub(super) fn failed(
        &self,
        error: RetentionError,
        committed: Option<RetentionCommit>,
    ) -> Box<ReconcileFailure> {
        Box::new(ReconcileFailure {
            error,
            progress: self.progress.clone(),
            committed,
        })
    }
}
