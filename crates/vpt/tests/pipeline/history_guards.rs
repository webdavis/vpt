use super::*;
use std::time::Duration;

pub(super) fn prepare() -> (Sandbox, Vec<Vec<String>>) {
    let sandbox = configured();
    analysis::enable(&sandbox);
    add(&sandbox, "one.m4a", 1_787_604_456);
    add(&sandbox, "two.m4a", 1_787_604_476);
    json(&invoke(&sandbox, &[]));
    let ledger = ledger(&sandbox);
    let rows = ledger.recordings().unwrap();
    let id = rows[0].id.as_str();
    let relation = format!("related:{}", rows[1].id);
    let flag = ledger
        .note_inputs(&rows[0].id)
        .unwrap()
        .unwrap()
        .review
        .flags[0]
        .identifier
        .hex();
    let analysis = ledger
        .note(&rows[0].id, NoteStage::Analysis)
        .unwrap()
        .unwrap();
    let manual = vec![
        "brief",
        "--title",
        "Private occasion",
        "--at",
        "2026-10-01T10:00:00-06:00",
    ];
    json(&support::run_with_budget(
        sandbox.vpt().args(&manual).arg("--json"),
        Duration::from_secs(5),
    ));
    let occasion = ledger.occasions().unwrap().remove(0).id;
    add(&sandbox, "pending.m4a", 1_787_604_496);
    history_exports::upcoming(&sandbox);
    analysis::append(&sandbox, "\n[retention]\nenabled=true\n");
    let commands = vec![
        vec!["ingest"],
        vec!["run"],
        vec!["transcribe", id],
        vec!["note", "write", id],
        vec!["analyze", id],
        vec![
            "verify-note",
            analysis.target.to_str().unwrap(),
            "--recording",
            id,
        ],
        vec!["review", id, "--resolve", &flag, "--confirm"],
        vec!["confirm", "--term", "Private term"],
        vec!["confirm", id, "--tag", "planning"],
        vec!["confirm", id, "--reject-tag", "planning"],
        vec!["confirm", id, "--relation", &relation],
        manual,
        vec!["brief", occasion.as_str()],
        vec!["brief", "--upcoming"],
        vec!["redact", id, "--stage", "transcript"],
        vec!["redact", id, "--stage", "analysis"],
        vec!["retention", "run"],
    ]
    .into_iter()
    .map(|args| args.into_iter().map(str::to_owned).collect())
    .collect();
    (sandbox, commands)
}

fn assert_stopped(unsafe_state: bool) {
    let (sandbox, commands) = prepare();
    let ledger = ledger(&sandbox);
    let state = sandbox.path().join("state/vpt");
    if unsafe_state {
        fs::set_permissions(&state, fs::Permissions::from_mode(0o755)).unwrap();
    } else {
        sql(
            &sandbox,
            "CREATE TRIGGER refuse_run_start BEFORE INSERT ON history_runs BEGIN SELECT RAISE(ABORT,'Private required history failure'); END",
        );
    }
    let snapshot = fixture::snapshot(&sandbox);
    let records = ledger.recordings().unwrap();
    let artifacts = ledger.retention_artifacts().unwrap();
    let history = ledger.recovery_view().unwrap().records;
    let occasions = ledger.occasions().unwrap();
    for args in commands {
        for structured in [false, true] {
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
                Some(if unsafe_state { 3 } else { 1 }),
                "{args:?}: {}",
                stderr(&output)
            );
            assert!(output.stdout.is_empty());
            assert!(!stderr(&output).contains("Private"));
            if structured {
                let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
                assert_eq!(error["schema"], "vpt.error/1");
                assert_eq!(
                    error["error"]["kind"],
                    if unsafe_state { "refused" } else { "ledger" }
                );
                assert_eq!(error["error"]["completed"], serde_json::json!([]));
                if unsafe_state {
                    assert_eq!(error["error"]["rule"], "unsafe_state");
                }
            }
            assert_eq!(ledger.recordings().unwrap(), records);
            assert_eq!(ledger.retention_artifacts().unwrap(), artifacts);
            assert_eq!(ledger.recovery_view().unwrap().records, history);
            assert_eq!(ledger.occasions().unwrap(), occasions);
            assert!(ledger.pending_publications().unwrap().is_empty());
            assert_eq!(analysis::calls(&sandbox, "agent.calls"), 2);
            assert_eq!(analysis::calls(&sandbox, "calls"), 2);
            assert!(!sandbox.path().join("context.calls").exists());
            assert!(
                !sandbox
                    .config_path()
                    .with_file_name("known-terms.txt")
                    .exists()
            );
            let after = fixture::snapshot(&sandbox);
            assert_eq!(
                after
                    .iter()
                    .filter(|row| !row.is_within(&state))
                    .collect::<Vec<_>>(),
                snapshot
                    .iter()
                    .filter(|row| !row.is_within(&state))
                    .collect::<Vec<_>>(),
                "{args:?}: files outside state changed",
            );
            if unsafe_state {
                assert_eq!(fixture::snapshot(&sandbox), snapshot, "{args:?}");
            }
        }
    }
}

#[test]
fn unsafe_state_stops_every_mutating_command_without_changing_permissions_or_running_children() {
    assert_stopped(true);
}

#[test]
fn required_run_start_failure_stops_every_mutating_command_before_new_work() {
    assert_stopped(false);
}

pub(super) fn observations(sandbox: &Sandbox) -> Vec<Vec<String>> {
    let ledger = ledger(sandbox);
    let id = ledger.recordings().unwrap().remove(0).id;
    let occasion = ledger.occasions().unwrap().remove(0).id;
    [
        vec!["review", id.as_str()],
        vec!["occasions"],
        vec!["path", id.as_str(), "--stage", "transcript"],
        vec!["path", id.as_str(), "--stage", "analysis"],
        vec!["path", "--occasion", occasion.as_str(), "--stage", "brief"],
        vec!["show", id.as_str(), "--stage", "released"],
        vec!["show", id.as_str(), "--stage", "transcript"],
        vec!["show", id.as_str(), "--stage", "transcript", "--redact"],
        vec!["show", id.as_str(), "--stage", "analysis"],
        vec!["show", occasion.as_str(), "--stage", "brief"],
    ]
    .into_iter()
    .map(|args| args.into_iter().map(str::to_owned).collect())
    .collect()
}
