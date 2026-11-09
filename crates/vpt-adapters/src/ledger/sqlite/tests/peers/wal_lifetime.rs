use super::{Connection, Path, RootDir, child_for_root, finish, map, root, seed};
use std::time::Instant;

#[test]
fn idle_ledger_allows_native_checkpoint_and_observes_the_next_commit() {
    let (temp, _root) = root();
    let started = Instant::now();
    let spawn =
        |scenario| child_for_root(scenario, temp.path(), std::process::id(), started).unwrap();
    let mut owner = spawn("ledger_peer_idle_wal_owner");
    owner.ready().unwrap();
    finish(spawn("ledger_peer_idle_wal_native"));
    owner.send("continue").unwrap();
    finish(owner);
}

#[test]
fn a_new_ledger_recovers_committed_wal_after_the_idle_owner_is_terminated() {
    let (temp, _root) = root();
    let started = Instant::now();
    let spawn =
        |scenario| child_for_root(scenario, temp.path(), std::process::id(), started).unwrap();
    let mut owner = spawn("ledger_peer_idle_wal_owner");
    owner.ready().unwrap();
    drop(owner);
    finish(spawn("ledger_peer_idle_wal_recover"));
}

pub(super) fn run(scenario: &str, root: &RootDir, ready: fn(), native: fn(&Path) -> Connection) {
    if scenario == "ledger_peer_idle_wal_owner" {
        let ledger = seed(root);
        ledger
            .transaction(|transaction| {
                transaction
                    .execute(
                        "INSERT INTO dirty_publications VALUES ('idle', NULL, 'before', 1)",
                        [],
                    )
                    .map_err(map)?;
                Ok(())
            })
            .unwrap();
        ready();
        ledger
            .read(|connection| {
                let digest: String = connection
                    .query_row(
                        "SELECT intended FROM dirty_publications WHERE target='idle'",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(map)?;
                assert_eq!(digest, "after");
                Ok(())
            })
            .unwrap();
    } else if scenario == "ledger_peer_idle_wal_recover" {
        let ledger = seed(root);
        ledger
            .read(|connection| {
                let digest: String = connection
                    .query_row(
                        "SELECT intended FROM dirty_publications WHERE target='idle'",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(map)?;
                assert_eq!(digest, "before");
                Ok(())
            })
            .unwrap();
    } else {
        let connection = native(&root.path().join("vpt.db"));
        let checkpoint: (i32, i32, i32) = connection
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .unwrap();
        assert_eq!(
            checkpoint,
            (0, 0, 0),
            "idle ledger must not retain a reader snapshot"
        );
        connection
            .execute(
                "UPDATE dirty_publications SET intended='after' WHERE target='idle'",
                [],
            )
            .unwrap();
        connection.close().unwrap();
    }
}
