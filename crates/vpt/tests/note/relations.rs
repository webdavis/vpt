use super::*;
use vpt_domain::note::managed::Regions;
use vpt_domain::relations::{Kind, Provenance, Relation, State, Target};

pub(super) fn update(
    sandbox: &Sandbox,
    id: &vpt_domain::identity::RecordingId,
    relations: Vec<Relation>,
) {
    let ledger =
        SqliteLedger::open(&RootDir::open(&sandbox.path().join("state/vpt")).unwrap()).unwrap();
    let at = UtcInstant { secs: 100 };
    let HistoryWrite::Committed(run) = ledger.begin_run(RunKind::Confirm, at) else {
        panic!("run");
    };
    let change = RelationChange {
        ambiguities: None,
        recording: id.clone(),
        expected: ledger.relations(id).unwrap(),
        relations,
        at,
    };
    assert!(matches!(
        ledger.commit_step(
            run.run,
            &WorkDelta {
                relations: Some(change),
                ..Default::default()
            },
            &[(
                at,
                HistoryFact::WorkStep {
                    recording: Some(id.clone()),
                    outcome: WorkOutcome::Completed
                }
            )]
        ),
        HistoryWrite::Committed(_)
    ));
}
fn relation(path: &std::path::Path, kind: Kind, state: State) -> Relation {
    Relation {
        kind,
        target: Target::Note(path.to_str().unwrap().into()),
        state,
        provenance: Provenance::Manual,
        ranges: vec![],
    }
}

#[test]
fn saved_relations_render_escaped_links_and_provenance_without_changing_content_or_external_notes()
{
    let (sandbox, row) = configured();
    success(&invoke(&sandbox, row.id.as_str()));
    let first = saved(&sandbox, &row);
    std::fs::write(&first.target, format!("{}\nOperator ending\n", first.text)).unwrap();
    let path = sandbox.path().join("Private [contact] #1.md");
    std::fs::write(&path, "External private prose").unwrap();
    let mut related = relation(&path, Kind::Related, State::Suggested);
    related.provenance = Provenance::Proposal {
        by: "Private [agent]".into(),
        reason: "same <topic>\n<!-- vpt:content end -->".into(),
        input: row.digest,
    };
    update(
        &sandbox,
        &row.id,
        vec![
            related,
            relation(&path, Kind::Mentions, State::Confirmed),
            relation(
                &sandbox.path().join("Rejected.md"),
                Kind::Related,
                State::Rejected,
            ),
        ],
    );
    let output = invoke(&sandbox, row.id.as_str());
    success(&output);
    let next = saved(&sandbox, &row);
    let links = Regions::parse(&next.text).unwrap().links().to_string();
    assert!(links.contains("- Mentions: [Private &#91;contact&#93; &#35;1](../../../Private%20%5Bcontact%5D%20%231.md)"), "{links}");
    assert!(links.contains("- Related: [Private &#91;contact&#93; &#35;1]"));
    assert!(links.contains("suggested by Private &#91;agent&#93;"));
    assert!(links.contains("same &lt;topic&gt;&#10;&lt;!-- vpt&#58;content end --&gt;"));
    assert!(!links.contains("Rejected"));
    assert!(next.text.ends_with("\nOperator ending\n"));
    assert_eq!(first.content_digest, next.content_digest);
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "External private prose"
    );
    assert!(!stdout(&output).contains("Private"));
    success(&invoke(&sandbox, row.id.as_str()));
    assert_eq!(saved(&sandbox, &row).text, next.text);
}

#[test]
fn suggested_related_links_obey_the_cap_and_wiki_style_is_independent_of_profile() {
    let (sandbox, row) = configured();
    let mut relations = Vec::new();
    for index in 0..7 {
        let mut value = relation(
            &sandbox
                .path()
                .join(format!("home/.vpt/briefs/Topic {index}.md")),
            Kind::Related,
            State::Suggested,
        );
        value.provenance = Provenance::Proposal {
            by: "agent".into(),
            reason: "reason".into(),
            input: row.digest,
        };
        relations.push(value);
    }
    update(&sandbox, &row.id, relations);
    fixture::write_config(&sandbox, "[note]\nlink_style = 'wiki'\n");
    success(&invoke(&sandbox, row.id.as_str()));
    let first = saved(&sandbox, &row);
    let links = Regions::parse(&first.text).unwrap().links();
    assert_eq!(links.matches("- Related:").count(), 5);
    assert!(links.contains("[[briefs/Topic 0|Topic 0]]"));
    assert!(!links.contains("Topic 5"));
    fixture::write_config(&sandbox, "[relations]\nmax_suggested = 0\n");
    success(&invoke(&sandbox, row.id.as_str()));
    let next = saved(&sandbox, &row);
    assert_eq!(
        Regions::parse(&next.text).unwrap().links(),
        audio_only(&row)
    );
    assert_eq!(first.content_digest, next.content_digest);
}

fn audio_only(record: &RecordingRecord) -> String {
    format!("- Audio: [{0}.m4a](../audio/{0}.m4a)\n", record.id)
}

fn seed_predecessor(sandbox: &Sandbox, current: &RecordingRecord) -> RecordingRecord {
    let ledger =
        SqliteLedger::open(&RootDir::open(&sandbox.path().join("state/vpt")).unwrap()).unwrap();
    let mut previous = fixture::record(sandbox, 1, StageStates::fresh());
    previous.duration_secs = 1;
    ledger
        .commit(&LedgerCommit {
            recordings: vec![previous.clone()],
            ..Default::default()
        })
        .unwrap();
    let mut accepted = ledger.accepted_transcript(&current.id).unwrap().unwrap();
    accepted.recording = previous.id.clone();
    accepted.duration = 1.0;
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
                    recording: Some(previous.id.clone()),
                    outcome: WorkOutcome::Completed
                }
            )]
        ),
        HistoryWrite::Committed(_)
    ));
    previous
}

