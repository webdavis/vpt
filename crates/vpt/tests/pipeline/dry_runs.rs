#[path = "dry_runs/recovery.rs"]
mod recovery;
use super::*;
use std::time::Duration;

const MISSING_ID: &str = "2026-08-24T144736-4f3ab19c02de";
const VERBS: [&str; 6] = [
    "ingest",
    "run",
    "transcribe",
    "synthesize",
    "retention",
    "brief",
];

fn enable_commands(sandbox: &Sandbox) {
    let marker = sandbox.path().join("forbidden-command-called");
    let config = fs::read_to_string(sandbox.config_path()).unwrap();
    fs::write(sandbox.config_path(), format!("{config}\n[synthesis]\ncommand = ['/usr/bin/touch', {marker:?}]\n[report]\ncommand = ['/usr/bin/touch', {marker:?}]\n[alert]\ncommand = ['/usr/bin/touch', {marker:?}]\n[retention]\nenabled = true\ninclude_audio = true\n[retention.hold]\naudio = '1d'\n")).unwrap();
    let helper = sandbox.path().join("bin/vpt-macos");
    fs::write(
        &helper,
        format!("#!/bin/sh\n/usr/bin/touch {marker:?}\nexit 17\n"),
    )
    .unwrap();
    fs::set_permissions(helper, fs::Permissions::from_mode(0o700)).unwrap();
}

fn preview(sandbox: &Sandbox, verb: &str, id: &str, json: bool) -> std::process::Output {
    let mut command = sandbox.vpt();
    command.arg(verb);
    match verb {
        "transcribe" | "synthesize" => {
            command.arg(id);
        }
        "retention" => {
            command.arg("run");
        }
        "brief" => {
            command.args(["--title", "Private preview", "--at", "2026-10-01T10:00:00Z"]);
        }
        _ => {}
    }
    command.arg("--dry-run");
    if json {
        command.arg("--json");
    }
    let budget = match verb {
        "ingest" => Duration::from_secs(3),
        "retention" => Duration::from_millis(500),
        "run" => Duration::from_secs(8),
        _ => Duration::from_secs(5),
    };
    support::run_with_budget(&mut command, budget)
}

fn assert_preview(sandbox: &Sandbox, verb: &str, id: &str, expected: &[i32]) {
    for structured in [true, false] {
        let before = fixture::snapshot(sandbox);
        let output = preview(sandbox, verb, id, structured);
        assert!(
            output
                .status
                .code()
                .is_some_and(|code| expected.contains(&code)),
            "{verb}: {}",
            stderr(&output)
        );
        assert_eq!(
            fixture::snapshot(sandbox),
            before,
            "{verb} changed the filesystem"
        );
        if output.status.success() {
            assert!(output.stderr.is_empty(), "{verb}: {}", stderr(&output));
            if structured {
                let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
                if verb == "brief" {
                    assert_eq!(result["schema"], "vpt.brief/1");
                } else {
                    assert_eq!(result["dry_run"], true);
                    assert!(
                        result
                            .get("completed")
                            .is_none_or(|ids| *ids == serde_json::json!([]))
                    );
                }
            } else {
                assert!(!output.stdout.is_empty());
            }
        } else {
            assert!(output.stdout.is_empty());
            if structured {
                let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
                assert_eq!(error["schema"], "vpt.error/1");
                assert_eq!(error["error"]["completed"], serde_json::json!([]));
                assert!(matches!(
                    error["error"]["kind"].as_str(),
                    Some("ledger" | "usage")
                ));
            }
        }
    }
}

#[test]
fn every_dry_run_preserves_absent_state_and_absent_stores() {
    let sandbox = configured();
    enable_commands(&sandbox);
    add(&sandbox, "one.m4a", 1_787_604_456);
    for verb in VERBS {
        let code = if matches!(verb, "transcribe" | "synthesize") {
            2
        } else {
            0
        };
        assert_preview(&sandbox, verb, MISSING_ID, &[code]);
    }
    assert!(!sandbox.path().join("state/vpt").exists());
    assert!(!sandbox.path().join("home/.vpt").exists());
}

#[test]
fn every_dry_run_preserves_the_version_one_database_without_migration() {
    let sandbox = configured();
    enable_commands(&sandbox);
    let state = sandbox.path().join("state/vpt");
    fs::create_dir(&state).unwrap();
    fs::set_permissions(&state, fs::Permissions::from_mode(0o700)).unwrap();
    let database = state.join("vpt.db");
    let connection = rusqlite::Connection::open(&database).unwrap();
    connection
        .execute_batch(include_str!(
            "../../../vpt-adapters/src/ledger/sqlite/migrations/v1.sql"
        ))
        .unwrap();
    connection.pragma_update(None, "user_version", 1).unwrap();
    drop(connection);
    fs::set_permissions(&database, fs::Permissions::from_mode(0o600)).unwrap();
    for verb in VERBS {
        assert_preview(&sandbox, verb, MISSING_ID, &[0, 1, 2]);
    }
}

#[test]
fn every_dry_run_refuses_a_future_schema_without_changes() {
    let sandbox = configured();
    enable_commands(&sandbox);
    fixture::seed(&sandbox, Vec::new());
    sql(&sandbox, "PRAGMA user_version = 999");
    for verb in VERBS {
        assert_preview(&sandbox, verb, MISSING_ID, &[1]);
    }
}

#[test]
fn pipeline_dry_run_does_not_collect_enabled_automatic_brief_context() {
    let sandbox = configured();
    enable_commands(&sandbox);
    let marker = sandbox.path().join("context-called");
    let script = sandbox.path().join("bin/dam");
    fs::write(
        &script,
        format!("#!/bin/sh\n/usr/bin/touch {marker:?}\nprintf '%s' '{{\"objects\":[]}}'\n"),
    )
    .unwrap();
    fs::set_permissions(script, fs::Permissions::from_mode(0o700)).unwrap();
    let config = fs::read_to_string(sandbox.config_path()).unwrap();
    fs::write(sandbox.config_path(), format!("{config}\n[context]\ntype = 'dam'\ncalendars = ['calendar/work/']\nprojects = ['work/']\n[brief.trigger]\nenabled = true\n")).unwrap();
    assert_preview(&sandbox, "run", MISSING_ID, &[0]);
    assert!(!marker.exists());
}
