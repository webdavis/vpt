use super::super::SqliteLedger;
use super::{finish, map, seed};
use crate::RootDir;
use crate::database::{child_for_root, root, snapshot};
use rusqlite::Connection;
use std::path::Path;
use std::time::{Duration, Instant};
use vpt_application::ports::LedgerError;
mod snapshot;
mod wal_lifetime;

#[test]
fn a_native_write_lock_returns_busy_within_the_shared_watchdog_budget() {
    let (temp, _root) = root();
    let started = Instant::now();
    let spawn =
        |scenario| child_for_root(scenario, temp.path(), std::process::id(), started).unwrap();
    let mut contender = spawn("ledger_peer_contender");
    contender.ready().unwrap();
    let mut holder = spawn("ledger_peer_holder");
    holder.ready().unwrap();
    contender.send("continue").unwrap();
    finish(contender);
    holder.send("continue").unwrap();
    finish(holder);
}

#[test]
fn observation_reads_committed_rows_from_a_native_live_log_without_changes() {
    let (temp, _root) = root();
    let started = Instant::now();
    let spawn =
        |scenario| child_for_root(scenario, temp.path(), std::process::id(), started).unwrap();
    let mut writer = spawn("ledger_peer_live_writer");
    writer.ready().unwrap();
    finish(spawn("ledger_peer_live_reader"));
    writer.send("continue").unwrap();
    finish(writer);
}

pub(super) fn run(scenario: &str, root: &RootDir, ready: fn(), native: fn(&Path) -> Connection) {
    if scenario.starts_with("ledger_peer_idle_wal_") {
        wal_lifetime::run(scenario, root, ready, native);
        return;
    }
    if scenario.starts_with("ledger_peer_snapshot_") {
        snapshot::run(scenario, root, ready, native);
        return;
    }
    match scenario {
        "ledger_peer_contender" => {
            let ledger = seed(root);
            ready();
            eprintln!("VPT_PHASE ledger-busy-transaction");
            let started = Instant::now();
            let result = ledger.transaction(|t| {
                t.execute("DELETE FROM dirty_publications", [])
                    .map(|_| ())
                    .map_err(map)
            });
            assert_eq!(result, Err(LedgerError::Busy));
            assert!(started.elapsed() >= Duration::from_millis(40));
            assert!(started.elapsed() < Duration::from_millis(500));
        }
        "ledger_peer_holder" | "ledger_peer_live_writer" => {
            if scenario == "ledger_peer_live_writer" {
                drop(seed(root));
            }
            let connection = native(&root.path().join("vpt.db"));
            if scenario == "ledger_peer_holder" {
                connection.execute_batch("BEGIN IMMEDIATE").unwrap();
            } else {
                connection.execute_batch("PRAGMA wal_autocheckpoint=0; INSERT INTO dirty_publications VALUES('live', NULL, 'committed', 42)").unwrap();
            }
            ready();
            if scenario == "ledger_peer_holder" {
                connection.execute_batch("ROLLBACK").unwrap();
            }
            connection.close().unwrap();
        }
        "ledger_peer_live_reader" => {
            let before = snapshot(root.path());
            let ledger = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
            ledger
                .read(|c| {
                    assert_eq!(
                        c.query_row(
                            "SELECT recorded_at FROM dirty_publications WHERE target='live'",
                            [],
                            |row| row.get::<_, i64>(0)
                        )
                        .unwrap(),
                        42
                    );
                    assert_eq!(snapshot(root.path()), before);
                    Ok(())
                })
                .unwrap();
            drop(ledger);
            assert_eq!(snapshot(root.path()), before);
        }
        _ => panic!("unknown peer scenario"),
    }
}
