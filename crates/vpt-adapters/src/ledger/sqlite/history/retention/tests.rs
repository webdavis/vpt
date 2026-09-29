use super::*;
use crate::RootDir;
use crate::ledger::contract;
use vpt_application::ports::{ArtifactKind, LedgerCommit, RetentionJournal, RunHistory, WorkDelta};
use vpt_domain::run::RunKind;

mod application;
mod application_calls;
pub(in crate::ledger::sqlite) mod pair_fixture;
mod receipts;

macro_rules! case {
    ($name:ident) => {
        #[test]
        fn $name() {
            crate::ledger::sqlite::tests::supervised(concat!(
                "ledger_contract_history_retention_fault_",
                stringify!($name)
            ));
        }
    };
}

case!(preparation_insert_failure_leaves_no_history_or_journal);
case!(preparation_history_failure_rolls_back_the_inserted_intent);
case!(audio_update_failure_leaves_the_intent_and_effect_pending);
case!(journal_update_failure_rolls_back_the_audio_mark);
case!(last_history_fact_failure_rolls_back_all_completion_writes);
case!(duplicate_intent_completion_keeps_each_original_stored_time);
case!(prepared_ids_and_receipt_survive_post_commit_close_failure);
case!(completed_id_and_receipt_survive_post_commit_root_refusal);
case!(failed_preparation_stops_the_retention_application_before_trash);
case!(failed_completion_retains_the_actual_move_and_recording_identity);
case!(post_commit_preparation_failure_retains_receipt_without_trash_or_further_history);
case!(post_commit_completion_failure_retains_move_identity_receipt_and_stops);
case!(post_commit_absent_reconciliation_failure_retains_completion_without_move);

fn committed<T: std::fmt::Debug>(result: HistoryWrite<T>) -> T {
    match result {
        HistoryWrite::Committed(value) => value,
        other => panic!("expected committed fixture: {other:?}"),
    }
}

fn new_intent() -> NewIntent {
    let record = contract::record(125);
    NewIntent {
        kind: ArtifactKind::Audio,
        recording: record.id,
        path: record.audio_path,
        expected: record.digest,
        recorded_at: UtcInstant { secs: 120 },
    }
}

fn seeded(ledger: &SqliteLedger) -> PreparedRetention {
    let run = committed(ledger.begin_run(RunKind::Retain, UtcInstant { secs: 119 })).run;
    let intent = new_intent();
    let id = ledger.record_intent(&intent).unwrap();
    let receipt = committed(ledger.commit_step(
        run,
        &WorkDelta {
            commit: LedgerCommit {
                recordings: vec![contract::record(125)],
                ..Default::default()
            },
            ..Default::default()
        },
        &[(
            intent.recorded_at,
            HistoryFact::EffectIntent {
                kind: EffectKind::RetentionTrash,
                reference: EffectReference::RetentionIntent {
                    id: u64::try_from(id).unwrap(),
                    path: intent.path,
                },
            },
        )],
    ));
    PreparedRetention {
        intent_id: id,
        effect_id: receipt.records[0].id,
        receipt,
    }
}

fn abort(ledger: &SqliteLedger, clause: &str) {
    ledger.transaction(|transaction| {
        transaction.execute_batch(&format!(
            "CREATE TRIGGER owned_retention_abort {clause} BEGIN SELECT RAISE(ABORT, 'owned retention fault'); END;"
        )).map_err(map)
    }).unwrap();
}

fn snapshot(ledger: &SqliteLedger) -> (u32, Option<i64>, Option<i64>, u32) {
    ledger
        .read(|connection| {
            connection
                .query_row(
                    "SELECT (SELECT count(*) FROM retention_intents), \
         (SELECT completed_at FROM retention_intents ORDER BY id LIMIT 1), \
         (SELECT audio_trashed_at FROM recordings LIMIT 1), \
         (SELECT count(*) FROM history_records)",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .map_err(map)
        })
        .unwrap()
}

pub(in crate::ledger::sqlite) fn run(scenario: &str, root: &RootDir) {
    let name = scenario
        .strip_prefix("ledger_contract_history_retention_fault_")
        .unwrap();
    let ledger = SqliteLedger::open(root).unwrap();
    match name {
        "preparation_insert_failure_leaves_no_history_or_journal"
        | "preparation_history_failure_rolls_back_the_inserted_intent" => {
            let start = committed(ledger.begin_run(RunKind::Retain, UtcInstant { secs: 119 }));
            let clause = if name == "preparation_insert_failure_leaves_no_history_or_journal" {
                "BEFORE INSERT ON retention_intents"
            } else {
                "BEFORE INSERT ON history_records WHEN EXISTS(SELECT 1 FROM retention_intents)"
            };
            abort(&ledger, clause);
            let result = ledger.prepare_retention(start.run, &new_intent());
            assert_eq!(
                result,
                HistoryWrite::Unconfirmed(LedgerError::Conflict(
                    "database constraint violated".into()
                ))
            );
            assert_eq!(snapshot(&ledger), (0, None, None, 1));
        }
        "audio_update_failure_leaves_the_intent_and_effect_pending"
        | "journal_update_failure_rolls_back_the_audio_mark"
        | "last_history_fact_failure_rolls_back_all_completion_writes" => {
            let prepared = seeded(&ledger);
            let clause = match name {
                "audio_update_failure_leaves_the_intent_and_effect_pending" => {
                    "BEFORE UPDATE OF audio_trashed_at ON recordings"
                }
                "journal_update_failure_rolls_back_the_audio_mark" => {
                    "BEFORE UPDATE OF completed_at ON retention_intents WHEN (SELECT audio_trashed_at FROM recordings LIMIT 1) IS NOT NULL"
                }
                _ => {
                    "BEFORE INSERT ON history_records WHEN json_extract(NEW.fact, '$.tag') = 'work' AND (SELECT completed_at FROM retention_intents LIMIT 1) IS NOT NULL"
                }
            };
            abort(&ledger, clause);
            let result = ledger.complete_retention(
                prepared.receipt.run,
                prepared.intent_id,
                prepared.effect_id,
                UtcInstant { secs: 130 },
            );
            assert_eq!(
                result,
                HistoryWrite::Unconfirmed(LedgerError::Conflict(
                    "database constraint violated".into()
                ))
            );
            assert_eq!(snapshot(&ledger), (1, None, None, 2));
        }
        "duplicate_intent_completion_keeps_each_original_stored_time" => {
            receipts::duplicate_times(&ledger);
        }
        "prepared_ids_and_receipt_survive_post_commit_close_failure" => {
            receipts::prepared_after_close(&ledger);
        }
        "completed_id_and_receipt_survive_post_commit_root_refusal" => {
            receipts::completed_after_root(&ledger, root);
        }
        "failed_preparation_stops_the_retention_application_before_trash" => {
            application::preparation(&ledger, root);
        }
        "failed_completion_retains_the_actual_move_and_recording_identity" => {
            application::completion(&ledger, root);
        }
        "post_commit_preparation_failure_retains_receipt_without_trash_or_further_history" => {
            application::post_commit::preparation(&ledger, root);
        }
        "post_commit_completion_failure_retains_move_identity_receipt_and_stops" => {
            application::post_commit::completion(&ledger, root);
        }
        "post_commit_absent_reconciliation_failure_retains_completion_without_move" => {
            application::post_commit::absent(&ledger, root);
        }
        _ => panic!("unknown retention history fault scenario"),
    }
}
