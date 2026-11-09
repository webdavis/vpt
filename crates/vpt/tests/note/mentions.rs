use super::*;
use std::fs;
use vpt_domain::{
    note::managed::Regions,
    relations::{Kind, Target},
};

fn configured_mentions(
    obsidian: bool,
) -> (
    Sandbox,
    RecordingRecord,
    std::path::PathBuf,
    std::path::PathBuf,
) {
    let (sandbox, row) = configured();
    let terms = sandbox.config_path().with_file_name("known-terms.txt");
    fs::write(&terms, "Private\nshared\n").unwrap();
    let root = if obsidian {
        sandbox.path().join("vault")
    } else {
        sandbox.path().join("home/.vpt/briefs")
    };
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("Private.md"),
        "---\naliases: [shared]\n---\nExternal first",
    )
    .unwrap();
    fs::write(
        root.join("Other.md"),
        "---\naliases: [shared]\n---\nExternal second",
    )
    .unwrap();
    let profile = if obsidian {
        format!("[note]\nprofile='obsidian'\n[note.obsidian]\nvault_root={root:?}\n")
    } else {
        String::new()
    };
    fixture::write_config(
        &sandbox,
        &format!("[reconcile]\nknown_terms_path={terms:?}\n{profile}"),
    );
    let ledger =
        SqliteLedger::open(&RootDir::open(&sandbox.path().join("state/vpt")).unwrap()).unwrap();
    let mut accepted = ledger.accepted_transcript(&row.id).unwrap().unwrap();
    accepted.transcript.segments[0].text = "Private shared Private Other".into();
    accepted.transcript.segments[0].words = ["Private", "shared", "Private", "Other"]
        .into_iter()
        .enumerate()
        .map(|(n, text)| Word {
            text: text.into(),
            start: n as f64 / 4.0,
            end: (n + 1) as f64 / 4.0,
            confidence: None,
        })
        .collect();
    let at = accepted.accepted_at;
    let HistoryWrite::Committed(run) = ledger.begin_run(RunKind::Transcribe, at) else {
        panic!("run");
    };
    assert!(matches!(
        ledger.commit_step(
            run.run,
            &WorkDelta {
                transcription: Some(TranscriptionChange::Accept(Box::new(accepted))),
                ..Default::default()
            },
            &[(
                at,
                HistoryFact::WorkStep {
                    recording: Some(row.id.clone()),
                    outcome: WorkOutcome::Completed
                }
            )]
        ),
        HistoryWrite::Committed(_)
    ));
    (sandbox, row, root, terms)
}

#[test]
fn note_write_derives_unique_mentions_and_saves_ambiguity_ranges_for_both_profiles() {
    for obsidian in [false, true] {
        let (sandbox, row, root, _) = configured_mentions(obsidian);
        let before = [
            fs::read(root.join("Private.md")).unwrap(),
            fs::read(root.join("Other.md")).unwrap(),
        ];
        let output = invoke(&sandbox, row.id.as_str());
        success(&output);
        let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
            .unwrap()
            .unwrap();
        let relations = ledger.relations(&row.id).unwrap();
        assert_eq!(relations.len(), 1);
        assert_eq!(relations[0].kind, Kind::Mentions);
        assert_eq!(
            relations[0].target,
            Target::Note(root.join("Private.md").to_str().unwrap().into())
        );
        assert_eq!(relations[0].ranges, [0..1, 2..3]);
        let ambiguities = ledger.mention_ambiguities(&row.id).unwrap();
        assert_eq!(ambiguities.len(), 1);
        assert_eq!(ambiguities[0].term, "shared");
        assert_eq!(ambiguities[0].ranges.len(), 1);
        assert_eq!(ambiguities[0].ranges[0], 1..2);
        let note = saved(&sandbox, &row);
        let links = Regions::parse(&note.text).unwrap().links();
        assert!(links.contains("- Mentions: [Private]"));
        assert!(!links.contains("Other"));
        assert_eq!(fs::read(root.join("Private.md")).unwrap(), before[0]);
        assert_eq!(fs::read(root.join("Other.md")).unwrap(), before[1]);
        assert!(!stdout(&output).contains("Private"));
    }
}

