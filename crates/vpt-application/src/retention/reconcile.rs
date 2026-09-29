use super::{Moved, ReconcileFailure, ReconcileProgress, RetentionCommit, RetentionError, history};
use crate::ports::{
    Clock, EventSink, LedgerError, RecordingLedger, RetentionHistory, RetentionIntent,
    RetentionJournal, RunHistory, StoreError, Stores, Trash, TrashError,
};
use std::path::Path;
use vpt_domain::history::{EventId, RunId};
use vpt_domain::layout::StoreKey;
use vpt_domain::run::{
    EffectKind, EffectOutcome, EffectReference, HistoryFact, HistoryRecord, WorkOutcome,
};

pub fn reconcile_intents<L, S, T, C, E>(
    run: RunId,
    ledger: &L,
    stores: &S,
    trash: &T,
    clock: &C,
    audio: &Path,
    sink: &mut E,
) -> Result<ReconcileProgress, Box<ReconcileFailure>>
where
    L: RecordingLedger + RetentionJournal + RetentionHistory + RunHistory,
    S: Stores,
    T: Trash,
    C: Clock,
    E: EventSink,
{
    Reconciler {
        run,
        ledger,
        stores,
        trash,
        clock,
        audio,
        sink,
        progress: ReconcileProgress::default(),
    }
    .run()
}

struct Reconciler<'a, L, S, T, C, E> {
    run: RunId,
    ledger: &'a L,
    stores: &'a S,
    trash: &'a T,
    clock: &'a C,
    audio: &'a Path,
    sink: &'a mut E,
    progress: ReconcileProgress,
}

impl<L, S, T, C, E> Reconciler<'_, L, S, T, C, E>
where
    L: RecordingLedger + RetentionJournal + RetentionHistory + RunHistory,
    S: Stores,
    T: Trash,
    C: Clock,
    E: EventSink,
{
    fn run(mut self) -> Result<ReconcileProgress, Box<ReconcileFailure>> {
        let intents = self
            .ledger
            .pending_intents()
            .map_err(|error| self.failed(RetentionError::Ledger(error), None))?;
        if intents.is_empty() {
            return Ok(self.progress);
        }
        let view = self
            .ledger
            .recovery_view()
            .map_err(|error| self.failed(RetentionError::RequiredHistory(error), None))?;
        for intent in intents {
            self.one(&intent, &view.records)?;
        }
        Ok(self.progress)
    }

    fn one(
        &mut self,
        intent: &RetentionIntent,
        records: &[HistoryRecord],
    ) -> Result<(), Box<ReconcileFailure>> {
        if intent.path.parent() != Some(self.audio) {
            return Err(self.failed(
                RetentionError::Stores(StoreError::Escape(intent.path.clone())),
                None,
            ));
        }
        let recording = self
            .ledger
            .by_id(&intent.recording)
            .map_err(|error| self.failed(RetentionError::Ledger(error), None))?
            .ok_or_else(|| {
                self.failed(
                    RetentionError::Ledger(LedgerError::Corrupt(
                        "pending retention has no recording".into(),
                    )),
                    None,
                )
            })?;
        if recording.audio_path != intent.path || recording.digest != intent.expected {
            return Err(self.failed(RetentionError::TargetModified(intent.path.clone()), None));
        }
        self.stores
            .validate_path(&intent.path)
            .map_err(|error| self.failed(RetentionError::Stores(error), None))?;
        let id = u64::try_from(intent.id).map_err(|_| {
            self.failed(
                RetentionError::MissingHistory {
                    intent_id: intent.id,
                },
                None,
            )
        })?;
        let effect = records.iter().find(|record| {
            record.fact
                == HistoryFact::EffectIntent {
                    kind: EffectKind::RetentionTrash,
                    reference: EffectReference::RetentionIntent {
                        id,
                        path: intent.path.clone(),
                    },
                }
        });
        let Some(effect) = effect else {
            return self.missing_history(intent.id);
        };
        let digest = self
            .stores
            .digest_of(&intent.path)
            .map_err(|error| self.failed(RetentionError::Stores(error), None))?;
        if let Some(digest) = digest {
            if digest != intent.expected {
                return Err(self.failed(RetentionError::TargetModified(intent.path.clone()), None));
            }
            self.stores
                .validate_path(&intent.path)
                .map_err(|error| self.failed(RetentionError::Stores(error), None))?;
            let moved = self.trash.trash(&intent.path);
            self.progress
                .diagnostics
                .extend(self.trash.take_diagnostics());
            if let Err(error) = moved {
                self.failed_effect(intent, effect.id, &error)?;
                return Err(self.failed(RetentionError::Trash(error), None));
            }
            self.progress.moved.push(Moved {
                recording: intent.recording.clone(),
                store: StoreKey::Audio,
                path: intent.path.clone(),
            });
            self.completed(&intent.recording);
        }
        let result =
            self.ledger
                .complete_retention(self.run, intent.id, effect.id, self.clock.now());
        let receipt = history::required(result).map_err(|failure| {
            if let Some(RetentionCommit::Step(receipt)) = &failure.committed {
                for id in &receipt.completed {
                    self.completed(id);
                }
            }
            self.failed(failure.error, failure.committed)
        })?;
        for id in &receipt.completed {
            self.completed(id);
        }
        history::export(self.sink, &receipt, &mut self.progress.warnings)
            .map_err(|failure| self.failed(failure.error, failure.committed))
    }

    fn failed_effect(
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
                        recording: Some(intent.recording.clone()),
                        outcome: work_outcome,
                    },
                ),
            ],
            &mut self.progress.warnings,
        )
        .map_err(|failure| self.failed(failure.error, failure.committed))?;
        Ok(())
    }

    fn missing_history(&mut self, intent_id: i64) -> Result<(), Box<ReconcileFailure>> {
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

    fn completed(&mut self, id: &vpt_domain::identity::RecordingId) {
        if !self.progress.completed.contains(id) {
            self.progress.completed.push(id.clone());
        }
    }

    fn failed(
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
