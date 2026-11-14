use super::{record, seed};
use vpt_application::ports::*;
use vpt_domain::{
    run::{HistoryFact, WorkOutcome},
    time::UtcInstant,
};

pub(crate) fn an_action_is_pending_until_settled_and_again_only_when_its_digest_changes(
    ledger: &(impl RecordingLedger + RunHistory + ActionLedger),
) {
    let run = seed(ledger);
    let recording = record(81).id;
    let action = |id: &str, digest: char| QueuedAction {
        recording: recording.clone(),
        id: id.into(),
        digest: digest.to_string().repeat(64),
    };
    let step = |change: ActionChange| {
        ledger.commit_step(
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
        )
    };
    let committed = |change| matches!(step(change), HistoryWrite::Committed(_));
    let settle = |id, digest, refused| ActionChange::Settle {
        action: action(id, digest),
        refused,
    };
    let pending = || {
        ledger
            .pending_actions()
            .unwrap()
            .into_iter()
            .map(|queued| (queued.id, queued.digest.chars().next().unwrap()))
            .collect::<Vec<_>>()
    };
    assert!(committed(ActionChange::Queue(vec![
        action("a", '1'),
        action("b", '1'),
        action("c", '1'),
    ])));
    assert!(committed(settle("a", '1', false)));
    assert!(committed(settle("b", '1', true)));
    assert_eq!(pending(), [("c".into(), '1')]);
    assert!(committed(ActionChange::Queue(vec![
        action("a", '1'),
        action("b", '1')
    ])));
    assert_eq!(pending(), [("c".into(), '1')]);
    assert!(committed(ActionChange::Queue(vec![action("a", '2')])));
    assert_eq!(pending(), [("a".into(), '2'), ("c".into(), '1')]);
    assert!(!committed(settle("a", '1', false)));
    assert!(committed(settle("a", '2', false)));
    assert!(!committed(settle("a", '2', false)));
    for invalid in [
        vec![action("", '1')],
        vec![action(&"x".repeat(201), '1')],
        vec![action("a\nb", '1')],
        vec![action("d", '1'), action("d", '2')],
    ] {
        assert!(!committed(ActionChange::Queue(invalid)));
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
