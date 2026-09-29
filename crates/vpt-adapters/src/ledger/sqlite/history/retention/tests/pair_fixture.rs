use super::*;
use crate::ledger::MemoryLedger;
use crate::ledger::contract::retention_history::{
    PendingPair, RetentionPairFixture, pair_through_ports,
};
use vpt_application::ports::{PublicationJournal, RecordingLedger};

impl RetentionPairFixture for SqliteLedger {
    fn seed_retention_pair(&self) -> PendingPair {
        committed(self.history_write(|transaction| {
            let namespace = storage::namespace(transaction)?;
            transaction
                .execute(
                    "INSERT INTO history_runs (kind, started_at) VALUES ('retain', 119)",
                    [],
                )
                .map_err(map)?;
            let run = RunId::new(
                namespace,
                storage::positive(transaction.last_insert_rowid())?,
            );
            storage::record(
                transaction,
                run,
                UtcInstant { secs: 119 },
                HistoryFact::RunStarted(RunKind::Retain),
            )?;
            let new = contract::retention_history::intent();
            let intents = [
                journal::insert(transaction, &new)?,
                journal::insert(transaction, &new)?,
            ];
            crate::ledger::sqlite::recordings::apply_commit(
                transaction,
                &LedgerCommit {
                    recordings: vec![contract::record(120)],
                    ..Default::default()
                },
            )?;
            let effect = |id| {
                storage::record(
                    transaction,
                    run,
                    new.recorded_at,
                    HistoryFact::EffectIntent {
                        kind: EffectKind::RetentionTrash,
                        reference: EffectReference::RetentionIntent {
                            id: storage::positive(id)?,
                            path: new.path.clone(),
                        },
                    },
                )
                .map(|record| record.id)
            };
            Ok(PendingPair {
                run,
                intents,
                effects: [effect(intents[0])?, effect(intents[1])?],
            })
        }))
    }
}

#[test]
fn batched_pair_matches_public_port_state() {
    crate::ledger::sqlite::tests::supervised("ledger_contract_history_pair_fixture_parity");
}

#[test]
fn batched_pair_preserves_namespace_and_run_metadata() {
    crate::ledger::sqlite::tests::supervised("ledger_contract_history_pair_fixture_identity");
}

pub(in crate::ledger::sqlite) fn run(scenario: &str, root: &RootDir) {
    let ledger = SqliteLedger::open(root).unwrap();
    match scenario {
        "ledger_contract_history_pair_fixture_parity" => parity(&ledger),
        "ledger_contract_history_pair_fixture_identity" => identity(&ledger),
        _ => panic!("unknown retention-pair fixture scenario"),
    }
}

fn parity(ledger: &SqliteLedger) {
    let memory = MemoryLedger::new();
    let mut expected = pair_through_ports(&memory);
    let actual = ledger.seed_retention_pair();
    let namespace = actual.run.namespace();
    expected.run = RunId::new(namespace, expected.run.sequence());
    expected.effects = expected
        .effects
        .map(|event| EventId::new(namespace, event.sequence()));
    assert_eq!(actual, expected);
    assert_eq!(ledger.recordings().unwrap(), memory.recordings().unwrap());
    assert_eq!(ledger.seen_all().unwrap(), memory.seen_all().unwrap());
    assert_eq!(
        ledger.pending_publications().unwrap(),
        memory.pending_publications().unwrap()
    );
    assert_eq!(
        ledger.pending_intents().unwrap(),
        memory.pending_intents().unwrap()
    );
    let mut history = memory.recovery_view().unwrap();
    for record in &mut history.records {
        record.id = EventId::new(namespace, record.id.sequence());
        record.run = RunId::new(namespace, record.run.sequence());
    }
    assert_eq!(ledger.recovery_view().unwrap(), history);
}

fn identity(ledger: &SqliteLedger) {
    let namespace = ledger.read(storage::namespace).unwrap();
    let pair = ledger.seed_retention_pair();
    assert_eq!(pair.run.namespace(), namespace);
    let stored = ledger
        .read(|connection| {
            let namespace = storage::namespace(connection)?;
            let run: (String, i64) = connection
                .query_row(
                    "SELECT kind, started_at FROM history_runs WHERE sequence = ?1",
                    [storage::stored(pair.run.sequence())?],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .map_err(map)?;
            Ok((namespace, run))
        })
        .unwrap();
    assert_eq!(stored, (namespace, ("retain".into(), 119)));
}
