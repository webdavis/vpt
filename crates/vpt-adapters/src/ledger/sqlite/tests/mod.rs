mod boundary;
mod contract;
mod migration;
mod observation;
mod peers;
mod retention;
mod schema;
pub(crate) mod schema_diagnostics;
mod wal_lifetime;

use super::{SqliteLedger, error, map};
use crate::RootDir;
use crate::database::{AccessPolicy, ChildGuard, Database, child_for_root, root};
use rusqlite::Connection;
use std::path::Path;
use std::time::{Duration, Instant};
use vpt_application::ports::LedgerError;

fn finish(child: ChildGuard) -> std::process::Output {
    let output = child.finish().expect("bounded ledger child");
    assert!(
        output.status.success(),
        "status={} stdout={} stderr={}",
        output.status,
        output.stdout.escape_ascii(),
        output.stderr.escape_ascii()
    );
    output
}

pub(super) fn supervised(scenario: &str) {
    let (temp, _root) = root();
    let started = Instant::now();
    let output =
        finish(child_for_root(scenario, temp.path(), std::process::id(), started).unwrap());
    if scenario == "ledger_schema_foreign_keys" {
        eprintln!(
            "VPT_TRACE parent_elapsed_ms={}",
            started.elapsed().as_millis()
        );
        for phase in schema_diagnostics::formatted_phases(&output.stderr) {
            eprintln!("VPT_TRACE {phase}");
        }
    }
}

pub(super) fn supervised_prepared(scenario: &str, prepare: impl FnOnce(&RootDir)) {
    let (temp, root) = root();
    prepare(&root);
    let started = Instant::now();
    finish(child_for_root(scenario, temp.path(), std::process::id(), started).unwrap());
}

fn database<T>(
    root: &RootDir,
    operation: impl FnOnce(&Connection) -> Result<T, LedgerError>,
) -> Result<T, LedgerError> {
    let mut owner = Database::open(
        root.try_clone().unwrap(),
        Path::new("vpt.db"),
        AccessPolicy::WritableLedger,
        Duration::ZERO,
    )
    .unwrap();
    let result = owner.with_connection(operation);
    error::finish(result, owner.try_close().map_err(LedgerError::from))
}

fn version(connection: &Connection) -> u32 {
    connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap()
}

pub(in crate::ledger::sqlite) fn remove_release_extension(
    connection: &Connection,
) -> Result<(), LedgerError> {
    connection.execute_batch("DROP TABLE artifact_expirations; DROP TABLE release_payloads; DROP TABLE release_completions; DROP INDEX release_recording_sequence; DROP INDEX release_destination; DROP INDEX release_report_path; ALTER TABLE releases DROP COLUMN report_digest;").map_err(map)
}

fn tables(connection: &Connection) -> Vec<String> {
    let mut statement = connection.prepare(
        "SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
    ).unwrap();
    statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn seed(root: &RootDir) -> SqliteLedger {
    SqliteLedger::open_with_timeout(root, Duration::from_millis(50)).expect("ledger seed")
}

pub(crate) fn run_scenario(
    scenario: &str,
    root: RootDir,
    ready: fn(),
    native: fn(&Path) -> Connection,
) {
    eprintln!("VPT_PHASE {scenario}");
    match scenario {
        s if s.starts_with("ledger_wal_lifetime_") => wal_lifetime::run(s, &root),
        s if s.starts_with("ledger_contract_history_pair_fixture_") => {
            super::history::retention::tests::pair_fixture::run(s, &root);
        }
        s if s.starts_with("ledger_contract_history_retention_fault_") => {
            super::history::retention::tests::run(s, &root);
        }
        s if s.starts_with("ledger_verification_") => super::verification::tests::run(s, &root),
        "ledger_engine_output_corrupt" => super::engine_outputs::tests::corrupt_saved(&root),
        s if s.starts_with("ledger_notes_") => super::notes::tests::run(s, &root),
        s if s.starts_with("ledger_relations_") => super::relations::tests::run(s, &root),
        s if s.starts_with("ledger_occasions_") => super::occasions::tests::run(s, &root),
        s if s.starts_with("ledger_tags_") => super::tags::tests::run(s, &root),
        s if s.starts_with("ledger_proposals_") => super::proposals::tests::run(s, &root),
        s if s.starts_with("ledger_terms_") => super::vocabulary::tests::run(s, &root),
        s if s.starts_with("ledger_flags_") => super::flags::tests::run(s, &root),
        "ledger_contract_schema_trace" => schema_diagnostics::run(&root),
        s if s.starts_with("ledger_transcript_") => super::transcript::tests::run(s, &root),
        s if s.starts_with("ledger_migration_") => migration::run(s, &root),
        s if s.starts_with("ledger_schema_") => schema::run(s, &root),
        s if s.starts_with("ledger_observe_") => observation::run(s, &root),
        s if s.starts_with("ledger_boundary_") => boundary::run(s, &root),
        s if s.starts_with("ledger_contract_sqlite_retention_") => retention::run(s, &root),
        s if s.starts_with("ledger_contract_sqlite_") => contract::run(s, &root),
        s if s.starts_with("ledger_contract_") => {
            let ledger = SqliteLedger::open(&root).expect("owned ledger");
            crate::ledger::contract::run_sqlite(s, &ledger);
        }
        s if s.starts_with("ledger_peer_") => peers::run(s, &root, ready, native),
        _ => panic!("unknown ledger scenario"),
    }
}
