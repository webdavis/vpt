use super::*;
use vpt_domain::{
    brief::{self, ContextSummary, Options, Request, document::Snapshot, markdown},
    time::UtcOffset,
};

fn pending(sandbox: &Sandbox) -> BriefArtifact {
    pending_named(sandbox, "Private provider id")
}
fn pending_named(sandbox: &Sandbox, key: &str) -> BriefArtifact {
    use std::os::unix::fs::PermissionsExt;
    fixture::write_config(sandbox, "[retention]\nenabled=true\n");
    let row = seed_named(sandbox, key);
    std::fs::set_permissions(
        sandbox.path().join("state/vpt"),
        std::fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    let generated_at = UtcInstant { secs: 1000 };
    let pack = brief::assemble(&Request {
        occasion: &row.details,
        terms: &[],
        candidates: &[],
        now: generated_at,
        options: Options::default(),
    })
    .unwrap();
    let snapshot = Snapshot {
        id: row.id.clone(),
        source: row.key.source(),
        occasion: row.details.clone(),
        pack,
        context: ContextSummary::default(),
        tasks: vec![],
        generated_at,
    };
    let origin = sandbox.path().join("home/.vpt/briefs");
    std::fs::create_dir_all(&origin).unwrap();
    let filename = markdown::filename(&snapshot.document(), 60).unwrap();
    let text = markdown::create(&markdown::Note {
        document: snapshot.document(),
        filename: &filename,
        origin: &origin,
        wiki_root: None,
        link_style: vpt_domain::note::links::LinkStyle::Markdown,
        profile: vpt_domain::note::document::Profile::Portable,
    })
    .unwrap();
    let artifact = vpt_adapters::prepare_brief_artifact(
        snapshot,
        origin.join(filename),
        text,
        row.details.tags.clone(),
        generated_at,
        UtcOffset { secs: 0 },
    )
    .unwrap();
    let root = RootDir::open(&sandbox.path().join("state/vpt")).unwrap();
    let ledger = SqliteLedger::open(&root).unwrap();
    let HistoryWrite::Committed(run) = ledger.begin_run(RunKind::Brief, generated_at) else {
        panic!("run")
    };
    assert!(matches!(
        ledger.commit_step(
            run.run,
            &WorkDelta {
                brief: Some(BriefChange {
                    expected_occasion: row,
                    expected: None,
                    artifact: artifact.clone()
                }),
                commit: LedgerCommit {
                    publications: vec![DirtyPublication {
                        target: artifact.target.clone(),
                        expected_previous: None,
                        intended: artifact.version.content,
                        recorded_at: generated_at
                    }],
                    ..Default::default()
                },
                ..Default::default()
            },
            &[(
                generated_at,
                HistoryFact::BriefSaved {
                    occasion: artifact.snapshot.id.clone()
                }
            )]
        ),
        HistoryWrite::Committed(_)
    ));
    artifact
}

#[test]
fn startup_recovers_saved_brief_bytes_and_marks_completion_without_a_notification() {
    let sandbox = fixture::configured("brief-recovery");
    let artifact = pending(&sandbox);
    let before = fixture::snapshot(&sandbox);
    let observed = invoke(&sandbox, &["occasions", "--json"]);
    assert_eq!(observed.status.code(), Some(0), "{}", stderr(&observed));
    assert_eq!(fixture::snapshot(&sandbox), before);
    let output = invoke(&sandbox, &["retention", "run", "--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        std::fs::read_to_string(&artifact.target).unwrap(),
        artifact.text
    );
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    assert!(ledger.pending_publications().unwrap().is_empty());
    assert!(
        ledger
            .occasion(&artifact.snapshot.id)
            .unwrap()
            .unwrap()
            .briefed_at
            .is_some()
    );
    assert!(
        ledger
            .brief(&artifact.snapshot.id)
            .unwrap()
            .unwrap()
            .published_at
            .is_some()
    );
    let events = std::fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
    assert!(!events.contains("Private"));
    assert!(!events.contains("brief_written"));
    assert!(events.contains("brief_published"));
}

#[test]
fn startup_refuses_replaced_brief_bytes_and_keeps_the_pending_pack() {
    let sandbox = fixture::configured("brief-replaced");
    let artifact = pending(&sandbox);
    std::fs::write(&artifact.target, b"Private operator replacement").unwrap();
    let output = invoke(&sandbox, &["retention", "run", "--json"]);
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    assert_eq!(
        std::fs::read(&artifact.target).unwrap(),
        b"Private operator replacement"
    );
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    assert_eq!(ledger.pending_publications().unwrap().len(), 1);
    assert_eq!(ledger.brief(&artifact.snapshot.id).unwrap(), Some(artifact));
    assert!(!stderr(&output).contains("Private"));
}

#[test]
fn startup_preserves_brief_completion_when_pipeline_source_is_missing() {
    let sandbox = fixture::configured("brief-then-missing-source");
    let artifact = pending(&sandbox);
    fixture::write_config(
        &sandbox,
        &format!(
            "[source]\nrecordings_dir={:?}\n[engines]\nmain='custom'\n[engines.custom]\nkind='command'\nfamily='independent'\nlocal=true\ncommand=['/usr/bin/false']\n",
            sandbox.path().join("missing-source/Recordings")
        ),
    );
    let output = invoke(&sandbox, &["run", "--json"]);
    assert!(!output.status.success());
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(
        error["error"]["completed"],
        serde_json::json!([artifact.snapshot.id.as_str()]),
        "{}",
        stderr(&output)
    );
    assert_eq!(
        std::fs::read_to_string(&artifact.target).unwrap(),
        artifact.text
    );
}

#[test]
fn later_publication_failure_preserves_the_first_completed_occasion() {
    let sandbox = fixture::configured("brief-partial-recovery");
    let mut artifacts = [
        pending_named(&sandbox, "Private first"),
        pending_named(&sandbox, "Private second"),
    ];
    artifacts.sort_by(|a, b| a.target.cmp(&b.target));
    std::fs::write(&artifacts[1].target, b"Private replacement").unwrap();
    let output = invoke(&sandbox, &["retention", "run", "--json"]);
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(
        error["error"]["completed"],
        serde_json::json!([artifacts[0].snapshot.id.as_str()])
    );
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    assert!(
        ledger
            .brief(&artifacts[0].snapshot.id)
            .unwrap()
            .unwrap()
            .published_at
            .is_some()
    );
    assert!(
        ledger
            .brief(&artifacts[1].snapshot.id)
            .unwrap()
            .unwrap()
            .published_at
            .is_none()
    );
    assert_eq!(ledger.pending_publications().unwrap().len(), 1);
    assert!(!stderr(&output).contains("Private"));
}

#[test]
fn recovery_finishes_a_brief_already_renamed_before_the_journal_was_cleared() {
    let sandbox = fixture::configured("brief-after-rename");
    let artifact = pending(&sandbox);
    std::fs::write(&artifact.target, &artifact.text).unwrap();
    let output = invoke(&sandbox, &["retention", "run", "--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    let published = ledger
        .brief(&artifact.snapshot.id)
        .unwrap()
        .unwrap()
        .published_at;
    assert!(published.is_some());
    assert_eq!(
        ledger
            .occasion(&artifact.snapshot.id)
            .unwrap()
            .unwrap()
            .briefed_at,
        published
    );
    assert!(ledger.pending_publications().unwrap().is_empty());
    let rerun = invoke(&sandbox, &["retention", "run", "--json"]);
    assert_eq!(rerun.status.code(), Some(0), "{}", stderr(&rerun));
    assert_eq!(
        ledger
            .brief(&artifact.snapshot.id)
            .unwrap()
            .unwrap()
            .published_at,
        published
    );
    assert_eq!(
        std::fs::read_to_string(&artifact.target).unwrap(),
        artifact.text
    );
}
