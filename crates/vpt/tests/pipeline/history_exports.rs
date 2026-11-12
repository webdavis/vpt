use super::*;
use std::time::Duration;
use vpt_domain::run::{HistoryFact, RunKind, RunOutcome};

const WARNING: &str = "event log export is unavailable";

#[test]
fn transcription_startup_refusal_retains_export_warning() {
    startup_publication_refusal("transcribe");
}

#[test]
fn retention_startup_refusal_retains_export_warning() {
    startup_publication_refusal("retention");
}

fn startup_publication_refusal(verb: &str) {
    for structured in [false, true] {
        let sandbox = configured();
        add(&sandbox, "one.m4a", 1_787_604_456);
        json(&invoke(&sandbox, &[]));
        analysis::append(&sandbox, "\n[retention]\nenabled=true\n");
        let root = vpt_adapters::RootDir::open(&sandbox.path().join("state/vpt")).unwrap();
        let ledger = SqliteLedger::open(&root).unwrap();
        let row = ledger.recordings().unwrap().remove(0);
        let note = ledger
            .note(&row.id, NoteStage::Transcript)
            .unwrap()
            .unwrap();
        ledger
            .commit(&LedgerCommit {
                publications: vec![DirtyPublication {
                    target: note.target.clone(),
                    expected_previous: Some(note.digest),
                    intended: note.digest,
                    recorded_at: vpt_adapters::SystemClock.now(),
                }],
                ..Default::default()
            })
            .unwrap();
        fs::write(&note.target, "Private operator replacement").unwrap();
        let log = sandbox.path().join("state/vpt/events.jsonl");
        fs::remove_file(&log).unwrap();
        fs::create_dir(&log).unwrap();
        let args = if verb == "transcribe" {
            vec![verb, row.id.as_str()]
        } else {
            vec![verb, "run"]
        };
        let output = invoke_later(&sandbox, &args, structured);
        assert_eq!(output.status.code(), Some(3), "{verb}: {}", stderr(&output));
        assert!(output.stdout.is_empty());
        assert!(!stderr(&output).contains("Private"));
        assert_eq!(
            stderr(&output).matches(WARNING).count(),
            1,
            "{verb}: {}",
            stderr(&output)
        );
        if structured {
            let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
            assert_eq!(error["error"]["rule"], "publication_target_modified");
            assert_eq!(error["error"]["completed"], serde_json::json!([]));
        }
        assert_eq!(
            fs::read_to_string(&note.target).unwrap(),
            "Private operator replacement"
        );
        assert_eq!(ledger.pending_publications().unwrap().len(), 1);
        assert_eq!(analysis::calls(&sandbox, "calls"), 1);
    }
}

fn invoke_later(sandbox: &Sandbox, args: &[&str], structured: bool) -> std::process::Output {
    let mut command = sandbox.vpt();
    command.args(args);
    if structured {
        command.arg("--json");
    }
    support::run_with_budget(&mut command, Duration::from_secs(5))
}

pub(super) fn upcoming(sandbox: &Sandbox) {
    let now = vpt_adapters::SystemClock.now().secs;
    let response = sandbox.path().join("calendar.json");
    fs::write(
        &response,
        support::dam::agenda(vec![support::dam::event(
            "export-brief",
            "Private occasion",
            now + 3600,
            now + 5400,
        )]),
    )
    .unwrap();
    let script = sandbox.path().join("collector.sh");
    let calls = sandbox.path().join("context.calls");
    fs::write(
        &script,
        format!(
            "printf 'called\\n' >> {calls:?}\n{}",
            support::dam::script(&response)
        ),
    )
    .unwrap();
    analysis::append(
        sandbox,
        &format!(
            "\n[brief.trigger]\nenabled=true\n[context]\ntype='dam'\ncalendars=['calendar/work/']\nprojects=['work/']\n[context.dam]\ncommand=['/bin/sh', {script:?}]\n"
        ),
    );
}

#[test]
fn required_brief_history_precedes_store_creation() {
    let sandbox = configured();
    add(&sandbox, "one.m4a", 1_787_604_456);
    json(&invoke(&sandbox, &[]));
    let briefs = sandbox.path().join("home/.vpt/briefs");
    fs::remove_dir(&briefs).unwrap();
    sql(
        &sandbox,
        "CREATE TRIGGER refuse_required_history BEFORE INSERT ON history_records BEGIN SELECT RAISE(ABORT,'private failure'); END",
    );
    let output = invoke_later(
        &sandbox,
        &[
            "brief",
            "--title",
            "Private occasion",
            "--at",
            "2026-10-01T10:00:00-06:00",
        ],
        true,
    );
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(
        !briefs.exists(),
        "store created before required run history"
    );
}

#[test]
fn required_brief_history_precedes_manual_and_upcoming_collection() {
    for automatic in [false, true] {
        let sandbox = configured();
        add(&sandbox, "one.m4a", 1_787_604_456);
        json(&invoke(&sandbox, &[]));
        upcoming(&sandbox);
        sql(
            &sandbox,
            "CREATE TRIGGER refuse_required_history BEFORE INSERT ON history_records BEGIN SELECT RAISE(ABORT,'private failure'); END",
        );
        let args: &[&str] = if automatic {
            &["brief", "--upcoming"]
        } else {
            &[
                "brief",
                "--title",
                "Private occasion",
                "--at",
                "2026-10-01T10:00:00-06:00",
            ]
        };
        let output = invoke_later(&sandbox, args, true);
        assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
        assert!(
            !sandbox.path().join("context.calls").exists(),
            "context collected before required run history (upcoming={automatic})"
        );
    }
}

