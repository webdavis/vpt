use super::*;
use vpt_application::ports::{NoteArtifact, NoteChange, NoteLedger, NoteStage, Stores, WorkDelta};
use vpt_domain::{
    run::{HistoryFact, WorkOutcome},
    time::UtcOffset,
};

fn seed_note(sandbox: &Sandbox) -> NoteArtifact {
    let row = fixture::record(sandbox, 7, StageStates::fresh());
    fixture::seed(sandbox, vec![row.clone()]);
    let directory = sandbox.path().join("home/.vpt/transcripts");
    std::fs::create_dir_all(&directory).unwrap();
    let stores = vpt_adapters::FilesystemStores::open(std::slice::from_ref(&directory)).unwrap();
    let text = format!(
        "---\nvptSchema: 1\nvptRecording: {}\nvptStage: transcript\n---\n# Pinned\n<!-- vpt:links start -->\n<!-- vpt:links end -->\n<!-- vpt:content start -->\nPrivate note canary\n<!-- vpt:content end -->\nOperator prose canary\n",
        row.id
    );
    let content = vpt_adapters::note_document::Document::parse(&text)
        .unwrap()
        .content()
        .unwrap()
        .to_owned();
    let at = UtcInstant { secs: 120 };
    let artifact = NoteArtifact {
        recording: row.id.clone(),
        stage: NoteStage::Transcript,
        target: directory.join("pinned.md"),
        digest: stores.digest_bytes(text.as_bytes()),
        content_digest: stores.digest_bytes(content.as_bytes()),
        text,
        managed_tags: vec![],
        first_written: at,
        write_offset: UtcOffset { secs: 0 },
        updated_at: at,
    };
    let ledger =
        SqliteLedger::open(&RootDir::open(&sandbox.path().join("state/vpt")).unwrap()).unwrap();
    let HistoryWrite::Committed(run) = ledger.begin_run(RunKind::Publish, at) else {
        panic!("run")
    };
    let delta = WorkDelta {
        notes: vec![NoteChange {
            relocated_from: None,
            artifact: artifact.clone(),
            expected_saved: None,
        }],
        commit: LedgerCommit {
            publications: vec![DirtyPublication {
                target: artifact.target.clone(),
                expected_previous: None,
                intended: artifact.digest,
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
    artifact
}

#[test]
fn startup_publishes_saved_note_bytes_without_transcription() {
    let sandbox = configured();
    sandbox.install_fake_helper();
    let note = seed_note(&sandbox);
    add(&sandbox, "new.m4a");
    let output = run(sandbox.vpt().args(["ingest", "--json"]));
    json(&output);
    assert_eq!(std::fs::read_to_string(&note.target).unwrap(), note.text);
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    assert!(ledger.pending_publications().unwrap().is_empty());
    assert_eq!(ledger.note_at(&note.target).unwrap(), Some(note));
    let events = std::fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
    for private in ["Private note canary", "Operator prose canary"] {
        assert!(!stdout(&output).contains(private));
        assert!(!events.contains(private));
    }
}

#[test]
fn dry_run_does_not_publish_pending_notes_or_change_saved_state() {
    let sandbox = configured();
    let note = seed_note(&sandbox);
    let before = snapshot(&sandbox);
    json(&run(sandbox.vpt().args(["ingest", "--dry-run", "--json"])));
    assert!(!note.target.exists());
    assert_eq!(snapshot(&sandbox), before);
}

#[test]
fn operator_replacement_survives_pending_note_recovery() {
    let sandbox = configured();
    let note = seed_note(&sandbox);
    std::fs::write(&note.target, "operator replacement").unwrap();
    let output = run(sandbox.vpt().args(["ingest", "--json"]));
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    assert_eq!(
        std::fs::read_to_string(&note.target).unwrap(),
        "operator replacement"
    );
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    assert_eq!(ledger.pending_publications().unwrap().len(), 1);
}

#[test]
fn already_published_note_is_acknowledged_without_replacing_the_file() {
    use std::os::unix::fs::MetadataExt;
    let sandbox = configured();
    sandbox.install_fake_helper();
    let note = seed_note(&sandbox);
    std::fs::write(&note.target, &note.text).unwrap();
    let before = std::fs::metadata(&note.target).unwrap();
    json(&run(sandbox.vpt().args(["ingest", "--json"])));
    let after = std::fs::metadata(&note.target).unwrap();
    assert_eq!(
        (before.ino(), before.mtime(), before.mtime_nsec()),
        (after.ino(), after.mtime(), after.mtime_nsec())
    );
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    assert!(ledger.pending_publications().unwrap().is_empty());
}
