use super::{record, seed};
use vpt_application::ports::*;
use vpt_domain::{
    history::RunId,
    identity::RecordingId,
    run::{HistoryFact, WorkOutcome},
    time::UtcInstant,
};

fn commits(
    ledger: &(impl RecordingLedger + RunHistory + ActionLedger),
    run: RunId,
    recording: &RecordingId,
    change: ActionChange,
) -> bool {
    let write = ledger.commit_step(
        run,
        &WorkDelta {
            actions: Some(change),
            ..Default::default()
        },
        &[(
            UtcInstant { secs: 50 },
            HistoryFact::WorkStep {
                recording: Some(recording.clone()),
                outcome: WorkOutcome::Completed,
            },
        )],
    );
    matches!(write, HistoryWrite::Committed(_))
}

fn queued(recording: &RecordingId, id: &str, digest: char) -> QueuedAction {
    QueuedAction {
        recording: recording.clone(),
        id: id.into(),
        digest: digest.to_string().repeat(64),
    }
}

fn ids(actions: Vec<QueuedAction>) -> Vec<(String, char)> {
    actions
        .into_iter()
        .map(|queued| (queued.id, queued.digest.chars().next().unwrap()))
        .collect()
}

pub(crate) fn an_action_is_pending_until_settled_and_again_only_when_its_digest_changes(
    ledger: &(impl RecordingLedger + RunHistory + ActionLedger),
) {
    let run = seed(ledger);
    let recording = record(81).id;
    let action = |id: &str, digest| queued(&recording, id, digest);
    let committed = |change| commits(ledger, run, &recording, change);
    let queue = |actions| ActionChange::Queue {
        recording: recording.clone(),
        actions,
        dropped: vec![],
    };
    let settle = |id, digest, refused| ActionChange::Settle {
        action: action(id, digest),
        refused,
    };
    let pending = || ids(ledger.pending_actions().unwrap());
    assert!(committed(queue(vec![
        action("a", '1'),
        action("b", '1'),
        action("c", '1'),
    ])));
    assert!(committed(settle("a", '1', false)));
    assert!(committed(settle("b", '1', true)));
    assert_eq!(pending(), [("c".into(), '1')]);
    assert!(committed(queue(vec![action("a", '1'), action("b", '1')])));
    assert_eq!(pending(), [("c".into(), '1')]);
    assert!(committed(queue(vec![action("a", '2')])));
    assert_eq!(pending(), [("a".into(), '2'), ("c".into(), '1')]);
    assert!(!committed(settle("a", '1', false)));
    assert!(committed(settle("a", '2', false)));
    assert!(!committed(settle("a", '2', false)));
    for invalid in [
        vec![action("", '1')],
        vec![action(&"x".repeat(201), '1')],
        vec![action("a\nb", '1')],
        vec![action("d", '1'), action("d", '2')],
        vec![queued(&record(82).id, "d", '1')],
    ] {
        assert!(!committed(queue(invalid)));
    }
    assert_eq!(pending(), [("c".into(), '1')]);
    assert!(committed(ActionChange::ResendRefused(Some(record(82).id))));
    assert_eq!(pending(), [("c".into(), '1')]);
    assert!(committed(ActionChange::ResendRefused(Some(
        recording.clone()
    ))));
    assert_eq!(pending(), [("b".into(), '1'), ("c".into(), '1')]);
    assert!(committed(settle("b", '1', true)));
    assert!(committed(ActionChange::ResendRefused(None)));
    assert_eq!(pending(), [("b".into(), '1'), ("c".into(), '1')]);
}

pub(crate) fn a_dropped_action_awaits_withdrawal_only_when_attempted_and_named(
    ledger: &(impl RecordingLedger + RunHistory + ActionLedger),
) {
    let run = seed(ledger);
    let recording = record(81).id;
    let action = |id: &str, digest| queued(&recording, id, digest);
    let committed = |change| commits(ledger, run, &recording, change);
    let queue = |actions, dropped: &[&str]| ActionChange::Queue {
        recording: recording.clone(),
        actions,
        dropped: dropped.iter().map(|id| (*id).to_owned()).collect(),
    };
    let withdrawing = || ids(ledger.withdrawing_actions().unwrap());
    let all = || vec![action("a", '1'), action("b", '1'), action("c", '1')];
    assert!(committed(queue(all(), &[])));
    assert!(committed(ActionChange::Attempt(action("a", '1'))));
    assert!(committed(ActionChange::Attempt(action("b", '1'))));
    assert!(!committed(ActionChange::Attempt(action("b", '2'))));
    assert!(committed(queue(vec![action("a", '1')], &["c"])));
    assert_eq!(withdrawing(), []);
    assert_eq!(
        ids(ledger.pending_actions().unwrap()),
        [("a".into(), '1'), ("b".into(), '1')]
    );
    assert!(!committed(queue(vec![action("a", '1')], &["a"])));
    assert!(committed(queue(vec![], &["a", "b"])));
    assert_eq!(withdrawing(), [("a".into(), '1'), ("b".into(), '1')]);
    assert_eq!(ledger.pending_actions().unwrap(), []);
    assert!(!committed(ActionChange::Attempt(action("a", '1'))));
    assert!(!committed(ActionChange::Withdrawn(action("a", '2'))));
    assert!(committed(ActionChange::Withdrawn(action("a", '1'))));
    assert!(!committed(ActionChange::Withdrawn(action("a", '1'))));
    assert!(committed(queue(vec![action("b", '1')], &[])));
    assert_eq!(withdrawing(), []);
    assert_eq!(ids(ledger.pending_actions().unwrap()), [("b".into(), '1')]);
    assert!(committed(queue(vec![], &["b"])));
    assert!(committed(queue(vec![], &[])));
    assert_eq!(withdrawing(), [("b".into(), '1')]);
}
