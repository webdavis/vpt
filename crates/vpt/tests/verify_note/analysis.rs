use super::*;
use vpt_adapters::{FilesystemStores, RootDir};
use vpt_domain::{
    review::FlagContent,
    run::{HistoryFact, RunKind, WorkOutcome},
    time::UtcInstant,
};

#[test]
fn verification_updates_only_the_selected_artifact_and_counts_both_finding_sets() {
    let (sandbox, record) = note::configured();
    success(&note::invoke(&sandbox, record.id.as_str()));
    let transcript = note::saved(&sandbox, &record);
    success(&invoke(&sandbox, &transcript.target, record.id.as_str()));
    let before = ledger(&sandbox)
        .review_snapshot(&record.id)
        .unwrap()
        .unwrap()
        .flags;
    let root = sandbox.path().join("home/.vpt/analysis");
    fs::create_dir_all(&root).unwrap();
    let stores = FilesystemStores::open(std::slice::from_ref(&root)).unwrap();
    let mut analysis = transcript.clone();
    analysis.stage = NoteStage::Analysis;
    analysis.target = root.join("analysis.md");
    analysis.text = analysis
        .text
        .replace("vptStage: \"transcript\"", "vptStage: \"analysis\"");
    analysis.digest = stores.digest_bytes(analysis.text.as_bytes());
    let at = UtcInstant { secs: 100 };
    analysis.updated_at = at;
    let writer =
        SqliteLedger::open(&RootDir::open(&sandbox.path().join("state/vpt")).unwrap()).unwrap();
    let HistoryWrite::Committed(run) = writer.begin_run(RunKind::Note, at) else {
        panic!("begin")
    };
    let delta = WorkDelta {
        notes: vec![NoteChange {
            artifact: analysis.clone(),
            expected_saved: None,
            relocated_from: None,
        }],
        commit: LedgerCommit {
            publications: vec![DirtyPublication {
                target: analysis.target.clone(),
                expected_previous: None,
                intended: analysis.digest,
                recorded_at: at,
            }],
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(matches!(
        writer.commit_step(
            run.run,
            &delta,
            &[(
                at,
                HistoryFact::WorkStep {
                    recording: Some(record.id.clone()),
                    outcome: WorkOutcome::Completed
                }
            )]
        ),
        HistoryWrite::Committed(_)
    ));
    fs::write(&analysis.target, &analysis.text).unwrap();
    writer.clear_publication(&analysis.target).unwrap();
    drop(writer);
    assert_eq!(
        success(&invoke(&sandbox, &analysis.target, record.id.as_str()))["owned"],
        true
    );
    let flags = ledger(&sandbox)
        .review_snapshot(&record.id)
        .unwrap()
        .unwrap()
        .flags;
    assert_eq!(flags.len(), 2);
    assert!(flags.contains(&before[0]));
    assert!(flags.iter().any(|flag| matches!(&flag.content, FlagContent::Verification(flag) if flag.artifact == NoteStage::Analysis)));
    assert!(
        fs::read_to_string(&analysis.target)
            .unwrap()
            .contains("vptOpenFlags: 2\n")
    );
    let connection = rusqlite::Connection::open(sandbox.path().join("state/vpt/vpt.db")).unwrap();
    fixture::preserve_wal(&connection);
    let count: u32 = connection
        .query_row("SELECT open_flags FROM recordings", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 2);
}
