use super::super::{SCHEMA_VERSION, SqliteLedger};
use crate::RootDir;
use crate::test_support::{root, snapshot};
use rusqlite::Connection;
use rusqlite::types::Value;
use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;

/// Files the database layer wrote at schema 20, kept as offsets and the bytes that differ from
/// zeros, or from the named file.
const WRITTEN_BY_MAIN: &str = include_str!("fixtures/main-v20.txt");
const CHECKPOINTED_ROWS: &str = include_str!("fixtures/main-v20-checkpointed-rows.txt");
const WAL_ROWS: &str = include_str!("fixtures/main-v20-wal-rows.txt");
const LEAVES: [&str; 3] = ["vpt.db", "vpt.db-wal", "vpt.db-shm"];

fn decode() -> BTreeMap<String, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut current: Option<(String, Vec<u8>)> = None;
    for line in WRITTEN_BY_MAIN.lines() {
        let words: Vec<&str> = line.split(' ').collect();
        if words[0] == "file" {
            files.extend(current.take());
            let bytes = match words.get(4) {
                Some(base) => files[*base].clone(),
                None => vec![0; words[2].parse().unwrap()],
            };
            current = Some((words[1].to_owned(), bytes));
        } else {
            let bytes = &mut current.as_mut().unwrap().1;
            let offset: usize = words[0].parse().unwrap();
            for (index, pair) in words[1].as_bytes().chunks(2).enumerate() {
                let hex = std::str::from_utf8(pair).unwrap();
                bytes[offset + index] = u8::from_str_radix(hex, 16).unwrap();
            }
        }
    }
    files.extend(current);
    files
}

fn place(set: &str, leaves: &[&str]) -> (tempfile::TempDir, RootDir) {
    let (temp, root) = root();
    let files = decode();
    for leaf in leaves {
        let path = root.path().join(leaf);
        std::fs::write(&path, &files[&format!("{set}/{leaf}")]).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    (temp, root)
}

fn rows(connection: &Connection) -> String {
    let mut out = String::new();
    let mut tables = connection
        .prepare("SELECT name FROM sqlite_schema WHERE type='table' ORDER BY name")
        .unwrap();
    let names = tables
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    for name in names {
        out.push_str(&format!("table {name}\n"));
        let mut select = connection
            .prepare(&format!("SELECT * FROM \"{name}\" ORDER BY rowid"))
            .unwrap();
        let columns = select.column_count();
        let mut cursor = select.query([]).unwrap();
        while let Some(row) = cursor.next().unwrap() {
            let values: Vec<String> = (0..columns)
                .map(|column| format!("{:?}", row.get::<_, Value>(column).unwrap()))
                .collect();
            out.push_str(&values.join("|"));
            out.push('\n');
        }
    }
    out
}

fn read_only_rows(root: &RootDir) -> String {
    let before = snapshot(root.path());
    let ledger = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
    assert_eq!(ledger.schema_version().unwrap(), SCHEMA_VERSION);
    let read = ledger.read(|connection| Ok(rows(connection))).unwrap();
    drop(ledger);
    assert_eq!(
        snapshot(root.path()),
        before,
        "a read-only open changed a file"
    );
    read
}

#[test]
fn a_checkpointed_ledger_written_by_main_reads_every_row_and_changes_nothing() {
    for leaves in [&LEAVES[..], &LEAVES[..1]] {
        let (_temp, root) = place("checkpointed", leaves);
        assert_eq!(read_only_rows(&root), CHECKPOINTED_ROWS);
        let ledger = SqliteLedger::open(&root).unwrap();
        assert_eq!(ledger.read(|c| Ok(rows(c))).unwrap(), CHECKPOINTED_ROWS);
    }
}

#[test]
fn a_ledger_written_by_main_with_rows_in_the_wal_reads_every_row_and_changes_nothing() {
    let (_temp, written) = place("wal", &LEAVES);
    assert_eq!(read_only_rows(&written), WAL_ROWS);
    drop(SqliteLedger::open(&written).unwrap());
    let (_copy_temp, copy) = root();
    std::fs::copy(written.path().join("vpt.db"), copy.path().join("vpt.db")).unwrap();
    assert_eq!(read_only_rows(&copy), WAL_ROWS);
}
