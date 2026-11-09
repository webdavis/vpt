use super::{
    Kept, KeptReason, Moved, Retention, RetentionCommit, RetentionError, RetentionReport, history,
};
use crate::ports::{
    ArtifactKind, Clock, EventSink, HistoryWrite, NewIntent, RetentionArtifact, RetentionCheck,
    RetentionHistory, RetentionJournal, RunHistory, Stores, Trash, TrashError,
};
use vpt_domain::history::RunId;
use vpt_domain::run::{EffectOutcome, HistoryFact, WorkOutcome};

impl<L, S, T, C, E> Retention<'_, L, S, T, C, E>
where
    L: RetentionJournal + RetentionHistory + RunHistory + crate::ports::RetentionArtifacts,
    S: Stores + crate::ports::RetentionFiles,
    T: Trash,
    C: Clock,
    E: EventSink,
{
    pub(super) fn expire(
        &mut self,
        run: Option<RunId>,
        artifact: &RetentionArtifact,
        report: &mut RetentionReport,
    ) -> Result<(), Box<history::Failure>> {
        let path = &artifact.path;
        self.stores
            .validate_path(path)
            .map_err(|error| history::failure(RetentionError::Stores(error)))?;
        let expected = match self
            .stores
            .check_retention(artifact)
            .map_err(|error| history::failure(RetentionError::Stores(error)))?
        {
            RetentionCheck::Owned(digest) => digest,
            other => {
                let reason = if other == RetentionCheck::Missing {
                    KeptReason::Missing
                } else if matches!(
                    artifact.kind,
                    ArtifactKind::Transcript | ArtifactKind::Analysis | ArtifactKind::Brief
                ) {
                    KeptReason::OwnershipMismatch
                } else {
                    KeptReason::DigestMismatch
                };
                report.kept.push(Kept {
                    path: path.clone(),
                    reason,
                });
                return Ok(());
            }
        };
        let moved = Moved {
            owner: artifact.owner.clone(),
            store: artifact.kind.store(),
            path: path.clone(),
        };
        let Some(run) = run else {
            report.moved.push(moved);
            return Ok(());
        };
        let intent = NewIntent {
            kind: artifact.kind,
            owner: artifact.owner.clone(),
            path: path.clone(),
            expected,
            recorded_at: self.clock.now(),
        };
        let prepared = match self.ledger.prepare_retention(run, &intent) {
            HistoryWrite::Committed(prepared) => prepared,
            HistoryWrite::CommittedWithFailure { receipt, error } => {
                return Err(Box::new(history::Failure {
                    error: RetentionError::RequiredHistory(error),
                    committed: Some(RetentionCommit::Preparation(receipt)),
                }));
            }
            HistoryWrite::Unconfirmed(error) => return Err(history::required_error(error)),
        };
        history::export(self.sink, &prepared.receipt, &mut report.warnings)?;
        self.stores
            .validate_path(path)
            .map_err(|error| history::failure(RetentionError::Stores(error)))?;
        if self
            .stores
            .digest_of(path)
            .map_err(|error| history::failure(RetentionError::Stores(error)))?
            != Some(expected)
        {
            return Err(history::failure(RetentionError::TargetModified(
                path.clone(),
            )));
        }
        self.stores
            .validate_path(path)
            .map_err(|error| history::failure(RetentionError::Stores(error)))?;
        let result = self.trash.trash(path);
        report.diagnostics.extend(self.trash.take_diagnostics());
        if let Err(error) = result {
            let (effect, work) = match error {
                TrashError::Unknown(_) => (EffectOutcome::Unknown, WorkOutcome::Unknown),
                _ => (EffectOutcome::Failed, WorkOutcome::Failed),
            };
            let at = self.clock.now();
            history::append(
                self.ledger,
                self.sink,
                run,
                &[
                    (
                        at,
                        HistoryFact::EffectObserved {
                            intent: prepared.effect_id,
                            outcome: effect,
                        },
                    ),
                    (
                        at,
                        HistoryFact::WorkStep {
                            recording: artifact.owner.recording().cloned(),
                            outcome: work,
                        },
                    ),
                ],
                &mut report.warnings,
            )?;
            return Err(history::failure(RetentionError::Trash(error)));
        }
        report.moved.push(moved);
        report.completed(&artifact.owner);
        let receipt = history::required(self.ledger.complete_retention(
            run,
            prepared.intent_id,
            prepared.effect_id,
            self.clock.now(),
        ))?;
        for id in &receipt.completed {
            report.completed(&id.clone().into());
        }
        history::export(self.sink, &receipt, &mut report.warnings)
    }
}
