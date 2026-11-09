use super::*;
use std::{os::unix::fs::symlink, time::Duration};

#[test]
fn a_linked_main_database_stops_every_command_without_touching_its_referent() {
    assert_link_refused("vpt.db", false);
}
#[test]
fn a_multiply_linked_main_database_stops_every_command_without_touching_its_referent() {
    assert_link_refused("vpt.db", true);
}
#[test]
fn a_linked_wal_stops_every_command_without_touching_its_referent() {
    assert_link_refused("vpt.db-wal", false);
}
#[test]
fn a_multiply_linked_wal_stops_every_command_without_touching_its_referent() {
    assert_link_refused("vpt.db-wal", true);
}
#[test]
fn a_linked_shared_memory_file_stops_every_command_without_touching_its_referent() {
    assert_link_refused("vpt.db-shm", false);
}
#[test]
fn a_multiply_linked_shared_memory_file_stops_every_command_without_touching_its_referent() {
    assert_link_refused("vpt.db-shm", true);
}
#[test]
fn a_linked_journal_stops_every_command_without_touching_its_referent() {
    assert_link_refused("vpt.db-journal", false);
}
#[test]
fn a_multiply_linked_journal_stops_every_command_without_touching_its_referent() {
    assert_link_refused("vpt.db-journal", true);
}

fn assert_link_refused(leaf: &str, hard_link: bool) {
    let (sandbox, mutations) = history_guards::prepare();
    let observations = history_guards::observations(&sandbox);
    synthesis::append(&sandbox, "\n[handoff]\nallow_private=true\n");
    let state = sandbox.path().join("state/vpt");
    let path = state.join(leaf);
    let held = sandbox.path().join("operator-private-file");
    if path.exists() {
        fs::rename(&path, &held).unwrap();
    } else {
        fs::write(&held, b"Private journal canary").unwrap();
    }
    fs::set_permissions(&held, fs::Permissions::from_mode(0o644)).unwrap();
    if hard_link {
        fs::hard_link(&held, &path).unwrap();
    } else {
        symlink(&held, &path).unwrap();
    }
    let before = fixture::snapshot(&sandbox);
    for (args, observational) in mutations
        .into_iter()
        .map(|args| (args, false))
        .chain(observations.into_iter().map(|args| (args, true)))
    {
        for structured in [false, true] {
            let immediate_before = fixture::snapshot(&sandbox);
            let mut command = sandbox.vpt();
            command.args(&args);
            if structured {
                command.arg("--json");
            }
            let output = support::run_with_budget(
                &mut command,
                Duration::from_secs(if args[0] == "run" { 8 } else { 5 }),
            );
            assert_eq!(
                output.status.code(),
                Some(3),
                "{leaf}, {hard_link}, {args:?}: {}",
                stderr(&output)
            );
            assert!(output.stdout.is_empty());
            assert!(!stderr(&output).contains("Private"));
            if structured {
                let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
                assert_eq!(error["schema"], "vpt.error/1");
                assert_eq!(error["error"]["rule"], "path_escape", "{args:?}: {error}");
                assert_eq!(error["error"]["completed"], serde_json::json!([]));
            }
            let after = fixture::snapshot(&sandbox);
            assert_eq!(
                after
                    .iter()
                    .filter(|row| !row.is_within(&state))
                    .collect::<Vec<_>>(),
                before
                    .iter()
                    .filter(|row| !row.is_within(&state))
                    .collect::<Vec<_>>(),
                "{leaf}, {hard_link}, {args:?}"
            );
            if observational {
                assert_eq!(
                    after, immediate_before,
                    "{args:?}: observation changed files"
                );
            }
        }
    }
}
