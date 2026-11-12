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
    assert_eq!(migrations::current(&connection).unwrap(), 18);
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM action_sends", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}
