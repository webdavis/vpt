use super::record;
use vpt_application::ports::*;
use vpt_domain::{
    run::{HistoryFact, RunKind, WorkOutcome},
    tags::{Provenance, State, Tag},
    time::UtcInstant,
};

mod actions;
mod learned;
mod renames;
pub(crate) use actions::*;
pub(crate) use learned::*;
pub(crate) use renames::*;

pub(crate) fn value() -> Tag {
    Tag {
        name: "Private_Tag".into(),
        state: State::Suggested,
        provenance: Provenance::Proposal {
            by: "PrivateAgent".into(),
            reason: "Private reason".into(),
            input: super::digest(6),
        },
        updated_at: Some(UtcInstant { secs: 50 }),
    }
}

pub(crate) fn change() -> TagChange {
    TagChange {
        recording: record(81).id,
        expected: None,
        tags: vec![value()],
        at: UtcInstant { secs: 50 },
    }
}

pub(crate) fn facts() -> Vec<(UtcInstant, HistoryFact)> {
    vec![(
        UtcInstant { secs: 50 },
        HistoryFact::WorkStep {
            recording: Some(record(81).id),
            outcome: WorkOutcome::Completed,
        },
    )]
}

pub(crate) fn seed(ledger: &(impl RecordingLedger + RunHistory)) -> vpt_domain::history::RunId {
    ledger
        .commit(&LedgerCommit {
            recordings: vec![record(81)],
            ..Default::default()
        })
        .unwrap();
    let HistoryWrite::Committed(receipt) =
        ledger.begin_run(RunKind::Confirm, UtcInstant { secs: 49 })
    else {
        panic!("run");
    };
    receipt.run
}

pub(crate) fn tags_and_private_provenance_commit_with_history(
    ledger: &(impl RecordingLedger + RunHistory + TagLedger),
) {
    let run = seed(ledger);
    assert_eq!(ledger.tags(&record(81).id).unwrap(), None);
    let delta = WorkDelta {
        tags: Some(change()),
        ..Default::default()
    };
    let HistoryWrite::Committed(receipt) = ledger.commit_step(run, &delta, &facts()) else {
        panic!("commit");
    };
    assert_eq!(receipt.completed, [record(81).id]);
    assert_eq!(ledger.tags(&record(81).id).unwrap(), Some(vec![value()]));
    assert!(!format!("{delta:?} {:?}", ledger.pending_exports().unwrap()).contains("Private"));
}

pub(crate) fn stale_or_mismatched_tag_changes_leave_saved_state_unchanged(
    ledger: &(impl RecordingLedger + RunHistory + TagLedger),
) {
    let run = seed(ledger);
    let delta = WorkDelta {
        tags: Some(change()),
        ..Default::default()
    };
    assert!(matches!(
        ledger.commit_step(run, &delta, &facts()),
        HistoryWrite::Committed(_)
    ));
    assert!(matches!(
        ledger.commit_step(run, &delta, &facts()),
        HistoryWrite::Unconfirmed(_)
    ));
    for phase in 0..3 {
        let mut change = change();
        change.expected = Some(vec![value()]);
        change.tags.clear();
        let mut facts = facts();
        match phase {
            0 => facts[0].0.secs += 1,
            1 => {
                facts[0].1 = HistoryFact::WorkStep {
                    recording: Some(record(80).id),
                    outcome: WorkOutcome::Completed,
                }
            }
            _ => {
                facts[0].1 = HistoryFact::WorkStep {
                    recording: Some(record(81).id),
                    outcome: WorkOutcome::Failed,
                }
            }
        }
        assert!(matches!(
            ledger.commit_step(
                run,
                &WorkDelta {
                    tags: Some(change),
                    ..Default::default()
                },
                &facts
            ),
            HistoryWrite::Unconfirmed(_)
        ));
    }
    assert_eq!(ledger.tags(&record(81).id).unwrap(), Some(vec![value()]));
}

