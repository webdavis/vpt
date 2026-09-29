use super::{
    Kept, KeptReason, Moved, Retention, RetentionCommit, RetentionError, RetentionReport, history,
};
use crate::ports::{
    ArtifactKind, Clock, EventSink, HistoryWrite, NewIntent, RecordingLedger, RecordingRecord,
    RetentionHistory, RetentionJournal, RunHistory, Stores, Trash, TrashError,
};
use vpt_domain::history::RunId;
use vpt_domain::layout::StoreKey;
use vpt_domain::run::{EffectOutcome, HistoryFact, WorkOutcome};

impl<L, S, T, C, E> Retention<'_, L, S, T, C, E>
where
    L: RecordingLedger + RetentionJournal + RetentionHistory + RunHistory,
    S: Stores,
    T: Trash,
    C: Clock,
    E: EventSink,
{
    pub(super) fn expire(
        &mut self,
        run: Option<RunId>,
        record: &RecordingRecord,
        report: &mut RetentionReport,
    ) -> Result<(), Box<history::Failure>> {
        let path = &record.audio_path;
        self.stores
            .validate_path(path)
            .map_err(|error| history::failure(RetentionError::Stores(error)))?;
        if self
            .stores
            .digest_of(path)
            .map_err(|error| history::failure(RetentionError::Stores(error)))?
            != Some(record.digest)
        {
            report.kept.push(Kept {
                path: path.clone(),
                reason: KeptReason::DigestMismatch,
            });
            return Ok(());
        }
        let moved = Moved {
            recording: record.id.clone(),
            store: StoreKey::Audio,
            path: path.clone(),
        };
        let Some(run) = run else {
            report.moved.push(moved);
            return Ok(());
        };
        let intent = NewIntent {
            kind: ArtifactKind::Audio,
            recording: record.id.clone(),
            path: path.clone(),
            expected: record.digest,
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
            != Some(record.digest)
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
                            recording: Some(record.id.clone()),
                            outcome: work,
                        },
                    ),
                ],
                &mut report.warnings,
            )?;
            return Err(history::failure(RetentionError::Trash(error)));
        }
        report.moved.push(moved);
        report.completed(&record.id);
        let receipt = history::required(self.ledger.complete_retention(
            run,
            prepared.intent_id,
            prepared.effect_id,
            self.clock.now(),
        ))?;
        for id in &receipt.completed {
            report.completed(id);
        }
        history::export(self.sink, &receipt, &mut report.warnings)
    }
}
