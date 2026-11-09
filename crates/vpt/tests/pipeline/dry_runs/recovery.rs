use super::*;
use vpt_adapters::{RootDir, WriteLock};
use vpt_domain::{run::RunKind, time::UtcInstant};

#[test]
fn every_dry_run_preserves_pending_recovery_and_ignores_the_write_lock() {
    let sandbox = configured();
    add(&sandbox, "one.m4a", 1_787_604_456);
    json(&invoke(&sandbox, &[]));
    let record = ledger(&sandbox).recordings().unwrap().remove(0);
    let brief = json(&support::run_with_budget(
        sandbox.vpt().args([
            "brief",
            "--title",
            "Private preview",
            "--at",
            "2026-10-01T10:00:00Z",
            "--json",
        ]),
        Duration::from_secs(5),
    ));
    enable_commands(&sandbox);
    let root = RootDir::open(&sandbox.path().join("state/vpt")).unwrap();
    let writer = SqliteLedger::open(&root).unwrap();
    let note = writer
        .note(&record.id, NoteStage::Transcript)
        .unwrap()
        .unwrap();
    let at = UtcInstant { secs: 100 };
    writer
        .commit(&LedgerCommit {
            publications: vec![DirtyPublication {
                target: note.target,
                expected_previous: None,
                intended: note.digest,
                recorded_at: at,
            }],
            ..Default::default()
        })
        .unwrap();
    let HistoryWrite::Committed(run) = writer.begin_run(RunKind::Retain, at) else {
        panic!("run")
    };
    assert!(matches!(
        writer.prepare_retention(
            run.run,
            &NewIntent {
                kind: ArtifactKind::Audio,
                owner: record.id.clone().into(),
                path: record.audio_path.clone(),
                expected: record.digest,
                recorded_at: at,
            }
        ),
        HistoryWrite::Committed(_)
    ));
    drop(writer);
    fs::write(
        sandbox.path().join("state/vpt/events.jsonl"),
        b"{\"unfinished\":",
    )
    .unwrap();
    let _lock = WriteLock::acquire(&root, Duration::ZERO).unwrap();
    for verb in VERBS {
        assert_preview(&sandbox, verb, record.id.as_str(), &[0]);
    }
    let occasion = brief["occasion"]["id"].as_str().unwrap();
    for flag in ["--dry-run", "--explain"] {
        for structured in [true, false] {
            let before = fixture::snapshot(&sandbox);
            let mut command = sandbox.vpt();
            command.args(["brief", occasion, flag]);
            if structured {
                command.arg("--json");
            }
            let output = support::run_with_budget(&mut command, Duration::from_secs(5));
            assert!(output.status.success(), "{}", stderr(&output));
            assert!(output.stderr.is_empty());
            assert!(!output.stdout.is_empty());
            if structured {
                assert_eq!(json(&output)["occasion"]["id"], occasion);
            }
            assert_eq!(fixture::snapshot(&sandbox), before);
        }
    }
    let ledger = ledger(&sandbox);
    assert_eq!(ledger.pending_intents().unwrap().len(), 1);
    assert_eq!(ledger.pending_publications().unwrap().len(), 1);
    assert!(!ledger.pending_exports().unwrap().is_empty());
    assert!(record.audio_path.exists());
    assert!(!sandbox.path().join("forbidden-command-called").exists());
}
