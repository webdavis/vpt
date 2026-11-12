use super::*;
use std::time::Duration;

const ENGINE: &str = "unknown additive engine field: future_engine";
const PROPOSAL: &str = "unknown additive proposal field: future_proposal";

fn prepared() -> Sandbox {
    let sandbox = configured();
    analysis::enable(&sandbox);
    add(&sandbox, "one.m4a", 1_787_604_456);
    for (file, field) in [
        ("engine.json", "future_engine"),
        ("agent.proposal", "future_proposal"),
    ] {
        let path = sandbox.path().join(file);
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        value[field] = "Private additive value".into();
        fs::write(path, value.to_string()).unwrap();
    }
    sandbox
}

fn assert_diagnostics(value: &serde_json::Value, expected: &[&str]) {
    let diagnostics = value.as_array().unwrap();
    for message in expected {
        assert_eq!(
            diagnostics
                .iter()
                .filter(|value| value.as_str().is_some_and(|text| text.contains(message)))
                .count(),
            1,
            "missing or repeated {message}: {diagnostics:?}"
        );
    }
    assert!(!value.to_string().contains("Private additive value"));
}

#[test]
fn publication_failure_keeps_diagnostics_from_the_current_and_prior_stages_once() {
    for stage in ["engine-outputs", "analysis"] {
        for structured in [true, false] {
            let sandbox = prepared();
            fixture::seed(&sandbox, Vec::new());
            sql(
                &sandbox,
                &format!(
                    "CREATE TRIGGER refuse_ack BEFORE DELETE ON dirty_publications WHEN OLD.target LIKE '%/{stage}/%' BEGIN SELECT RAISE(ABORT, 'Private acknowledgement'); END"
                ),
            );
            let mut command = sandbox.vpt();
            command.arg("run");
            if structured {
                command.arg("--json");
            }
            let output = support::run_with_budget(&mut command, Duration::from_secs(8));
            assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
            assert!(output.stdout.is_empty());
            let expected = if stage == "analysis" {
                vec![ENGINE, PROPOSAL]
            } else {
                vec![ENGINE]
            };
            if structured {
                let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
                assert_eq!(error["error"]["kind"], "ledger");
                assert_diagnostics(&error["error"]["diagnostics"], &expected);
                assert_eq!(error["error"]["completed"].as_array().unwrap().len(), 1);
            } else {
                for message in expected {
                    assert_eq!(stderr(&output).matches(message).count(), 1);
                }
            }
            assert!(!stderr(&output).contains("Private"));
            let root = sandbox.path().join("home/.vpt").join(stage);
            assert_eq!(
                ledger(&sandbox)
                    .pending_publications()
                    .unwrap()
                    .iter()
                    .filter(|publication| publication.target.parent() == Some(root.as_path()))
                    .count(),
                1
            );
        }
    }
}

#[test]
fn pipeline_preserves_engine_and_proposal_diagnostics_in_human_and_json_success() {
    for structured in [true, false] {
        let sandbox = prepared();
        let mut command = sandbox.vpt();
        command.arg("run");
        if structured {
            command.arg("--json");
        }
        let output = support::run_with_budget(&mut command, Duration::from_secs(8));
        assert!(output.status.success(), "{}", stderr(&output));
        assert!(output.stderr.is_empty());
        if structured {
            assert_diagnostics(&json(&output)["diagnostics"], &[ENGINE, PROPOSAL]);
        } else {
            for message in [ENGINE, PROPOSAL] {
                assert_eq!(stdout(&output).matches(message).count(), 1);
            }
        }
        assert!(!stdout(&output).contains("Private additive value"));
    }
}

#[test]
fn later_analysis_failure_preserves_diagnostics_from_completed_transcription() {
    let sandbox = prepared();
    fs::write(
        sandbox.path().join("agent.proposal"),
        r#"{"schema":"vpt.proposal/2"}"#,
    )
    .unwrap();
    let output = invoke(&sandbox, &[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_diagnostics(&error["error"]["diagnostics"], &[ENGINE]);
    assert_eq!(error["error"]["completed"].as_array().unwrap().len(), 1);
}

#[test]
fn stdout_failure_preserves_diagnostics_and_completed_pipeline_work() {
    let sandbox = prepared();
    let output = support::run_with_broken_stdout(
        sandbox.vpt().args(["run", "--json"]),
        Duration::from_secs(8),
    );
    assert_eq!(output.status.code(), Some(1));
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["message"], "standard output write failed");
    assert_diagnostics(&error["error"]["diagnostics"], &[ENGINE, PROPOSAL]);
    assert_eq!(error["error"]["completed"].as_array().unwrap().len(), 1);
}

#[test]
fn analysis_run_finish_failure_preserves_proposal_diagnostics_and_completed_work() {
    for structured in [false, true] {
        let sandbox = prepared();
        json(&invoke(&sandbox, &[]));
        let ledger = ledger(&sandbox);
        let id = ledger.recordings().unwrap().remove(0).id;
        sql(
            &sandbox,
            "CREATE TRIGGER refuse_analysis_finish BEFORE INSERT ON history_records WHEN NEW.fact LIKE '%run_finished%' AND (SELECT kind FROM history_runs WHERE sequence=NEW.run_sequence)='analyze' BEGIN SELECT RAISE(ABORT,'Private finish failure'); END",
        );
        let mut command = sandbox.vpt();
        command.args(["analyze", id.as_str()]);
        if structured {
            command.arg("--json");
        }
        let output = support::run_with_budget(&mut command, Duration::from_secs(5));
        assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
        assert!(output.stdout.is_empty());
        assert!(!stderr(&output).contains("Private"));
        if structured {
            let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
            assert_eq!(error["error"]["kind"], "ledger");
            assert_eq!(
                error["error"]["completed"],
                serde_json::json!([id.as_str()])
            );
            assert_diagnostics(&error["error"]["diagnostics"], &[PROPOSAL]);
        } else {
            assert!(stderr(&output).contains(id.as_str()));
            assert_eq!(stderr(&output).matches(PROPOSAL).count(), 1);
        }
        let analysis = ledger.note(&id, NoteStage::Analysis).unwrap().unwrap();
        assert_eq!(fs::read_to_string(&analysis.target).unwrap(), analysis.text);
        assert!(ledger.pending_publications().unwrap().is_empty());
    }
}
