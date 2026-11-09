use super::*;

fn path(sandbox: &Sandbox, id: &str, stage: &str, json: bool) -> std::process::Output {
    let mut command = sandbox.vpt();
    command.args(["path", id, "--stage", stage]);
    if json {
        command.arg("--json");
    }
    support::run_with_budget(&mut command, std::time::Duration::from_secs(5))
}

#[test]
fn pinned_path_is_observational_even_when_note_is_missing_or_changed() {
    let (sandbox, row) = configured();
    success(&invoke(&sandbox, row.id.as_str()));
    let note = saved(&sandbox, &row);
    let state = RootDir::open(&sandbox.path().join("state/vpt")).unwrap();
    let _held = vpt_adapters::WriteLock::acquire(&state, std::time::Duration::ZERO).unwrap();
    let unavailable_store = sandbox.path().join("regular-file-store");
    std::fs::write(&unavailable_store, "not a directory").unwrap();
    fixture::write_config(
        &sandbox,
        &format!("[stores]\ntranscripts = {unavailable_store:?}\n"),
    );
    for bytes in [Some("operator replacement"), None] {
        if let Some(bytes) = bytes {
            std::fs::write(&note.target, bytes).unwrap();
        } else {
            std::fs::remove_file(&note.target).unwrap();
        }
        let before = fixture::snapshot(&sandbox);
        let output = path(&sandbox, row.id.as_str(), "transcript", true);
        let result = success(&output);
        assert_eq!(result["command"], "path");
        assert_eq!(result["path"], note.target.to_str().unwrap());
        assert_eq!(fixture::snapshot(&sandbox), before);
        let human = path(&sandbox, row.id.as_str(), "transcript", false);
        assert_eq!(human.status.code(), Some(0), "{}", stderr(&human));
        assert_eq!(stdout(&human), format!("{}\n", note.target.display()));
        assert_eq!(fixture::snapshot(&sandbox), before);
    }
}

#[test]
fn absent_pin_invalid_stage_and_unknown_identity_do_not_write() {
    let (sandbox, row) = configured();
    for (id, stage) in [
        (row.id.as_str(), "transcript"),
        (row.id.as_str(), "analysis"),
        (row.id.as_str(), "wrong"),
        ("invalid", "transcript"),
    ] {
        let before = fixture::snapshot(&sandbox);
        let output = path(&sandbox, id, stage, true);
        assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
        assert!(output.stdout.is_empty());
        assert_eq!(fixture::snapshot(&sandbox), before);
    }
    let sandbox = fixture::configured("path-absent-ledger");
    let before = fixture::snapshot(&sandbox);
    assert_eq!(
        path(&sandbox, row.id.as_str(), "transcript", true)
            .status
            .code(),
        Some(2)
    );
    assert_eq!(fixture::snapshot(&sandbox), before);
}

#[test]
fn analysis_pin_is_read_without_publishing_pending_bytes() {
    let (sandbox, row) = configured();
    success(&invoke(&sandbox, row.id.as_str()));
    let mut note = saved(&sandbox, &row);
    note.stage = NoteStage::Analysis;
    note.target = sandbox.path().join("home/.vpt/analysis/pinned-analysis.md");
    note.text = note
        .text
        .replace("vptStage: \"transcript\"", "vptStage: \"analysis\"");
    let stores = vpt_adapters::FilesystemStores::open(&[]).unwrap();
    note.digest = stores.digest_bytes(note.text.as_bytes());
    let ledger =
        SqliteLedger::open(&RootDir::open(&sandbox.path().join("state/vpt")).unwrap()).unwrap();
    let HistoryWrite::Committed(run) = ledger.begin_run(RunKind::Note, note.updated_at) else {
        panic!("run")
    };
    assert!(matches!(
        ledger.commit_step(
            run.run,
            &WorkDelta {
                notes: vec![NoteChange {
                    relocated_from: None,
                    artifact: note.clone(),
                    expected_saved: None
                }],
                commit: LedgerCommit {
                    publications: vec![DirtyPublication {
                        target: note.target.clone(),
                        expected_previous: None,
                        intended: note.digest,
                        recorded_at: note.updated_at
                    }],
                    ..Default::default()
                },
                ..Default::default()
            },
            &[(
                note.updated_at,
                HistoryFact::WorkStep {
                    recording: Some(row.id.clone()),
                    outcome: WorkOutcome::Completed
                }
            )]
        ),
        HistoryWrite::Committed(_)
    ));
    drop(ledger);
    let before = fixture::snapshot(&sandbox);
    let result = success(&path(&sandbox, row.id.as_str(), "analysis", true));
    assert_eq!(result["path"], note.target.to_str().unwrap());
    assert!(!note.target.exists());
    assert_eq!(fixture::snapshot(&sandbox), before);
}