#[test]
fn later_commands_preserve_success_and_export_every_history_record_when_logging_recovers() {
    for verb in ["analyze", "verify-note", "redact", "brief", "upcoming"] {
        for structured in [false, true] {
            let sandbox = configured();
            analysis::enable(&sandbox);
            add(&sandbox, "one.m4a", 1_787_604_456);
            json(&invoke(&sandbox, &[]));
            let ledger = ledger(&sandbox);
            let row = ledger.recordings().unwrap().remove(0);
            let analysis = ledger.note(&row.id, NoteStage::Analysis).unwrap().unwrap();
            if verb == "upcoming" {
                upcoming(&sandbox);
            }
            let before = ledger.recovery_view().unwrap().records.len();
            let args = match verb {
                "verify-note" => vec![
                    verb,
                    analysis.target.to_str().unwrap(),
                    "--recording",
                    row.id.as_str(),
                ],
                "brief" => vec![
                    verb,
                    "--title",
                    "Private occasion",
                    "--at",
                    "2026-10-01T10:00:00-06:00",
                ],
                "upcoming" => vec!["brief", "--upcoming"],
                _ => vec![verb, row.id.as_str()],
            };
            let artifacts_before = ledger.retention_artifacts().unwrap();
            sql(
                &sandbox,
                "CREATE TRIGGER refuse_required_history BEFORE INSERT ON history_records BEGIN SELECT RAISE(ABORT,'Private history canary'); END",
            );
            let refused = invoke_later(&sandbox, &args, structured);
            assert_eq!(
                refused.status.code(),
                Some(1),
                "{verb}: {}",
                stderr(&refused)
            );
            assert!(refused.stdout.is_empty());
            assert!(!stderr(&refused).contains("Private history canary"));
            if structured {
                let error: serde_json::Value = serde_json::from_slice(&refused.stderr).unwrap();
                assert_eq!(error["error"]["kind"], "ledger");
                assert_eq!(error["error"]["completed"], serde_json::json!([]));
            }
            assert_eq!(ledger.recovery_view().unwrap().records.len(), before);
            assert_eq!(ledger.retention_artifacts().unwrap(), artifacts_before);
            assert_eq!(fs::read_to_string(&analysis.target).unwrap(), analysis.text);
            assert_eq!(analysis::calls(&sandbox, "agent.calls"), 1);
            assert!(ledger.occasions().unwrap().is_empty());
            assert!(ledger.releases(&row.id).unwrap().is_empty());
            sql(&sandbox, "DROP TRIGGER refuse_required_history");

            let log = sandbox.path().join("state/vpt/events.jsonl");
            fs::remove_file(&log).unwrap();
            fs::create_dir(&log).unwrap();
            let output = invoke_later(&sandbox, &args, structured);
            assert_eq!(output.status.code(), Some(0), "{verb}: {}", stderr(&output));
            assert!(output.stderr.is_empty());
            assert_eq!(stdout(&output).matches(WARNING).count(), 1, "{verb}");
            if structured {
                let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
                assert_eq!(result["warnings"], serde_json::json!([WARNING]));
            }
            assert!(row.audio_path.is_file());
            assert!(ledger.pending_publications().unwrap().is_empty());
            let kind = match verb {
                "analyze" => {
                    assert_eq!(analysis::calls(&sandbox, "agent.calls"), 2);
                    RunKind::Analyze
                }
                "verify-note" => RunKind::VerifyNote,
                "redact" => {
                    assert_eq!(ledger.releases(&row.id).unwrap().len(), 1);
                    RunKind::Redact
                }
                _ => {
                    let occasion = ledger.occasions().unwrap().remove(0);
                    assert!(occasion.briefed_at.is_some());
                    let brief = ledger.brief(&occasion.id).unwrap().unwrap();
                    assert_eq!(fs::read_to_string(&brief.target).unwrap(), brief.text);
                    RunKind::Brief
                }
            };
            let history = ledger.recovery_view().unwrap();
            let new = &history.records[before..];
            let run = new
                .iter()
                .find(|record| record.fact == HistoryFact::RunStarted(kind))
                .unwrap()
                .run;
            assert!(new.iter().any(|record| record.run == run
                && record.fact == HistoryFact::RunFinished(RunOutcome::Completed)));
            let pending = ledger.pending_exports().unwrap();
            assert!(!pending.is_empty());
            assert!(
                new.iter()
                    .all(|record| pending.iter().any(|pending| pending.id == record.id))
            );

            fs::remove_dir(&log).unwrap();
            json(&invoke(&sandbox, &[]));
            assert!(ledger.pending_exports().unwrap().is_empty());
            assert_eq!(
                fs::metadata(&log).unwrap().permissions().mode() & 0o777,
                0o600
            );
            let exported: Vec<serde_json::Value> = fs::read_to_string(&log)
                .unwrap()
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect();
            for record in pending {
                let matches: Vec<_> = exported
                    .iter()
                    .filter(|value| value["event_id"] == record.id.render())
                    .collect();
                assert_eq!(matches.len(), 1, "{verb}: missing or duplicate export");
                assert_eq!(matches[0]["occurred_at"], record.occurred_at.rfc3339());
            }
        }
    }
}
