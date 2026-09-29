use super::{SqliteLedger, connection, error, map};
use crate::database::AccessPolicy;
use rusqlite::{Transaction, TransactionBehavior};
mod codec;
mod export;
pub(super) mod retention;
mod storage;
mod store;
use vpt_application::ports::{HistoryReceipt, HistoryWrite, LedgerError, RunHistory, WorkDelta};
use vpt_domain::history::{EventId, RunId};
use vpt_domain::run::{
    ExportCursor, ExportFailure, ExportPosition, HistoryFact, HistoryRecord, RecoveryView, RunKind,
};
use vpt_domain::time::UtcInstant;

impl SqliteLedger {
    fn history_write<T>(
        &self,
        operation: impl FnOnce(&Transaction<'_>) -> Result<T, LedgerError>,
    ) -> HistoryWrite<T> {
        self.history_write_with(operation, || {}, |closed| closed)
    }

    fn history_write_with<T>(
        &self,
        operation: impl FnOnce(&Transaction<'_>) -> Result<T, LedgerError>,
        after_commit: impl FnOnce(),
        close_result: impl FnOnce(Result<(), LedgerError>) -> Result<(), LedgerError>,
    ) -> HistoryWrite<T> {
        if self.policy != AccessPolicy::WritableLedger {
            return HistoryWrite::Unconfirmed(LedgerError::Io("read-only ledger".into()));
        }
        let mut owner = match connection::open(&self.state, self.busy, self.policy) {
            Ok(owner) => owner,
            Err(error) => return HistoryWrite::Unconfirmed(error),
        };
        let mut committed = None;
        let result = owner.with_connection(|connection| {
            let transaction =
                Transaction::new_unchecked(connection, TransactionBehavior::Immediate)
                    .map_err(map)?;
            let receipt = operation(&transaction)?;
            self.state.revalidate().map_err(error::contained)?;
            transaction.commit().map_err(map)?;
            committed = Some(receipt);
            after_commit();
            Ok(())
        });
        let closed = close_result(owner.try_close().map_err(Into::into));
        let finished = error::finish(result, closed);
        match (finished, committed) {
            (Ok(()), Some(receipt)) => HistoryWrite::Committed(receipt),
            (Err(error), Some(receipt)) => HistoryWrite::CommittedWithFailure { receipt, error },
            (Err(error), None) => HistoryWrite::Unconfirmed(error),
            (Ok(()), None) => {
                HistoryWrite::Unconfirmed(LedgerError::Corrupt("missing history receipt".into()))
            }
        }
    }
}

impl RunHistory for SqliteLedger {
    fn begin_run(&self, kind: RunKind, at: UtcInstant) -> HistoryWrite<HistoryReceipt> {
        store::begin(self, kind, at)
    }

    fn commit_step(
        &self,
        run: RunId,
        delta: &WorkDelta,
        facts: &[(UtcInstant, HistoryFact)],
    ) -> HistoryWrite<HistoryReceipt> {
        store::step(self, run, delta, facts)
    }

    fn recovery_view(&self) -> Result<RecoveryView, LedgerError> {
        export::view(self)
    }

    fn pending_exports(&self) -> Result<Vec<HistoryRecord>, LedgerError> {
        self.read(|connection| storage::records(connection, true))
    }

    fn acknowledge_export(
        &self,
        event: EventId,
        expected: Option<ExportCursor>,
        position: ExportPosition,
        appended_at: UtcInstant,
    ) -> HistoryWrite<()> {
        export::acknowledge(self, event, expected, position, appended_at)
    }

    fn note_export_failure(&self, failure: &ExportFailure) -> HistoryWrite<()> {
        export::note_failure(self, failure)
    }

    fn restart_export(
        &self,
        expected: ExportCursor,
        observed: ExportPosition,
        observed_at: UtcInstant,
    ) -> HistoryWrite<()> {
        export::restart(self, expected, observed, observed_at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RootDir;
    use crate::database::root;

    #[cfg(unix)]
    #[test]
    fn non_utf8_history_fact_after_source_update_rolls_back_both() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;
        use vpt_application::ports::{LedgerCommit, RecordingLedger};
        use vpt_domain::run::{EffectKind, EffectReference};

        let (_temp, state) = root();
        let ledger = SqliteLedger::open(&state).unwrap();
        let recording = crate::ledger::contract::record(95);
        ledger
            .commit(&LedgerCommit {
                recordings: vec![recording.clone()],
                ..Default::default()
            })
            .unwrap();
        let run = match ledger.begin_run(RunKind::Publish, UtcInstant { secs: 10 }) {
            HistoryWrite::Committed(receipt) => receipt.run,
            other => panic!("run: {other:?}"),
        };
        let invalid = std::path::PathBuf::from(OsString::from_vec(vec![b'/', b'h', b'/', 0xff]));
        let changed = std::path::PathBuf::from("/vm/Recordings/changed-95.m4a");
        let result = ledger.commit_step(
            run,
            &WorkDelta {
                source_updates: vec![(recording.id.clone(), changed)],
                ..Default::default()
            },
            &[(
                UtcInstant { secs: 11 },
                HistoryFact::EffectIntent {
                    kind: EffectKind::Archive,
                    reference: EffectReference::ArchiveTarget(invalid),
                },
            )],
        );
        assert!(matches!(result, HistoryWrite::Unconfirmed(_)), "{result:?}");
        assert_eq!(
            ledger.by_id(&recording.id).unwrap().unwrap().source_path,
            recording.source_path
        );
        assert_eq!(ledger.recovery_view().unwrap().records.len(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_source_update_refuses_without_changing_recording_or_history() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;
        use vpt_application::ports::{LedgerCommit, RecordingLedger};
        use vpt_domain::run::WorkOutcome;

        let (_temp, state) = root();
        let ledger = SqliteLedger::open(&state).unwrap();
        let recording = crate::ledger::contract::record(96);
        ledger
            .commit(&LedgerCommit {
                recordings: vec![recording.clone()],
                ..Default::default()
            })
            .unwrap();
        let run = match ledger.begin_run(RunKind::Ingest, UtcInstant { secs: 10 }) {
            HistoryWrite::Committed(receipt) => receipt.run,
            other => panic!("run: {other:?}"),
        };
        let invalid =
            std::path::PathBuf::from(OsString::from_vec(vec![b'/', b'v', b'm', b'/', 0xff]));
        let result = ledger.commit_step(
            run,
            &WorkDelta {
                source_updates: vec![(recording.id.clone(), invalid)],
                ..Default::default()
            },
            &[(
                UtcInstant { secs: 11 },
                HistoryFact::WorkStep {
                    recording: None,
                    outcome: WorkOutcome::Completed,
                },
            )],
        );
        assert!(
            matches!(result, HistoryWrite::Unconfirmed(LedgerError::Corrupt(_))),
            "{result:?}"
        );
        assert_eq!(
            ledger.by_id(&recording.id).unwrap().unwrap().source_path,
            recording.source_path
        );
        assert_eq!(ledger.recovery_view().unwrap().records.len(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_publication_clear_refuses_without_deleting_colliding_target() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;
        use vpt_application::ports::{LedgerCommit, PublicationJournal, RecordingLedger};
        use vpt_domain::run::WorkOutcome;

        let (_temp, state) = root();
        let ledger = SqliteLedger::open(&state).unwrap();
        let publication = crate::ledger::contract::publication("/h/transcripts/�.md", 96, 10);
        ledger
            .commit(&LedgerCommit {
                publications: vec![publication.clone()],
                ..Default::default()
            })
            .unwrap();
        let run = match ledger.begin_run(RunKind::Publish, UtcInstant { secs: 10 }) {
            HistoryWrite::Committed(receipt) => receipt.run,
            other => panic!("run: {other:?}"),
        };
        let invalid =
            std::path::PathBuf::from(OsString::from_vec(b"/h/transcripts/\xff.md".to_vec()));
        let result = ledger.commit_step(
            run,
            &WorkDelta {
                publication_clears: vec![invalid],
                ..Default::default()
            },
            &[(
                UtcInstant { secs: 11 },
                HistoryFact::WorkStep {
                    recording: None,
                    outcome: WorkOutcome::Completed,
                },
            )],
        );
        assert!(
            matches!(result, HistoryWrite::Unconfirmed(LedgerError::Corrupt(_))),
            "{result:?}"
        );
        assert_eq!(ledger.pending_publications().unwrap(), vec![publication]);
        assert_eq!(ledger.recovery_view().unwrap().records.len(), 1);
    }

    fn insert_run(transaction: &rusqlite::Transaction<'_>) -> Result<u64, LedgerError> {
        transaction
            .execute(
                "INSERT INTO history_runs (kind, started_at) VALUES ('ingest', 10)",
                [],
            )
            .map_err(super::super::map)?;
        u64::try_from(transaction.last_insert_rowid())
            .map_err(|_| LedgerError::Corrupt("unreadable history sequence".into()))
    }

    #[test]
    fn committed_receipt_survives_post_commit_wrapper_root_refusal() {
        let (_temp, state) = root();
        let ledger = SqliteLedger::open(&state).unwrap();
        let held = state.path().with_file_name("held");
        let result = ledger.history_write_with(
            insert_run,
            || {
                std::fs::rename(state.path(), &held).unwrap();
                std::fs::create_dir(state.path()).unwrap();
            },
            |closed| closed,
        );
        assert!(
            matches!(
                result,
                HistoryWrite::CommittedWithFailure {
                    receipt: 1,
                    error: LedgerError::PathEscape(_)
                }
            ),
            "{result:?}"
        );
        let restored = RootDir::open(&held).unwrap();
        let reopened = SqliteLedger::open(&restored).unwrap();
        assert_eq!(
            reopened
                .read(|connection| connection
                    .query_row("SELECT count(*) FROM history_runs", [], |row| row
                        .get::<_, u32>(0))
                    .map_err(super::super::map))
                .unwrap(),
            1
        );
    }

    #[test]
    fn committed_receipt_survives_post_commit_close_failure() {
        let (_temp, state) = root();
        let ledger = SqliteLedger::open(&state).unwrap();
        let result = ledger.history_write_with(
            insert_run,
            || {},
            |closed| {
                assert_eq!(closed, Ok(()));
                Err(LedgerError::Io("injected close failure".into()))
            },
        );
        assert_eq!(
            result,
            HistoryWrite::CommittedWithFailure {
                receipt: 1,
                error: LedgerError::Io("injected close failure".into())
            }
        );
        drop(ledger);
        let reopened = SqliteLedger::open(&state).unwrap();
        assert_eq!(
            reopened
                .read(|connection| connection
                    .query_row("SELECT count(*) FROM history_runs", [], |row| row
                        .get::<_, u32>(0))
                    .map_err(super::super::map))
                .unwrap(),
            1
        );
    }

    #[test]
    fn reopened_ledger_keeps_namespace_and_new_ledger_gets_distinct_ids() {
        let (_first_temp, first_root) = root();
        let first = SqliteLedger::open(&first_root).unwrap();
        let first_id = match first.begin_run(RunKind::Ingest, UtcInstant { secs: 1 }) {
            HistoryWrite::Committed(receipt) => receipt.records[0].id,
            other => panic!("first run: {other:?}"),
        };
        drop(first);
        let reopened = SqliteLedger::open(&first_root).unwrap();
        let next_id = match reopened.begin_run(RunKind::Ingest, UtcInstant { secs: 2 }) {
            HistoryWrite::Committed(receipt) => receipt.records[0].id,
            other => panic!("reopened run: {other:?}"),
        };
        assert_eq!(next_id.namespace(), first_id.namespace());
        assert!(next_id.sequence() > first_id.sequence());

        let (_second_temp, second_root) = root();
        let independent = SqliteLedger::open(&second_root).unwrap();
        let other_id = match independent.begin_run(RunKind::Ingest, UtcInstant { secs: 3 }) {
            HistoryWrite::Committed(receipt) => receipt.records[0].id,
            other => panic!("independent run: {other:?}"),
        };
        assert_ne!(other_id.namespace(), first_id.namespace());
        assert_ne!(other_id.render(), first_id.render());
    }

    #[test]
    fn failed_final_cursor_update_rolls_back_restart_and_ack_clear() {
        let (_temp, state) = root();
        let ledger = SqliteLedger::open(&state).unwrap();
        let record = match ledger.begin_run(RunKind::Ingest, UtcInstant { secs: 1 }) {
            HistoryWrite::Committed(receipt) => receipt.records[0].clone(),
            other => panic!("run: {other:?}"),
        };
        let original = ExportPosition {
            device: 4,
            inode: 5,
            end_offset: 80,
        };
        assert_eq!(
            ledger.acknowledge_export(record.id, None, original, UtcInstant { secs: 2 }),
            HistoryWrite::Committed(())
        );
        let before = ledger.recovery_view().unwrap();
        let cursor = before.active_cursor.unwrap();
        ledger.transaction(|transaction| {
            transaction.execute_batch(
                "CREATE TRIGGER abort_history_cursor_update BEFORE UPDATE OF cursor_generation ON history_identity \
                 WHEN EXISTS(SELECT 1 FROM history_export_restarts) \
                  AND EXISTS(SELECT 1 FROM history_records WHERE appended_at IS NULL) \
                 BEGIN SELECT RAISE(ABORT, 'owned final cursor fault'); END;",
            ).map_err(super::super::map)
        }).unwrap();
        let observed = ExportPosition {
            device: 4,
            inode: 6,
            end_offset: 0,
        };
        assert!(matches!(
            ledger.restart_export(cursor, observed, UtcInstant { secs: 3 }),
            HistoryWrite::Unconfirmed(_)
        ));
        let after = ledger.recovery_view().unwrap();
        assert_eq!(after.active_cursor, before.active_cursor);
        assert_eq!(after.records, before.records);
        assert!(after.restarts.is_empty());
        assert!(ledger.pending_exports().unwrap().is_empty());
    }
}