#[test]
fn changed_aliases_and_terms_refresh_only_derived_mentions_and_keep_content() {
    use vpt_domain::relations::{Provenance, Relation, State};
    let (sandbox, row, root, terms) = configured_mentions(false);
    let manual = Relation {
        kind: Kind::Mentions,
        target: Target::Note(root.join("Other.md").to_str().unwrap().into()),
        state: State::Confirmed,
        provenance: Provenance::Manual,
        ranges: vec![],
    };
    relations::update(&sandbox, &row.id, vec![manual.clone()]);
    success(&invoke(&sandbox, row.id.as_str()));
    let before = saved(&sandbox, &row);
    fs::write(
        &before.target,
        format!("{}\nOperator ending\n", before.text),
    )
    .unwrap();
    fs::write(root.join("Other.md"), "External second").unwrap();
    success(&invoke(&sandbox, row.id.as_str()));
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    let links = ledger.relations(&row.id).unwrap();
    assert_eq!(links.len(), 2);
    assert!(links.contains(&manual));
    let derived = links
        .iter()
        .find(|link| link.provenance != Provenance::Manual)
        .unwrap();
    assert_eq!(derived.ranges, [0..1, 1..2, 2..3]);
    assert!(ledger.mention_ambiguities(&row.id).unwrap().is_empty());
    fs::write(&terms, "").unwrap();
    success(&invoke(&sandbox, row.id.as_str()));
    assert_eq!(ledger.relations(&row.id).unwrap(), [manual]);
    assert!(ledger.mention_ambiguities(&row.id).unwrap().is_empty());
    let after = saved(&sandbox, &row);
    assert_eq!(after.content_digest, before.content_digest);
    assert!(after.text.ends_with("\nOperator ending\n"));
    assert!(
        !Regions::parse(&after.text)
            .unwrap()
            .links()
            .contains("Private")
    );
}

#[test]
fn failed_note_update_preserves_mention_links_ambiguities_and_saved_note() {
    for late_failure in [false, true] {
        let (sandbox, row, root, terms) = configured_mentions(false);
        success(&invoke(&sandbox, row.id.as_str()));
        let before = saved(&sandbox, &row);
        let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
            .unwrap()
            .unwrap();
        let links = ledger.relations(&row.id).unwrap();
        let ambiguities = ledger.mention_ambiguities(&row.id).unwrap();
        fs::write(terms, "shared\n").unwrap();
        fs::write(root.join("Other.md"), "External second").unwrap();
        let expected = if late_failure {
            let connection =
                rusqlite::Connection::open(sandbox.path().join("state/vpt/vpt.db")).unwrap();
            fixture::preserve_wal(&connection);
            connection.execute_batch("CREATE TRIGGER refuse_mentions_history BEFORE INSERT ON history_records WHEN instr(NEW.fact, '\"tag\":\"work\"') > 0 BEGIN SELECT RAISE(ABORT, 'Private canary'); END;").unwrap();
            before.text.clone()
        } else {
            let edited = before.text.replace("<!-- vpt:content end -->", "removed");
            fs::write(&before.target, &edited).unwrap();
            edited
        };
        let output = invoke(&sandbox, row.id.as_str());
        assert_eq!(
            output.status.code(),
            Some(if late_failure { 1 } else { 3 }),
            "{}",
            stderr(&output)
        );
        assert!(output.stdout.is_empty());
        assert!(!stderr(&output).contains("Private canary"));
        assert_eq!(ledger.relations(&row.id).unwrap(), links);
        assert_eq!(ledger.mention_ambiguities(&row.id).unwrap(), ambiguities);
        assert_eq!(saved(&sandbox, &row), before);
        assert_eq!(fs::read_to_string(&before.target).unwrap(), expected);
        assert!(ledger.pending_publications().unwrap().is_empty());
    }
}
