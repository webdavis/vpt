use super::*;
use crate::ports::{RetentionArtifact, RetentionIntent, StoreError};
use crate::retention::{Moved, RetentionCommit, history};
use vpt_domain::run::{EffectKind, EffectReference, HistoryFact, HistoryRecord};

impl<L, S, T, C, E> Reconciler<'_, L, S, T, C, E>
where
    L: RetentionJournal + RetentionHistory + RunHistory + crate::ports::RetentionArtifacts,
    S: Stores,
    T: Trash,
    C: Clock,
    E: EventSink,
{
    pub(super) fn one(
        &mut self,
        intent: &RetentionIntent,
        artifact: &RetentionArtifact,
        records: &[HistoryRecord],
    ) -> Result<(), Box<ReconcileFailure>> {
        if intent.path.parent() != Some(self.paths.get(artifact.kind.store())) {
            return Err(self.failed(
                RetentionError::Stores(StoreError::Escape(intent.path.clone())),
                None,
            ));
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
                owner: intent.owner.clone(),
                store: artifact.kind.store(),
                path: intent.path.clone(),
            });
            self.completed(&intent.owner);
        }
        let result =
            self.ledger
                .complete_retention(self.run, intent.id, effect.id, self.clock.now());
        let receipt = history::required(result).map_err(|failure| {
            if let Some(RetentionCommit::Step(_)) = &failure.committed {
                self.completed(&intent.owner);
            }
            self.failed(failure.error, failure.committed)
        })?;
        self.completed(&intent.owner);
        history::export(self.sink, &receipt, &mut self.progress.warnings)
            .map_err(|failure| self.failed(failure.error, failure.committed))
    }
}
