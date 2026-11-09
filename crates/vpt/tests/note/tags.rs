use super::*;
use vpt_domain::tags::{Provenance, State, Tag};

pub(super) fn update(sandbox: &Sandbox, id: &vpt_domain::identity::RecordingId, tags: Vec<Tag>) {
    let root = RootDir::open(&sandbox.path().join("state/vpt")).unwrap();
    let ledger = SqliteLedger::open(&root).unwrap();
    let at = UtcInstant { secs: 100 };
    let HistoryWrite::Committed(run) = ledger.begin_run(RunKind::Confirm, at) else {
        panic!("run");
    };
    let change = TagChange {
        recording: id.clone(),
        expected: ledger.tags(id).unwrap(),
        tags,
        at,
    };
    assert!(matches!(
        ledger.commit_step(
            run.run,
            &WorkDelta {
                tags: Some(change),
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

pub(super) fn tag(name: &str, state: State) -> Tag {
    Tag {
        name: name.into(),
        state,
        provenance: Provenance::Manual,
        updated_at: Some(UtcInstant { secs: 100 }),
    }
}

#[test]
fn saved_tag_states_render_and_rejection_preserves_operator_tags() {
    let (sandbox, row) = configured();
    update(
        &sandbox,
        &row.id,
        vec![
            tag("Project_Finance", State::Confirmed),
            tag("future-topic", State::Suggested),
            tag("rejected-topic", State::Rejected),
        ],
    );
    success(&invoke(&sandbox, row.id.as_str()));
    let first = saved(&sandbox, &row);
    assert_eq!(
        vpt_adapters::note_document::Document::parse(&first.text)
            .unwrap()
            .tags,
        ["Project_Finance"]
    );
    assert_eq!(first.managed_tags, ["Project_Finance"]);
    assert!(first.text.contains("future-topic"));
    assert!(!first.text.contains("rejected-topic"));
    let edited = first.text.replace(
        "tags:\n  - \"Project_Finance\"",
        "tags: [Project_Finance, operator]",
    );
    assert_ne!(edited, first.text);
    std::fs::write(&first.target, edited).unwrap();
    update(
        &sandbox,
        &row.id,
        vec![tag("Project_Finance", State::Rejected)],
    );
    success(&invoke(&sandbox, row.id.as_str()));
    let second = saved(&sandbox, &row);
    assert_eq!(
        vpt_adapters::note_document::Document::parse(&second.text)
            .unwrap()
            .tags,
        ["operator"]
    );
    assert!(second.managed_tags.is_empty());
    assert!(!second.text.contains("future-topic"));
    assert_eq!(second.content_digest, first.content_digest);
}