#[test]
fn note_write_derives_continuation_updates_renamed_target_and_never_falls_back_past_overlap() {
    let (sandbox, row) = configured();
    let previous = seed_predecessor(&sandbox, &row);
    success(&invoke(&sandbox, previous.id.as_str()));
    success(&invoke(&sandbox, row.id.as_str()));
    let first = saved(&sandbox, &row);
    assert!(
        Regions::parse(&first.text)
            .unwrap()
            .links()
            .contains("- Continues: [2026-08-24-title-1-01010101](2026-08-24-title-1-01010101.md)")
    );
    let pin = saved(&sandbox, &previous).target;
    let renamed = pin.with_file_name("Renamed predecessor.md");
    std::fs::rename(&pin, &renamed).unwrap();
    success(&invoke(&sandbox, previous.id.as_str()));
    success(&invoke(&sandbox, row.id.as_str()));
    let next = saved(&sandbox, &row);
    assert!(
        Regions::parse(&next.text)
            .unwrap()
            .links()
            .contains("[Renamed predecessor](Renamed%20predecessor.md)")
    );
    assert_eq!(next.content_digest, first.content_digest);
    fixture::write_config(&sandbox, "[relations]\nsession_gap_minutes = 0\n");
    success(&invoke(&sandbox, row.id.as_str()));
    assert_eq!(
        Regions::parse(&saved(&sandbox, &row).text).unwrap().links(),
        audio_only(&row)
    );
    fixture::write_config(&sandbox, "");
    let ledger =
        SqliteLedger::open(&RootDir::open(&sandbox.path().join("state/vpt")).unwrap()).unwrap();
    let overlapping = fixture::record(&sandbox, 6, StageStates::fresh());
    ledger
        .commit(&LedgerCommit {
            recordings: vec![overlapping],
            ..Default::default()
        })
        .unwrap();
    success(&invoke(&sandbox, row.id.as_str()));
    assert!(ledger.relations(&row.id).unwrap().is_empty());
    assert_eq!(
        Regions::parse(&saved(&sandbox, &row).text).unwrap().links(),
        audio_only(&row)
    );
}

#[test]
fn failed_note_preparation_or_history_commit_preserves_prior_relations_and_note() {
    for late_failure in [false, true] {
        let (sandbox, row) = configured();
        success(&invoke(&sandbox, row.id.as_str()));
        let first = saved(&sandbox, &row);
        seed_predecessor(&sandbox, &row);
        let expected = if late_failure {
            let connection =
                rusqlite::Connection::open(sandbox.path().join("state/vpt/vpt.db")).unwrap();
            fixture::preserve_wal(&connection);
            connection.execute_batch("CREATE TRIGGER refuse_relation_note_history BEFORE INSERT ON history_records WHEN instr(NEW.fact, '\"tag\":\"work\"') > 0 BEGIN SELECT RAISE(ABORT, 'Private canary'); END;").unwrap();
            first.text.clone()
        } else {
            let edited = first.text.replace("<!-- vpt:content end -->", "removed");
            std::fs::write(&first.target, &edited).unwrap();
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
        assert_eq!(saved(&sandbox, &row), first);
        assert_eq!(std::fs::read_to_string(&first.target).unwrap(), expected);
        let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
            .unwrap()
            .unwrap();
        assert!(ledger.relations(&row.id).unwrap().is_empty());
        assert!(ledger.pending_publications().unwrap().is_empty());
    }
}

#[test]
fn manual_relations_survive_derivation_and_unpublished_recording_targets_are_visible() {
    let (sandbox, row) = configured();
    let previous = seed_predecessor(&sandbox, &row);
    let manual = Relation {
        kind: Kind::Continues,
        target: Target::Recording(previous.id.clone()),
        state: State::Confirmed,
        provenance: Provenance::Manual,
        ranges: vec![],
    };
    let supersedes = Relation {
        kind: Kind::Supersedes,
        ..manual.clone()
    };
    update(&sandbox, &row.id, vec![manual.clone(), supersedes.clone()]);
    success(&invoke(&sandbox, row.id.as_str()));
    let note = saved(&sandbox, &row);
    let links = Regions::parse(&note.text).unwrap().links();
    assert!(links.contains(&format!("- Continues: {} (note unavailable)", previous.id)));
    assert!(links.contains(&format!("- Supersedes: {} (note unavailable)", previous.id)));
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    assert_eq!(ledger.relations(&row.id).unwrap(), [manual, supersedes]);
}

#[test]
fn tag_confirmation_refreshes_continuation_with_the_note_in_the_same_commit() {
    let (sandbox, row) = configured();
    success(&invoke(&sandbox, row.id.as_str()));
    let previous = seed_predecessor(&sandbox, &row);
    let path = sandbox.config_path().with_file_name("known-tags.txt");
    fixture::write_config(&sandbox, &format!("[tags]\nknown_tags_path = {path:?}\n"));
    let output = support::run_with_budget(
        sandbox
            .vpt()
            .args(["confirm", row.id.as_str(), "--tag", "topic", "--json"]),
        std::time::Duration::from_secs(5),
    );
    success(&output);
    let note = saved(&sandbox, &row);
    assert!(
        Regions::parse(&note.text)
            .unwrap()
            .links()
            .contains(&format!("- Continues: {} (note unavailable)", previous.id))
    );
    assert_eq!(note.managed_tags, ["topic"]);
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    assert_eq!(ledger.relations(&row.id).unwrap().len(), 1);
}
