use super::*;
use vpt_application::ports::{StageState, SynthesisStateChange};

fn change(
    ledger: &(impl RecordingLedger + RunHistory),
    from: StageState,
    to: StageState,
) -> (vpt_domain::history::RunId, SynthesisStateChange) {
    let mut row = record(13);
    row.stages.synthesis = from;
    ledger
        .commit(&LedgerCommit {
            recordings: vec![row.clone()],
            ..Default::default()
        })
        .unwrap();
    let run = committed(ledger.begin_run(RunKind::Pipeline, UtcInstant { secs: 10 })).run;
    (
        run,
        SynthesisStateChange {
            recording: row.id,
            expected: from,
            next: to,
            at: UtcInstant { secs: 11 },
        },
    )
}

pub(crate) fn synthesis_eligibility_commits_state_and_history_without_completed_work(
    ledger: &(impl RecordingLedger + RunHistory),
) {
    let (run, mut change) = change(ledger, StageState::Pending, StageState::Disabled);
    let mut original = ledger.by_id(&change.recording).unwrap().unwrap();
    for (from, to) in [
        (StageState::Pending, StageState::Disabled),
        (StageState::Disabled, StageState::Pending),
    ] {
        change.expected = from;
        change.next = to;
        let facts = [change.fact()];
        let receipt = committed(ledger.commit_step(
            run,
            &WorkDelta {
                synthesis_state: Some(change.clone()),
                ..Default::default()
            },
            &facts,
        ));
        assert!(receipt.completed.is_empty());
        assert_eq!(receipt.records.len(), 1);
        assert_eq!(receipt.records[0].fact, facts[0].1);
        original.stages.synthesis = to;
        assert_eq!(
            ledger.by_id(&change.recording).unwrap(),
            Some(original.clone())
        );
        assert_eq!(
            ledger.pending_exports().unwrap().last().unwrap().fact,
            facts[0].1
        );
    }
}

pub(crate) fn synthesis_eligibility_refuses_stale_or_fabricated_changes(
    ledger: &(impl RecordingLedger + RunHistory),
) {
    let (run, initial) = change(ledger, StageState::Pending, StageState::Disabled);
    let before = ledger.by_id(&initial.recording).unwrap();
    let history = ledger.pending_exports().unwrap();
    for variant in 0..10 {
        let mut change = initial.clone();
        match variant {
            0 => change.expected = StageState::Failed,
            1 => change.next = StageState::Succeeded,
            2 => change.next = StageState::Expired,
            3 => change.next = StageState::Pending,
            4 => change.recording = record(14).id,
            _ => {}
        }
        let mut facts = vec![change.fact()];
        let mut delta = WorkDelta {
            synthesis_state: Some(change),
            ..Default::default()
        };
        match variant {
            5 => facts = vec![step(11, WorkOutcome::Completed)],
            6 => delta.synthesis_state = None,
            7 => delta
                .source_updates
                .push((initial.recording.clone(), "/changed.m4a".into())),
            8 => facts.push(initial.fact()),
            9 => facts[0].0.secs += 1,
            _ => {}
        }
        assert!(matches!(
            ledger.commit_step(run, &delta, &facts),
            HistoryWrite::Unconfirmed(_)
        ));
        assert_eq!(ledger.by_id(&initial.recording).unwrap(), before);
        assert_eq!(ledger.pending_exports().unwrap(), history);
    }
}

pub(crate) fn expired_synthesis_cannot_be_reenabled_by_configuration(
    ledger: &(impl RecordingLedger + RunHistory),
) {
    let (run, mut change) = change(ledger, StageState::Expired, StageState::Pending);
    let before = ledger.pending_exports().unwrap();
    for next in [StageState::Pending, StageState::Disabled] {
        change.next = next;
        assert!(matches!(
            ledger.commit_step(
                run,
                &WorkDelta {
                    synthesis_state: Some(change.clone()),
                    ..Default::default()
                },
                &[change.fact()]
            ),
            HistoryWrite::Unconfirmed(_)
        ));
        assert_eq!(
            ledger
                .by_id(&change.recording)
                .unwrap()
                .unwrap()
                .stages
                .synthesis,
            StageState::Expired
        );
        assert_eq!(ledger.pending_exports().unwrap(), before);
    }
}

pub(crate) fn synthesis_eligibility_rolls_back_when_later_history_is_invalid(
    ledger: &(impl RecordingLedger + RunHistory),
) {
    let (run, change) = change(ledger, StageState::Pending, StageState::Disabled);
    let before = ledger.pending_exports().unwrap();
    let result = ledger.commit_step(
        run,
        &WorkDelta {
            synthesis_state: Some(change.clone()),
            ..Default::default()
        },
        &[
            change.fact(),
            (
                change.at,
                HistoryFact::HookFailed {
                    event: vpt_domain::history::EventId::new(run.namespace(), 999),
                    failure: vpt_domain::run::HookFailureClass::Spawn,
                },
            ),
        ],
    );
    assert!(matches!(result, HistoryWrite::Unconfirmed(_)));
    assert_eq!(
        ledger
            .by_id(&change.recording)
            .unwrap()
            .unwrap()
            .stages
            .synthesis,
        StageState::Pending
    );
    assert_eq!(ledger.pending_exports().unwrap(), before);
}
