use super::*;

#[test]
fn accepted_proposal_diagnostics_survive_commit_and_publication_failures() {
    for publish in [false, true] {
        for structured in [true, false] {
            let (sandbox, record) = note::configured();
            agent(&sandbox, "");
            let path = sandbox.path().join("captured.proposal");
            let mut value: serde_json::Value =
                serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            value["future_proposal"] = "Private additive value".into();
            fs::write(path, value.to_string()).unwrap();
            workflow::sql(
                &sandbox,
                if publish {
                    "CREATE TRIGGER refuse_analysis_ack BEFORE DELETE ON dirty_publications WHEN OLD.target LIKE '%/analysis/%' BEGIN SELECT RAISE(ABORT, 'Private acknowledgement'); END"
                } else {
                    "CREATE TRIGGER refuse_analysis BEFORE INSERT ON owned_notes WHEN NEW.stage='analysis' BEGIN SELECT RAISE(ABORT, 'Private commit'); END"
                },
            );
            let mut command = sandbox.vpt();
            command.args(["synthesize", record.id.as_str()]);
            if structured {
                command.arg("--json");
            }
            let output = support::run_with_budget(&mut command, Duration::from_secs(5));
            assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
            assert!(output.stdout.is_empty());
            assert!(!stderr(&output).contains("Private"));
            let expected = "unknown additive proposal field: future_proposal";
            if structured {
                let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
                assert_eq!(error["error"]["kind"], "ledger");
                let diagnostics = error["error"]["diagnostics"]
                    .as_array()
                    .expect("proposal diagnostic on save failure");
                assert_eq!(
                    diagnostics
                        .iter()
                        .filter(|value| value.as_str() == Some(expected))
                        .count(),
                    1
                );
                assert_eq!(
                    error["error"]["completed"],
                    serde_json::json!([record.id.as_str()])
                );
            } else {
                assert_eq!(stderr(&output).matches(expected).count(), 1);
            }
            assert_eq!(
                ledger(&sandbox)
                    .accepted_proposal(&record.id)
                    .unwrap()
                    .is_some(),
                publish
            );
            assert_eq!(
                !ledger(&sandbox).pending_publications().unwrap().is_empty(),
                publish
            );
        }
    }
}

#[test]
fn additive_proposal_diagnostics_are_visible_in_human_and_json_success() {
    for structured in [true, false] {
        let (sandbox, record) = note::configured();
        agent(&sandbox, "");
        let path = sandbox.path().join("captured.proposal");
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        value["future_proposal"] = "Private additive value".into();
        fs::write(path, value.to_string()).unwrap();
        let mut command = sandbox.vpt();
        command.args(["synthesize", record.id.as_str()]);
        if structured {
            command.arg("--json");
        }
        let output = support::run_with_budget(&mut command, Duration::from_secs(5));
        assert!(output.status.success(), "{}", stderr(&output));
        assert!(output.stderr.is_empty());
        let diagnostic = "unknown additive proposal field: future_proposal";
        if structured {
            let result = note::success(&output);
            assert!(
                result["diagnostics"]
                    .as_array()
                    .unwrap()
                    .contains(&serde_json::json!(diagnostic))
            );
        } else {
            assert_eq!(stdout(&output).matches(diagnostic).count(), 1);
        }
        assert!(!stdout(&output).contains("Private additive value"));
    }
}

#[test]
fn unknown_proposal_versions_are_named_without_exposing_or_replacing_content() {
    let (sandbox, record) = note::configured();
    agent(&sandbox, "");
    note::success(&invoke(&sandbox, record.id.as_str(), false));
    let accepted = ledger(&sandbox).accepted_proposal(&record.id).unwrap();
    let analysis = ledger(&sandbox)
        .note(&record.id, NoteStage::Analysis)
        .unwrap()
        .unwrap();
    let proposal = sandbox.path().join("captured.proposal");
    let mut value: serde_json::Value =
        serde_json::from_slice(&fs::read(&proposal).unwrap()).unwrap();
    for version in [2, u64::MAX] {
        value["schema"] = format!("vpt.proposal/{version}").into();
        fs::write(&proposal, value.to_string()).unwrap();
        for structured in [true, false] {
            let mut command = sandbox.vpt();
            command.args(["synthesize", record.id.as_str()]);
            if structured {
                command.arg("--json");
            }
            let output = support::run_with_budget(&mut command, Duration::from_secs(5));
            assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
            assert!(output.stdout.is_empty());
            let message = format!("unsupported synthesis output schema vpt.proposal/{version}");
            if structured {
                let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
                assert_eq!(error["error"]["kind"], "command");
                assert_eq!(error["error"]["message"], message);
            } else {
                assert!(stderr(&output).contains(&message));
            }
            assert!(!stderr(&output).contains("private"));
            let ledger = ledger(&sandbox);
            assert_eq!(ledger.accepted_proposal(&record.id).unwrap(), accepted);
            let mut after = ledger
                .note(&record.id, NoteStage::Analysis)
                .unwrap()
                .unwrap();
            after.updated_at = analysis.updated_at;
            assert_eq!(after, analysis);
            assert_eq!(fs::read_to_string(&analysis.target).unwrap(), analysis.text);
        }
    }
}
