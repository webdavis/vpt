use super::*;
use crate::actions::{configure_with, sent};
use crate::withdraw::{analyze, attempts, counts, keep_first_two, withdraw_setting};
use serde_json::json;

const V18: &str = include_str!("../../../vpt-adapters/src/ledger/sqlite/migrations/v18.sql");

fn rewrite_as_schema_eighteen(sandbox: &Sandbox, id: &str) {
    let connection = rusqlite::Connection::open(sandbox.path().join("state/vpt/vpt.db")).unwrap();
    connection
        .execute_batch(&format!(
            "PRAGMA foreign_keys=OFF;
             ALTER TABLE action_sends RENAME TO action_sends_v19;
             {V18}
             INSERT INTO action_sends (recording, id, digest, state)
             SELECT recording, id, digest, state FROM action_sends_v19;
             DROP TABLE action_sends_v19;
             UPDATE action_sends SET state = 'pending' WHERE id = '{id}@00:00#3';
             UPDATE action_sends SET state = 'refused' WHERE id = (
                 SELECT min(id) FROM action_sends WHERE id NOT IN ('{id}#1', '{id}@00:00#3'));
             PRAGMA user_version=18;"
        ))
        .unwrap();
    fixture::preserve_wal(&connection);
}

fn states(sandbox: &Sandbox) -> (i64, Vec<String>) {
    let connection = rusqlite::Connection::open(sandbox.path().join("state/vpt/vpt.db")).unwrap();
    let version = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    let mut statement = connection
        .prepare("SELECT state FROM action_sends ORDER BY id")
        .unwrap();
    let states = statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    fixture::preserve_wal(&connection);
    (version, states)
}

#[test]
fn a_schema_eighteen_ledger_withdraws_its_dropped_sent_and_pending_actions_once() {
    let (sandbox, mut rows) = note::configured_saying(&[7], "um private words tomorrow");
    let id = rows.remove(0).id.as_str().to_owned();
    let log = configure_with(&sandbox, "");
    assert_eq!(
        analyze(&sandbox, &id)["actions"],
        json!({"sent": 4, "failed": 0, "refused": 0})
    );
    rewrite_as_schema_eighteen(&sandbox, &id);
    let (setting, withdrawn) = withdraw_setting(&sandbox);
    configure_with(&sandbox, &setting);
    keep_first_two(&sandbox);
    assert_eq!(analyze(&sandbox, &id)["actions"], counts(0, 0, 0, 2));
    let ids: Vec<_> = sent(&withdrawn)
        .into_iter()
        .map(|args| args[2].clone())
        .collect();
    assert_eq!(ids, [format!("{id}#1"), format!("{id}@00:00#3")]);
    assert_eq!(
        states(&sandbox),
        (19, vec!["refused".into(), "sent".into()])
    );
    assert_eq!(analyze(&sandbox, &id)["actions"], counts(0, 0, 0, 0));
    assert_eq!(attempts(&withdrawn), 2);
    assert_eq!(attempts(&log), 4);
}
