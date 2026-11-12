use super::migrations;
use rusqlite::Connection;
use vpt_application::ports::LedgerError;

#[test]
fn expiration_migration_preserves_legacy_journal_and_rolls_back_on_failure() {
    let connection = Connection::open_in_memory().unwrap();
    migrations::migrate(&connection, || Ok(())).unwrap();
    connection
        .execute_batch(
            "DROP TABLE action_sends; DROP TABLE learned_tags; DROP TABLE tag_renames; DROP TABLE artifact_expirations; PRAGMA user_version=14;
        INSERT INTO retention_intents(artifact_kind,recording,path,expected,recorded_at)
        VALUES('brief','legacy occasion','/briefs/private.md','saved digest',123);",
        )
        .unwrap();
    let before = journal(&connection);
    let injected = LedgerError::Io("injected precommit failure".into());
    assert_eq!(
        migrations::migrate(&connection, || Err(injected.clone())),
        Err(injected)
    );
    assert_eq!(migrations::current(&connection).unwrap(), 14);
    assert_eq!(journal(&connection), before);
    assert!(
        !connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE name='artifact_expirations')",
                [],
                |row| row.get::<_, bool>(0)
            )
            .unwrap()
    );
    migrations::migrate(&connection, || Ok(())).unwrap();
    assert_eq!(migrations::current(&connection).unwrap(), 18);
    assert_eq!(journal(&connection), before);
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM artifact_expirations", [], |row| row
                .get::<_, i64>(
                0
            ))
            .unwrap(),
        0
    );
}

fn journal(connection: &Connection) -> (i64, String, String, String, String, i64, Option<i64>) {
    connection.query_row("SELECT id,artifact_kind,recording,path,expected,recorded_at,completed_at FROM retention_intents", [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?))).unwrap()
}
