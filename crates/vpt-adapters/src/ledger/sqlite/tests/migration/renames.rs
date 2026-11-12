use super::migrations;
use rusqlite::Connection;

#[test]
fn version_fifteen_upgrade_adds_an_empty_tag_rename_table() {
    let connection = Connection::open_in_memory().unwrap();
    migrations::migrate(&connection, || Ok(())).unwrap();
    connection
        .execute_batch("DROP TABLE learned_tags; DROP TABLE tag_renames; PRAGMA user_version=15;")
        .unwrap();
    migrations::migrate(&connection, || Ok(())).unwrap();
    assert_eq!(migrations::current(&connection).unwrap(), 17);
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM tag_renames", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}
