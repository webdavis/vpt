use super::*;

#[test]
fn a_reader_keeps_one_snapshot_while_a_native_writer_commits() {
    let (temp, _root) = root();
    let started = Instant::now();
    let spawn =
        |scenario| child_for_root(scenario, temp.path(), std::process::id(), started).unwrap();
    let mut writer = spawn("ledger_peer_snapshot_writer");
    writer.ready().unwrap();
    let mut reader = spawn("ledger_peer_snapshot_reader");
    reader.ready().unwrap();
    writer.send("continue").unwrap();
    finish(writer);
    reader.send("continue").unwrap();
    finish(reader);
}

pub(super) fn run(scenario: &str, root: &RootDir, ready: fn(), native: fn(&Path) -> Connection) {
    if scenario == "ledger_peer_snapshot_writer" {
        drop(seed(root));
        let connection = native(&root.path().join("vpt.db"));
        connection.execute_batch("PRAGMA wal_autocheckpoint=0; INSERT INTO dirty_publications VALUES('snapshot', NULL, 'committed', 1)").unwrap();
        ready();
        connection
            .execute(
                "UPDATE dirty_publications SET recorded_at=2 WHERE target='snapshot'",
                [],
            )
            .unwrap();
        connection.close().unwrap();
    } else {
        let ledger = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
        let value = |connection: &Connection| {
            connection
                .query_row(
                    "SELECT recorded_at FROM dirty_publications WHERE target='snapshot'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(map)
        };
        ledger
            .snapshot(|connection| {
                assert_eq!(value(connection)?, 1);
                ready();
                assert_eq!(value(connection)?, 1);
                Ok(())
            })
            .unwrap();
        assert_eq!(ledger.read(value).unwrap(), 2);
    }
}
