use super::*;
use vpt_domain::relations::MentionAmbiguity;

pub(crate) fn ambiguity() -> MentionAmbiguity {
    MentionAmbiguity {
        term: "Private Person".into(),
        ranges: vec![1..3, 7..9],
    }
}
pub(crate) fn ambiguity_delta() -> WorkDelta {
    let mut delta = delta();
    delta.relations.as_mut().unwrap().ambiguities = Some(AmbiguityChange {
        expected: vec![],
        ambiguities: vec![ambiguity()],
    });
    delta
}
pub(crate) fn ambiguities_and_relations_commit_together_without_private_debug_text(
    ledger: &(impl RecordingLedger + RunHistory + RelationLedger + NoteInputsLedger),
) {
    let run = seed(ledger);
    let delta = ambiguity_delta();
    assert!(matches!(
        ledger.commit_step(run, &delta, &facts()),
        HistoryWrite::Committed(_)
    ));
    assert_eq!(
        ledger.mention_ambiguities(&record(81).id).unwrap(),
        [ambiguity()]
    );
    assert_eq!(ledger.relations(&record(81).id).unwrap(), values());
    assert_eq!(
        ledger
            .note_inputs(&record(81).id)
            .unwrap()
            .unwrap()
            .mention_ambiguities,
        [ambiguity()]
    );
    assert!(!format!("{delta:?} {:?}", ledger.pending_exports().unwrap()).contains("Private"));
    let mut next = delta.clone();
    let change = next.relations.as_mut().unwrap();
    change.expected = values();
    change.relations.clear();
    change.ambiguities = None;
    assert!(matches!(
        ledger.commit_step(run, &next, &facts()),
        HistoryWrite::Committed(_)
    ));
    assert_eq!(
        ledger.mention_ambiguities(&record(81).id).unwrap(),
        [ambiguity()]
    );
    let change = next.relations.as_mut().unwrap();
    change.expected.clear();
    change.ambiguities = Some(AmbiguityChange {
        expected: vec![ambiguity()],
        ambiguities: vec![],
    });
    assert!(matches!(
        ledger.commit_step(run, &next, &facts()),
        HistoryWrite::Committed(_)
    ));
    assert!(
        ledger
            .mention_ambiguities(&record(81).id)
            .unwrap()
            .is_empty()
    );
}

pub(crate) fn ambiguity_updates_refuse_stale_expected_state_and_roll_back_on_history_failure(
    ledger: &(impl RecordingLedger + RunHistory + RelationLedger),
) {
    let run = seed(ledger);
    let mut facts = facts();
    facts.push((
        UtcInstant { secs: 50 },
        vpt_domain::run::HistoryFact::HookFailed {
            event: vpt_domain::history::EventId::new(run.namespace(), 999),
            failure: vpt_domain::run::HookFailureClass::Spawn,
        },
    ));
    assert!(matches!(
        ledger.commit_step(run, &ambiguity_delta(), &facts),
        HistoryWrite::Unconfirmed(_)
    ));
    assert!(
        ledger
            .mention_ambiguities(&record(81).id)
            .unwrap()
            .is_empty()
    );
    assert!(ledger.relations(&record(81).id).unwrap().is_empty());
    facts.pop();
    assert!(matches!(
        ledger.commit_step(run, &ambiguity_delta(), &facts),
        HistoryWrite::Committed(_)
    ));
    let mut delta = ambiguity_delta();
    delta.relations.as_mut().unwrap().expected = values();
    delta
        .relations
        .as_mut()
        .unwrap()
        .ambiguities
        .as_mut()
        .unwrap()
        .ambiguities
        .clear();
    assert!(matches!(
        ledger.commit_step(run, &delta, &facts),
        HistoryWrite::Unconfirmed(_)
    ));
    assert_eq!(
        ledger.mention_ambiguities(&record(81).id).unwrap(),
        [ambiguity()]
    );
}

pub(crate) fn ambiguous_terms_and_source_ranges_are_bounded_and_validated(
    ledger: &(impl RecordingLedger + RunHistory + RelationLedger),
) {
    let run = seed(ledger);
    for case in 0..10 {
        let mut delta = ambiguity_delta();
        let list = &mut delta
            .relations
            .as_mut()
            .unwrap()
            .ambiguities
            .as_mut()
            .unwrap()
            .ambiguities;
        match case {
            0 => list[0].term = " ".into(),
            1 => list[0].term = "Private\0".into(),
            2 => list[0].ranges.clear(),
            3 => list[0].ranges = std::iter::once(std::ops::Range { start: 2, end: 1 }).collect(),
            4 => list[0].ranges = std::iter::once(0..65_537).collect(),
            5 => list.push(MentionAmbiguity {
                term: "PRIVATE  PERSON".into(),
                ..ambiguity()
            }),
            6 => list[0].term = "x".repeat(65_537),
            7 => *list = vec![ambiguity(); 65_537],
            8 => list[0].ranges = vec![0..1; 65_537],
            _ => list[0].term = "ﷺ".repeat(4_000),
        }
        let error = ledger.commit_step(run, &delta, &facts());
        assert!(matches!(error, HistoryWrite::Unconfirmed(_)), "case {case}");
        assert!(!format!("{error:?}").contains("Private"));
        assert!(ledger.relations(&record(81).id).unwrap().is_empty());
        assert!(
            ledger
                .mention_ambiguities(&record(81).id)
                .unwrap()
                .is_empty()
        );
    }
}

pub(crate) fn ambiguity_only_changes_invalidate_unexpired_notes(
    ledger: &(impl RecordingLedger + RunHistory + RelationLedger),
) {
    for (seed, state, expected) in [
        (81, StageState::Succeeded, StageState::Pending),
        (82, StageState::Expired, StageState::Expired),
    ] {
        let mut row = record(seed);
        row.stages.note = state;
        ledger
            .commit(&LedgerCommit {
                recordings: vec![row.clone()],
                ..Default::default()
            })
            .unwrap();
        let at = UtcInstant { secs: 50 };
        let HistoryWrite::Committed(run) = ledger.begin_run(vpt_domain::run::RunKind::Note, at)
        else {
            panic!("run");
        };
        let mut delta = ambiguity_delta();
        let change = delta.relations.as_mut().unwrap();
        change.recording = row.id.clone();
        change.relations.clear();
        let facts = [(
            at,
            vpt_domain::run::HistoryFact::WorkStep {
                recording: Some(row.id.clone()),
                outcome: vpt_domain::run::WorkOutcome::Completed,
            },
        )];
        assert!(matches!(
            ledger.commit_step(run.run, &delta, &facts),
            HistoryWrite::Committed(_)
        ));
        assert_eq!(
            ledger.by_id(&row.id).unwrap().unwrap().stages.note,
            expected
        );
    }
}
