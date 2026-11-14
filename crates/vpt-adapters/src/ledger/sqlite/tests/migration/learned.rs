use super::migrations;
use rusqlite::Connection;

#[test]
fn version_sixteen_upgrade_adds_an_empty_learned_tag_table() {
    let connection = Connection::open_in_memory().unwrap();
    migrations::migrate(&connection, || Ok(())).unwrap();
    connection
        .execute_batch("DROP TABLE action_sends; DROP TABLE learned_tags; PRAGMA user_version=16;")
        .unwrap();
    migrations::migrate(&connection, || Ok(())).unwrap();
    assert_eq!(migrations::current(&connection).unwrap(), 19);
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM learned_tags", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}