pub(crate) fn tag_state_rolls_back_after_late_history_failure(
    ledger: &(impl RecordingLedger + RunHistory + TagLedger),
) {
    let run = seed(ledger);
    let mut facts = facts();
    facts.push((
        UtcInstant { secs: 50 },
        HistoryFact::HookFailed {
            event: vpt_domain::history::EventId::new(run.namespace(), 999),
            failure: vpt_domain::run::HookFailureClass::Spawn,
        },
    ));
    assert!(matches!(
        ledger.commit_step(
            run,
            &WorkDelta {
                tags: Some(change()),
                ..Default::default()
            },
            &facts
        ),
        HistoryWrite::Unconfirmed(_)
    ));
    assert_eq!(ledger.tags(&record(81).id).unwrap(), None);
    assert_eq!(ledger.pending_exports().unwrap().len(), 1);
}

pub(crate) fn invalid_duplicate_and_oversized_tag_state_is_refused(
    ledger: &(impl RecordingLedger + RunHistory + TagLedger),
) {
    let run = seed(ledger);
    for phase in 0..5 {
        let mut change = change();
        match phase {
            0 => change.tags[0].name = "Private#tag".into(),
            1 => {
                let mut duplicate = value();
                duplicate.name = "private-tag".into();
                change.tags.push(duplicate);
            }
            2 => change.tags = vec![value(); 257],
            3 => {
                change.tags[0].provenance = Provenance::Proposal {
                    by: "".into(),
                    reason: "reason".into(),
                    input: super::digest(6),
                }
            }
            _ => change.tags[0].name = "a".repeat(65_537),
        }
        let error = ledger.commit_step(
            run,
            &WorkDelta {
                tags: Some(change),
                ..Default::default()
            },
            &facts(),
        );
        assert!(matches!(error, HistoryWrite::Unconfirmed(_)));
        assert!(!format!("{error:?}").contains("Private"));
        assert_eq!(ledger.tags(&record(81).id).unwrap(), None);
    }
}

pub(crate) fn confirmed_rejected_and_empty_tag_sets_round_trip(
    ledger: &(impl RecordingLedger + RunHistory + TagLedger),
) {
    let run = seed(ledger);
    let mut expected = None;
    for tags in [
        vec![Tag {
            state: State::Confirmed,
            provenance: Provenance::Manual,
            ..value()
        }],
        vec![Tag {
            state: State::Rejected,
            provenance: Provenance::Manual,
            ..value()
        }],
        vec![],
    ] {
        let mut change = change();
        change.expected = expected;
        change.tags = tags.clone();
        assert!(matches!(
            ledger.commit_step(
                run,
                &WorkDelta {
                    tags: Some(change),
                    ..Default::default()
                },
                &facts()
            ),
            HistoryWrite::Committed(_)
        ));
        assert_eq!(
            ledger.tag_states().unwrap(),
            tags.iter()
                .map(|tag| (tag.name.clone(), tag.state))
                .collect::<Vec<_>>()
        );
        expected = Some(tags);
        assert_eq!(ledger.tags(&record(81).id).unwrap(), expected);
    }
}

macro_rules! cases {
    ($case:ident) => {
        $case!(tag_note_and_vocabulary_changes_commit_together);
        $case!(tag_note_and_vocabulary_changes_roll_back_together);
        $case!(tag_changes_make_successful_notes_pending);
        $case!(tag_changes_leave_expired_notes_expired);
        $case!(tags_and_private_provenance_commit_with_history);
        $case!(stale_or_mismatched_tag_changes_leave_saved_state_unchanged);
        $case!(tag_state_rolls_back_after_late_history_failure);
        $case!(invalid_duplicate_and_oversized_tag_state_is_refused);
        $case!(confirmed_rejected_and_empty_tag_sets_round_trip);
        $case!(tag_renames_are_remembered_and_the_latest_rename_of_a_name_wins);
        $case!(learned_tags_are_remembered_until_forgotten);
        $case!(an_action_is_pending_until_settled_and_again_only_when_its_digest_changes);
    };
}
pub(crate) use cases;

