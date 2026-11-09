use super::*;
use vpt_domain::{
    history::EventId,
    run::{EffectKind, EffectOutcome, EffectReference, HookFailureClass},
};

pub(crate) fn note_stage_completion_commits_with_publication_history(
    ledger: &(impl RecordingLedger + RunHistory + NoteLedger + PublicationJournal),
) {
    let run = seed(ledger);
    committed(ledger.commit_step(run, &delta(), &facts()));
    assert_eq!(
        ledger
            .by_id(&artifact().recording)
            .unwrap()
            .unwrap()
            .stages
            .note,
        StageState::Pending
    );
    let at = UtcInstant { secs: 60 };
    let intent = committed(ledger.commit_step(
        run,
        &WorkDelta::default(),
        &[(
            at,
            HistoryFact::EffectIntent {
                kind: EffectKind::Publication,
                reference: EffectReference::PublicationTarget(artifact().target),
            },
        )],
    ))
    .records[0]
        .id;
    let clear = WorkDelta {
        publication_clears: vec![artifact().target],
        ..Default::default()
    };
    let observed = (
        at,
        HistoryFact::EffectObserved {
            intent,
            outcome: EffectOutcome::Completed,
        },
    );
    let bad = (
        at,
        HistoryFact::HookFailed {
            event: EventId::new(run.namespace(), 999),
            failure: HookFailureClass::Spawn,
        },
    );
    assert!(matches!(
        ledger.commit_step(run, &clear, &[observed.clone(), bad]),
        HistoryWrite::Unconfirmed(_)
    ));
    assert_eq!(
        ledger
            .by_id(&artifact().recording)
            .unwrap()
            .unwrap()
            .stages
            .note,
        StageState::Pending
    );
    assert_eq!(
        ledger.pending_publications().unwrap(),
        delta().commit.publications
    );
    assert_eq!(ledger.pending_exports().unwrap().len(), 3);
    committed(ledger.commit_step(run, &clear, &[observed]));
    assert!(ledger.pending_publications().unwrap().is_empty());
    let stages = ledger.by_id(&artifact().recording).unwrap().unwrap().stages;
    assert_eq!(stages.note, StageState::Succeeded);
    assert_eq!(stages.synthesis, StageState::Pending);
    assert_eq!(ledger.pending_exports().unwrap().len(), 4);
}
