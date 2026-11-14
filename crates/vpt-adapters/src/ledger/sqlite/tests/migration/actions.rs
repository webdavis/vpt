use super::migrations;
use rusqlite::Connection;

#[test]
fn version_seventeen_upgrade_adds_an_empty_action_send_table() {
    let connection = Connection::open_in_memory().unwrap();
    migrations::migrate(&connection, || Ok(())).unwrap();
    connection
        .execute_batch("DROP TABLE action_sends; PRAGMA user_version=17;")
        .unwrap();
    migrations::migrate(&connection, || Ok(())).unwrap();
    assert_eq!(migrations::current(&connection).unwrap(), 19);
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM action_sends", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn version_eighteen_upgrade_counts_every_saved_action_as_attempted() {
    let connection = Connection::open_in_memory().unwrap();
    migrations::migrate(&connection, || Ok(())).unwrap();
    let recording = crate::ledger::contract::record(94);
    connection
        .execute_batch(&format!(
            "PRAGMA foreign_keys=OFF; DROP TABLE action_sends; {v18} PRAGMA user_version=18;
             INSERT INTO action_sends (recording, id, digest, state) VALUES
             ('{id}', 'sent', '{digest}', 'sent'),
             ('{id}', 'pending', '{digest}', 'pending'),
             ('{id}', 'refused', '{digest}', 'refused');",
            id = recording.id.as_str(),
            digest = "a".repeat(64),
            v18 = include_str!("../../migrations/v18.sql"),
        ))
        .unwrap();
    migrations::migrate(&connection, || Ok(())).unwrap();
    assert_eq!(migrations::current(&connection).unwrap(), 19);
    let mut statement = connection
        .prepare("SELECT id, attempted, state FROM action_sends ORDER BY id")
        .unwrap();
    let rows: Vec<(String, bool, String)> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(
        rows,
        [
            ("pending".into(), true, "pending".into()),
            ("refused".into(), true, "refused".into()),
            ("sent".into(), true, "sent".into()),
        ]
    );
    connection
        .execute(
            "UPDATE action_sends SET state = 'withdrawing' WHERE id = 'sent'",
            [],
        )
        .unwrap();
}
