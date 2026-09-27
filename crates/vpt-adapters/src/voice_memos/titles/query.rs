use rusqlite::{Connection, OptionalExtension};

pub(super) fn table(connection: &Connection) -> rusqlite::Result<Option<String>> {
    let mut statement =
        connection.prepare("SELECT name FROM sqlite_master WHERE type = 'table'")?;
    let names = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for name in names {
        let mut statement = connection.prepare(&format!(
            "PRAGMA table_info(\"{}\")",
            name.replace('"', "\"\"")
        ))?;
        let columns = statement
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        if columns.iter().any(|name| name == "ZPATH")
            && columns.iter().any(|name| name == "ZCUSTOMLABEL")
        {
            return Ok(Some(name));
        }
    }
    Ok(None)
}

pub(super) fn title(
    connection: &Connection,
    table: &str,
    file_name: &str,
) -> rusqlite::Result<Option<String>> {
    let sql = format!(
        "SELECT ZCUSTOMLABEL FROM \"{}\" WHERE ZPATH = ?1 COLLATE BINARY OR substr(ZPATH, -length(?1) - 1) = ('/' || ?1) COLLATE BINARY LIMIT 1",
        table.replace('"', "\"\"")
    );
    connection
        .query_row(&sql, [file_name], |row| row.get::<_, Option<String>>(0))
        .optional()
        .map(Option::flatten)
}
