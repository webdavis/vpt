use super::*;
use std::time::Duration;

#[test]
fn every_mutating_command_emits_one_versioned_success_or_human_summary() {
    for structured in [true, false] {
        let (sandbox, commands) = history_guards::prepare();
        for args in commands {
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
                Some(0),
                "{args:?}: {}",
                stderr(&output)
            );
            assert!(output.stderr.is_empty(), "{args:?}");
            assert!(!output.stdout.is_empty(), "{args:?}");
            if structured {
                let document: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
                let is_pack = args[0] == "brief" && args[1] != "--upcoming";
                assert_eq!(
                    document["schema"],
                    if is_pack {
                        "vpt.brief/1"
                    } else {
                        "vpt.result/1"
                    },
                    "{args:?}"
                );
                if !is_pack {
                    assert_eq!(
                        document["command"],
                        match args[0].as_str() {
                            "note" => "note write",
                            "retention" => "retention run",
                            name => name,
                        },
                        "{args:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn observational_commands_emit_one_versioned_document_and_preserve_all_files() {
    let (sandbox, _) = history_guards::prepare();
    let ledger = ledger(&sandbox);
    let id = ledger.recordings().unwrap().remove(0).id;
    json(&support::run_with_budget(
        sandbox.vpt().args(["redact", id.as_str(), "--json"]),
        Duration::from_secs(5),
    ));
    synthesis::append(&sandbox, "\n[handoff]\nallow_private=true\n");
    let before = fixture::snapshot(&sandbox);
    for args in history_guards::observations(&sandbox) {
        for structured in [false, true] {
            let mut command = sandbox.vpt();
            command.args(&args);
            if structured {
                command.arg("--json");
            }
            let output = support::run_with_budget(&mut command, Duration::from_secs(5));
            assert_eq!(
                output.status.code(),
                Some(0),
                "{args:?}: {}",
                stderr(&output)
            );
            assert!(output.stderr.is_empty());
            assert!(!output.stdout.is_empty());
            if structured {
                let document: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
                assert_eq!(
                    document["schema"],
                    if args[0] == "handoff" {
                        "vpt.handoff/1"
                    } else {
                        "vpt.result/1"
                    },
                    "{args:?}"
                );
                if args[0] != "handoff" {
                    assert_eq!(document["command"], args[0]);
                }
            }
            assert_eq!(fixture::snapshot(&sandbox), before, "{args:?}");
        }
    }
}
