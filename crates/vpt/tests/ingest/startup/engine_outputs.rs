use super::*;
use std::os::unix::fs::PermissionsExt;
use vpt_application::ports::{AcceptedTranscript, EngineArtifact, TranscriptionChange, WorkDelta};
use vpt_domain::run::{HistoryFact, WorkOutcome};
use vpt_domain::transcript::{EngineMetadata, LanguageSource, Segment, Transcript, Word};

fn seed_output(sandbox: &Sandbox) -> (std::path::PathBuf, Vec<u8>) {
    let row = fixture::record(sandbox, 7, StageStates::fresh());
    fixture::seed(sandbox, vec![row.clone()]);
    let root = sandbox.path().join("home/.vpt/engine-outputs");
    std::fs::create_dir_all(&root).unwrap();
    let target = root.join(format!("{}.fixture.json", row.id));
    let bytes = serde_json::to_vec_pretty(&serde_json::json!({"schema":"vpt.engine/1", "engine": {
        "name":"fixture", "tool":"fixture", "version":"1", "model":"test", "family":"test"},
        "language":"en-US", "local":true, "segments":[{"start":0.0, "end":1.0, "text":"private transcript canary", "words":[{"text":"private transcript canary", "start":0.0, "end":1.0}]}]})).unwrap();
    let stores = vpt_adapters::FilesystemStores::open(std::slice::from_ref(&root)).unwrap();
    let digest = vpt_application::ports::Stores::digest_bytes(&stores, &bytes);
    let at = UtcInstant { secs: 120 };
    let accepted = AcceptedTranscript {
        checker_engine: None,
        uncertainty: Default::default(),
        recording: row.id.clone(),
        requested_language: "en-US".into(),
        duration: 1.0,
        accepted_at: at,
        lexical_classification: false,
        from_checker: false,
        transcript: Transcript {
            engine: EngineMetadata {
                name: "fixture".into(),
                tool: "fixture".into(),
                version: "1".into(),
                model: "test".into(),
                family: "test".into(),
            },
            language: "en-US".into(),
            language_source: LanguageSource::Reported,
            local: true,
            segments: vec![Segment {
                speaker: None,
                start: 0.0,
                end: 1.0,
                text: "private transcript canary".into(),
                words: vec![Word {
                    text: "private transcript canary".into(),
                    start: 0.0,
                    end: 1.0,
                    confidence: None,
                }],
            }],
        },
    };
    let ledger =
        SqliteLedger::open(&RootDir::open(&sandbox.path().join("state/vpt")).unwrap()).unwrap();
    let HistoryWrite::Committed(run) = ledger.begin_run(RunKind::Transcribe, at) else {
        panic!("run")
    };
    let delta = WorkDelta {
        transcription: Some(TranscriptionChange::Accept(Box::new(accepted))),
        engine_outputs: vec![EngineArtifact {
            recording: row.id.clone(),
            engine: "fixture".into(),
            target: target.clone(),
            digest,
            duration: 1.0,
            language_source: LanguageSource::Reported,
            bytes: bytes.clone(),
            created_at: at,
        }],
        commit: LedgerCommit {
            publications: vec![DirtyPublication {
                target: target.clone(),
                expected_previous: None,
                intended: digest,
                recorded_at: at,
            }],
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(matches!(
        ledger.commit_step(
            run.run,
            &delta,
            &[(
                at,
                HistoryFact::WorkStep {
                    recording: Some(row.id),
                    outcome: WorkOutcome::Completed
                }
            )]
        ),
        HistoryWrite::Committed(_)
    ));
    (target, bytes)
}

#[test]
fn startup_recovers_saved_engine_output_privately_without_running_an_engine() {
    let sandbox = configured();
    sandbox.install_fake_helper();
    let (target, bytes) = seed_output(&sandbox);
    add(&sandbox, "new.m4a");
    let output = run(sandbox.vpt().args(["ingest", "--json"]));
    json(&output);
    assert_eq!(std::fs::read(&target).unwrap(), bytes);
    assert_eq!(
        std::fs::metadata(&target).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    assert!(ledger.pending_publications().unwrap().is_empty());
    assert!(!stdout(&output).contains("private transcript canary"));
    assert!(
        !std::fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl"))
            .unwrap()
            .contains("private transcript canary")
    );
}

#[test]
fn dry_run_preserves_pending_engine_output_and_saved_state() {
    let sandbox = configured();
    let (target, _) = seed_output(&sandbox);
    add(&sandbox, "new.m4a");
    let before = snapshot(&sandbox);
    json(&run(sandbox.vpt().args(["ingest", "--dry-run", "--json"])));
    assert_eq!(snapshot(&sandbox), before);
    assert!(!target.exists());
}

#[test]
fn retention_repairs_engine_outputs_before_new_work_but_its_dry_run_does_not() {
    let sandbox = configured();
    let config = std::fs::read_to_string(sandbox.config_path()).unwrap();
    std::fs::write(
        sandbox.config_path(),
        format!("{config}\n[retention]\nenabled = true\n"),
    )
    .unwrap();
    let (target, bytes) = seed_output(&sandbox);
    let before = snapshot(&sandbox);
    json(&run(sandbox.vpt().args([
        "retention",
        "run",
        "--dry-run",
        "--json",
    ])));
    assert_eq!(snapshot(&sandbox), before);
    json(&run(sandbox.vpt().args(["retention", "run", "--json"])));
    assert_eq!(std::fs::read(&target).unwrap(), bytes);
    assert_eq!(
        std::fs::metadata(target).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[test]
fn replaced_raw_output_is_preserved_and_its_publication_stays_pending() {
    let sandbox = configured();
    let (target, _) = seed_output(&sandbox);
    std::fs::write(&target, b"operator replacement").unwrap();
    add(&sandbox, "new.m4a");
    let output = run(sandbox.vpt().args(["ingest", "--json"]));
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    assert!(output.stdout.is_empty());
    assert_eq!(std::fs::read(&target).unwrap(), b"operator replacement");
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    assert_eq!(ledger.pending_publications().unwrap().len(), 1);
}
