mod ambiguities;
mod boundaries;
use super::{
    record,
    tags::{facts, seed},
};
pub(crate) use ambiguities::*;
pub(crate) use boundaries::*;
use vpt_application::ports::*;
use vpt_domain::{relations::*, time::UtcInstant};

pub(crate) fn values() -> Vec<Relation> {
    vec![
        Relation {
            kind: Kind::Continues,
            target: Target::Recording(record(80).id),
            state: State::Confirmed,
            provenance: Provenance::Derived(Rule::ImmediatePredecessor),
            ranges: vec![],
        },
        Relation {
            kind: Kind::Mentions,
            target: Target::Note("/vault/Private note.md".into()),
            state: State::Confirmed,
            provenance: Provenance::Derived(Rule::KnownTerm),
            ranges: vec![0..1, 2..4],
        },
        Relation {
            kind: Kind::Related,
            target: Target::Recording(record(80).id),
            state: State::Suggested,
            provenance: Provenance::Proposal {
                by: "Private agent".into(),
                reason: "Private reason".into(),
                input: super::digest(2),
            },
            ranges: vec![],
        },
        Relation {
            kind: Kind::Supersedes,
            target: Target::Recording(record(80).id),
            state: State::Confirmed,
            provenance: Provenance::Manual,
            ranges: vec![],
        },
    ]
}
pub(crate) fn delta() -> WorkDelta {
    WorkDelta {
        relations: Some(RelationChange {
            ambiguities: None,
            recording: record(81).id,
            expected: vec![],
            relations: values(),
            at: UtcInstant { secs: 50 },
        }),
        ..Default::default()
    }
}

pub(crate) fn relation_state_commits_with_history_and_private_provenance(
    ledger: &(impl RecordingLedger + RunHistory + RelationLedger + NoteInputsLedger),
) {
    let run = seed(ledger);
    let HistoryWrite::Committed(receipt) = ledger.commit_step(run, &delta(), &facts()) else {
        panic!("commit");
    };
    assert_eq!(receipt.completed, [record(81).id]);
    assert_eq!(ledger.relations(&record(81).id).unwrap(), values());
    let input = ledger.note_inputs(&record(81).id).unwrap().unwrap();
    assert_eq!(
        input
            .relations
            .iter()
            .map(|item| item.relation.clone())
            .collect::<Vec<_>>(),
        values()
    );
    assert!(input.relations[0].path.is_none());
    assert_eq!(
        input.relations[1].path.as_deref(),
        Some(std::path::Path::new("/vault/Private note.md"))
    );
    assert!(!format!("{:?} {:?}", delta(), ledger.pending_exports().unwrap()).contains("Private"));
}

pub(crate) fn relation_updates_reject_stale_state_and_missing_success_history(
    ledger: &(impl RecordingLedger + RunHistory + RelationLedger),
) {
    let run = seed(ledger);
    assert!(matches!(
        ledger.commit_step(run, &delta(), &facts()),
        HistoryWrite::Committed(_)
    ));
    assert!(matches!(
        ledger.commit_step(run, &delta(), &facts()),
        HistoryWrite::Unconfirmed(_)
    ));
    let mut next = delta();
    next.relations.as_mut().unwrap().expected = values();
    next.relations.as_mut().unwrap().relations.clear();
    let mut wrong = facts();
    wrong[0].0.secs += 1;
    assert!(matches!(
        ledger.commit_step(run, &next, &wrong),
        HistoryWrite::Unconfirmed(_)
    ));
    assert_eq!(ledger.relations(&record(81).id).unwrap(), values());
    assert!(matches!(
        ledger.commit_step(run, &next, &facts()),
        HistoryWrite::Committed(_)
    ));
    assert!(ledger.relations(&record(81).id).unwrap().is_empty());
}

pub(crate) fn relation_state_rolls_back_after_late_history_failure(
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
        ledger.commit_step(run, &delta(), &facts),
        HistoryWrite::Unconfirmed(_)
    ));
    assert!(ledger.relations(&record(81).id).unwrap().is_empty());
    assert_eq!(ledger.pending_exports().unwrap().len(), 1);
}

pub(crate) fn relation_changes_invalidate_only_unexpired_notes(
    ledger: &(impl RecordingLedger + RunHistory + RelationLedger),
) {
    for (n, state, expected) in [
        (81, StageState::Succeeded, StageState::Pending),
        (82, StageState::Expired, StageState::Expired),
    ] {
        let mut row = record(n);
        row.stages.note = state;
        ledger
            .commit(&LedgerCommit {
                recordings: vec![row.clone()],
                ..Default::default()
            })
            .unwrap();
        let HistoryWrite::Committed(run) =
            ledger.begin_run(vpt_domain::run::RunKind::Confirm, UtcInstant { secs: 49 })
        else {
            panic!("run");
        };
        let mut delta = delta();
        delta.relations.as_mut().unwrap().recording = row.id.clone();
        let facts = [(
            UtcInstant { secs: 50 },
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

pub(crate) fn invalid_relations_are_refused_without_private_diagnostics(
    ledger: &(impl RecordingLedger + RunHistory + RelationLedger),
) {
    let run = seed(ledger);
    for case in 0..8 {
        let mut delta = delta();
        let change = delta.relations.as_mut().unwrap();
        match case {
            0 => change.relations.push(values()[0].clone()),
            1 => change.relations[3].provenance = Provenance::Derived(Rule::KnownTerm),
            2 => change.relations[1].ranges = std::iter::once(4..4).collect(),
            3 => change.relations[1].ranges = std::iter::once(0..65_537).collect(),
            4 => change.relations[1].target = Target::Note("/vault/../Private.md".into()),
            5 => change.relations[1].target = Target::Note("/vault/Private".repeat(65_537)),
            6 => {
                change.relations[2].provenance = Provenance::Proposal {
                    by: "".into(),
                    reason: "Private".into(),
                    input: super::digest(2),
                }
            }
            _ => change.relations[0].target = Target::Recording(change.recording.clone()),
        }
        let result = ledger.commit_step(run, &delta, &facts());
        assert!(
            matches!(result, HistoryWrite::Unconfirmed(_)),
            "case {case}"
        );
        assert!(!format!("{result:?}").contains("Private"));
        assert!(ledger.relations(&record(81).id).unwrap().is_empty());
    }
}

macro_rules! cases {
    ($case:ident) => {
        $case!(relation_state_commits_with_history_and_private_provenance);
        $case!(ambiguities_and_relations_commit_together_without_private_debug_text);
        $case!(ambiguity_updates_refuse_stale_expected_state_and_roll_back_on_history_failure);
        $case!(ambiguous_terms_and_source_ranges_are_bounded_and_validated);
        $case!(ambiguity_only_changes_invalidate_unexpired_notes);
        $case!(note_inputs_choose_the_immediate_predecessor_before_testing_its_duration);
        $case!(relation_order_is_canonical_and_unchanged_relations_keep_note_success);
        $case!(noncanonical_note_targets_are_refused);
        $case!(relations_and_note_publication_commit_or_roll_back_together);
        $case!(relation_capacity_and_range_limits_are_enforced);
        $case!(relation_updates_reject_stale_state_and_missing_success_history);
        $case!(relation_state_rolls_back_after_late_history_failure);
        $case!(relation_changes_invalidate_only_unexpired_notes);
        $case!(invalid_relations_are_refused_without_private_diagnostics);
    };
}
pub(crate) use cases;
