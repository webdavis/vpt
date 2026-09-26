use super::finish;
use crate::database::tests::children::child_for_root;
use std::path::Path;
use std::time::Instant;

pub(super) fn run(scenario: &str, directory: &Path, started: Instant) {
    let spawn = |name| child_for_root(name, directory, std::process::id(), started).unwrap();
    let phase = |name| {
        eprintln!(
            "VPT_PHASE {name} elapsed_ms={}",
            started.elapsed().as_millis()
        )
    };
    match scenario {
        "wal_native_writer" | "wal_native_contention" => {
            let mut native = spawn("native_wal_writer");
            native.ready().unwrap();
            finish(spawn(scenario));
            native.send("continue").unwrap();
            finish(native);
        }
        "wal_native_reader" => {
            let mut rooted = spawn("rooted_snapshot_writer");
            rooted.ready().unwrap();
            let mut native = spawn("native_reader");
            native.ready().unwrap();
            rooted.send("continue").unwrap();
            rooted.ready().unwrap();
            native.send("continue").unwrap();
            finish(native);
            rooted.send("continue").unwrap();
            finish(rooted);
        }
        "wal_rooted_contention" => {
            let mut rooted = spawn("rooted_wal_writer");
            rooted.ready().unwrap();
            let mut native = spawn("native_wal_contender");
            native.ready().unwrap();
            native.send("continue").unwrap();
            finish(native);
            rooted.send("continue").unwrap();
            finish(rooted);
        }
        "wal_crash_uncommitted" | "wal_crash_committed" | "wal_recovery_observation" => {
            phase("child-seed-start");
            let participant = match scenario {
                "wal_crash_committed" => "rooted_committed",
                "wal_recovery_observation" => "rooted_rollback",
                _ => "rooted_uncommitted",
            };
            let mut process = spawn(participant);
            process.ready().unwrap();
            phase("child-seed-ready");
            drop(process);
            finish(spawn(scenario));
        }
        _ => finish(spawn(scenario)),
    }
}