pub(crate) fn tag_changes_make_successful_notes_pending(
    ledger: &(impl RecordingLedger + RunHistory + TagLedger),
) {
    invalidation(ledger, StageState::Succeeded, StageState::Pending);
}

pub(crate) fn tag_note_and_vocabulary_changes_commit_together(
    ledger: &(
         impl RecordingLedger
         + RunHistory
         + TagLedger
         + NoteLedger
         + VocabularyLedger
         + PublicationJournal
     ),
) {
    coupled(ledger, false);
}
pub(crate) fn tag_note_and_vocabulary_changes_roll_back_together(
    ledger: &(
         impl RecordingLedger
         + RunHistory
         + TagLedger
         + NoteLedger
         + VocabularyLedger
         + PublicationJournal
     ),
) {
    coupled(ledger, true);
}
fn coupled(
    ledger: &(
         impl RecordingLedger
         + RunHistory
         + TagLedger
         + NoteLedger
         + VocabularyLedger
         + PublicationJournal
     ),
    fail: bool,
) {
    let run = seed(ledger);
    let mut delta = super::notes::delta();
    let mut change = change();
    change.tags[0].name = "managed".into();
    change.tags[0].state = State::Confirmed;
    delta.tags = Some(change.clone());
    delta.vocabulary = Some(super::known_terms::tag_publication());
    let mut facts = facts();
    facts.extend(super::known_terms::tag_facts());
    if fail {
        facts.push((
            UtcInstant { secs: 50 },
            HistoryFact::HookFailed {
                event: vpt_domain::history::EventId::new(run.namespace(), 999),
                failure: vpt_domain::run::HookFailureClass::Spawn,
            },
        ));
    }
    let result = ledger.commit_step(run, &delta, &facts);
    let note = super::notes::artifact();
    let terms = super::known_terms::tag_publication();
    if fail {
        assert!(matches!(result, HistoryWrite::Unconfirmed(_)));
        assert_eq!(ledger.tags(&note.recording).unwrap(), None);
        assert_eq!(ledger.note_at(&note.target).unwrap(), None);
        assert_eq!(
            ledger.vocabulary_publication(&terms.entry.target).unwrap(),
            None
        );
        assert!(ledger.pending_publications().unwrap().is_empty());
    } else {
        let HistoryWrite::Committed(receipt) = result else {
            panic!("commit");
        };
        assert_eq!(receipt.completed, std::slice::from_ref(&note.recording));
        assert_eq!(ledger.tags(&note.recording).unwrap(), Some(change.tags));
        assert_eq!(ledger.note_at(&note.target).unwrap(), Some(note));
        assert_eq!(
            ledger.vocabulary_publication(&terms.entry.target).unwrap(),
            Some(terms)
        );
        assert_eq!(ledger.pending_publications().unwrap().len(), 2);
    }
}
pub(crate) fn tag_changes_leave_expired_notes_expired(
    ledger: &(impl RecordingLedger + RunHistory + TagLedger),
) {
    invalidation(ledger, StageState::Expired, StageState::Expired);
}
fn invalidation(
    ledger: &(impl RecordingLedger + RunHistory + TagLedger),
    initial: StageState,
    expected: StageState,
) {
    let mut row = record(81);
    row.stages.note = initial;
    ledger
        .commit(&LedgerCommit {
            recordings: vec![row.clone()],
            ..Default::default()
        })
        .unwrap();
    let HistoryWrite::Committed(run) = ledger.begin_run(RunKind::Confirm, UtcInstant { secs: 49 })
    else {
        panic!("run");
    };
    assert!(matches!(
        ledger.commit_step(
            run.run,
            &WorkDelta {
                tags: Some(change()),
                ..Default::default()
            },
            &facts()
        ),
        HistoryWrite::Committed(_)
    ));
    assert_eq!(
        ledger.by_id(&row.id).unwrap().unwrap().stages.note,
        expected
    );
}
