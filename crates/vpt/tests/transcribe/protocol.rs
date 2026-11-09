use super::*;
use std::{fs, time::Duration};

#[test]
fn accepted_engine_diagnostics_survive_commit_and_publication_failures() {
    for publish in [false, true] {
        for structured in [true, false] {
            let (sandbox, record) = configured();
            let path = sandbox.path().join("engine.json");
            let mut value: serde_json::Value =
                serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            value["future_engine"] = "Private additive value".into();
            fs::write(path, value.to_string()).unwrap();
            let connection =
                rusqlite::Connection::open(sandbox.path().join("state/vpt/vpt.db")).unwrap();
            fixture::preserve_wal(&connection);
            connection.execute_batch(if publish {
                "CREATE TRIGGER refuse_output BEFORE DELETE ON dirty_publications WHEN OLD.target LIKE '%/engine-outputs/%' BEGIN SELECT RAISE(ABORT, 'Private acknowledgement'); END"
            } else {
                "CREATE TRIGGER refuse_transcript BEFORE INSERT ON transcripts BEGIN SELECT RAISE(ABORT, 'Private commit'); END"
            }).unwrap();
            drop(connection);
            let mut command = sandbox.vpt();
            command.args(["transcribe", record.id.as_str()]);
            if structured {
                command.arg("--json");
            }
            let output = support::run_with_budget(&mut command, Duration::from_secs(5));
            assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
            assert!(output.stdout.is_empty());
            assert!(!stderr(&output).contains("Private"));
            let expected = "custom: unknown additive engine field: future_engine";
            if structured {
                let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
                assert_eq!(error["error"]["kind"], "ledger");
                let diagnostics = error["error"]["diagnostics"]
                    .as_array()
                    .expect("engine diagnostic on save failure");
                assert_eq!(
                    diagnostics
                        .iter()
                        .filter(|value| value.as_str() == Some(expected))
                        .count(),
                    1
                );
                assert_eq!(
                    error["error"]["completed"],
                    if publish {
                        serde_json::json!([record.id.as_str()])
                    } else {
                        serde_json::json!([])
                    }
                );
            } else {
                assert_eq!(stderr(&output).matches(expected).count(), 1);
            }
            assert_eq!(
                ledger(&sandbox)
                    .accepted_transcript(&record.id)
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
fn unsupported_engine_version_is_named_in_both_error_formats() {
    for structured in [true, false] {
        let (sandbox, record) = configured();
        fs::write(
            sandbox.path().join("engine.json"),
            br#"{"schema":"vpt.engine/2","Private":"provider value"}"#,
        )
        .unwrap();
        let mut command = sandbox.vpt();
        command.args(["transcribe", record.id.as_str()]);
        if structured {
            command.arg("--json");
        }
        let output = support::run_with_budget(&mut command, Duration::from_secs(5));
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(stderr(&output).contains("UnsupportedVersion(2)"));
        assert!(!stderr(&output).contains("provider value"));
        assert!(
            ledger(&sandbox)
                .accepted_transcript(&record.id)
                .unwrap()
                .is_none()
        );
    }
}
