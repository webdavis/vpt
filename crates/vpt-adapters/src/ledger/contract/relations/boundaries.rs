use super::*;

pub(crate) fn noncanonical_note_targets_are_refused(
    ledger: &(impl RecordingLedger + RunHistory + RelationLedger),
) {
    let run = seed(ledger);
    for path in [
        "/vault/./Private.md",
        "/vault//Private.md",
        "/vault/Private.md/",
        "relative.md",
        "/",
    ] {
        let mut delta = delta();
        delta.relations.as_mut().unwrap().relations[1].target = Target::Note(path.into());
        assert!(
            matches!(
                ledger.commit_step(run, &delta, &facts()),
                HistoryWrite::Unconfirmed(_)
            ),
            "{path}"
        );
        assert!(ledger.relations(&record(81).id).unwrap().is_empty());
    }
}

pub(crate) fn relation_order_is_canonical_and_unchanged_relations_keep_note_success(
    ledger: &(impl RecordingLedger + RunHistory + RelationLedger + NoteLedger + PublicationJournal),
) {
    let run = seed(ledger);
    let mut delta = delta();
    delta.relations.as_mut().unwrap().relations.reverse();
    assert!(matches!(
        ledger.commit_step(run, &delta, &facts()),
        HistoryWrite::Committed(_)
    ));
    assert_eq!(ledger.relations(&record(81).id).unwrap(), values());
    let note = crate::ledger::contract::notes::delta();
    assert!(matches!(
        ledger.commit_step(run, &note, &facts()),
        HistoryWrite::Committed(_)
    ));
    use vpt_domain::run::{EffectKind, EffectOutcome, EffectReference, HistoryFact};
    let at = UtcInstant { secs: 60 };
    let HistoryWrite::Committed(intent) = ledger.commit_step(
        run,
        &WorkDelta::default(),
        &[(
            at,
            HistoryFact::EffectIntent {
                kind: EffectKind::Publication,
                reference: EffectReference::PublicationTarget(
                    note.notes[0].artifact.target.clone(),
                ),
            },
        )],
    ) else {
        panic!("intent");
    };
    assert!(matches!(
        ledger.commit_step(
            run,
            &WorkDelta {
                publication_clears: vec![note.notes[0].artifact.target.clone()],
                ..Default::default()
            },
            &[(
                at,
                HistoryFact::EffectObserved {
                    intent: intent.records[0].id,
                    outcome: EffectOutcome::Completed
                }
            )]
        ),
        HistoryWrite::Committed(_)
    ));
    delta.relations.as_mut().unwrap().expected = values();
    assert!(matches!(
        ledger.commit_step(run, &delta, &facts()),
        HistoryWrite::Committed(_)
    ));
    assert_eq!(
        ledger.by_id(&record(81).id).unwrap().unwrap().stages.note,
        StageState::Succeeded
    );
}

pub(crate) fn relations_and_note_publication_commit_or_roll_back_together(
    ledger: &(impl RecordingLedger + RunHistory + RelationLedger + NoteLedger + PublicationJournal),
) {
    let run = seed(ledger);
    let mut delta = crate::ledger::contract::notes::delta();
    delta.relations = super::delta().relations;
    let mut bad = facts();
    bad.push((
        UtcInstant { secs: 50 },
        vpt_domain::run::HistoryFact::HookFailed {
            event: vpt_domain::history::EventId::new(run.namespace(), 999),
            failure: vpt_domain::run::HookFailureClass::Spawn,
        },
    ));
    assert!(matches!(
        ledger.commit_step(run, &delta, &bad),
        HistoryWrite::Unconfirmed(_)
    ));
    assert!(ledger.relations(&record(81).id).unwrap().is_empty());
    assert!(
        ledger
            .note_at(&delta.notes[0].artifact.target)
            .unwrap()
            .is_none()
    );
    assert!(ledger.pending_publications().unwrap().is_empty());
    assert!(matches!(
        ledger.commit_step(run, &delta, &facts()),
        HistoryWrite::Committed(_)
    ));
    assert_eq!(ledger.relations(&record(81).id).unwrap(), values());
    assert_eq!(
        ledger.note_at(&delta.notes[0].artifact.target).unwrap(),
        Some(delta.notes[0].artifact.clone())
    );
    assert_eq!(ledger.pending_publications().unwrap().len(), 1);
}

pub(crate) fn relation_capacity_and_range_limits_are_enforced(
    ledger: &(impl RecordingLedger + RunHistory + RelationLedger),
) {
    let run = seed(ledger);
    for phase in 0..5 {
        let mut delta = delta();
        let change = delta.relations.as_mut().unwrap();
        match phase {
            0 => change.relations = vec![values()[0].clone(); 65_537],
            1 => change.relations[1].ranges = vec![0..1; 65_537],
            2 => change.relations[1].ranges = vec![2..4, 0..1],
            3 => change.relations[1].ranges = vec![0..1, 0..1],
            _ => {
                change.relations[1].ranges = (0..65_536).map(|n| n..n + 1).collect();
                let mut other = change.relations[1].clone();
                other.target = Target::Note("/vault/Other.md".into());
                other.ranges = std::iter::once(0..1).collect();
                change.relations.push(other);
            }
        }
        assert!(
            matches!(
                ledger.commit_step(run, &delta, &facts()),
                HistoryWrite::Unconfirmed(_)
            ),
            "phase {phase}"
        );
        assert!(ledger.relations(&record(81).id).unwrap().is_empty());
    }
}

pub(crate) fn note_inputs_choose_the_immediate_predecessor_before_testing_its_duration(
    ledger: &(impl RecordingLedger + NoteInputsLedger),
) {
    let current = record(81);
    let make = |seed, offset| {
        let mut row = record(seed);
        row.captured_at.secs = current.captured_at.secs - 100;
        row.captured_offset.secs = offset;
        row.id = vpt_domain::identity::RecordingId::derive(
            row.captured_at,
            row.captured_offset,
            &row.digest,
        )
        .unwrap();
        row.audio_path = format!("/audio/{}.m4a", row.id).into();
        row.duration_secs = 200;
        row
    };
    let low = make(80, -3600);
    let high = make(82, 3600);
    let later = record(90);
    assert!(low.id.as_str() < high.id.as_str());
    ledger
        .commit(&LedgerCommit {
            recordings: vec![high.clone(), current.clone(), later, low],
            ..Default::default()
        })
        .unwrap();
    let inputs = ledger.note_inputs(&current.id).unwrap().unwrap();
    let predecessor = inputs.predecessor.unwrap();
    assert_eq!(predecessor.record, high);
    assert!(predecessor.path.is_none());
}
