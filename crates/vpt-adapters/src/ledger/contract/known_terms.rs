use super::engine_outputs::committed;
use sha2::{Digest, Sha256};
use vpt_application::ports::*;
use vpt_domain::{digest::Sha256Digest, history::EventId, run::*, time::UtcInstant};
mod refusal;
mod tags;
pub(crate) use refusal::*;
pub(crate) use tags::*;

pub(crate) fn publication() -> VocabularyPublication {
    let bytes = b"Private confirmation canary\n".to_vec();
    VocabularyPublication {
        kind: vpt_application::ports::VocabularyKind::Terms,
        entry: DirtyPublication {
            target: "/config/known-terms.txt".into(),
            expected_previous: None,
            intended: Sha256Digest(Sha256::digest(&bytes).into()),
            recorded_at: UtcInstant { secs: 20 },
        },
        bytes,
    }
}
pub(crate) fn delta() -> WorkDelta {
    WorkDelta {
        vocabulary: Some(publication()),
        ..Default::default()
    }
}
pub(crate) fn facts() -> Vec<(UtcInstant, HistoryFact)> {
    vec![(
        publication().entry.recorded_at,
        HistoryFact::EffectIntent {
            kind: EffectKind::Publication,
            reference: EffectReference::PublicationTarget(publication().entry.target),
        },
    )]
}

pub(crate) fn known_terms_bytes_intent_and_history_commit_together(
    ledger: &(impl RunHistory + VocabularyLedger + PublicationJournal),
) {
    let run = committed(ledger.begin_run(RunKind::Publish, UtcInstant { secs: 10 })).run;
    let receipt = committed(ledger.commit_step(run, &delta(), &facts()));
    assert_eq!(
        ledger
            .vocabulary_publication(&publication().entry.target)
            .unwrap(),
        Some(publication())
    );
    assert_eq!(
        ledger.pending_publications().unwrap(),
        vec![publication().entry]
    );
    assert!(receipt.completed.is_empty());
    assert!(
        !format!(
            "{:?} {:?} {:?}",
            delta(),
            receipt,
            ledger.pending_exports().unwrap()
        )
        .contains("Private confirmation canary")
    );
    assert!(matches!(
        ledger.commit_step(run, &delta(), &facts()),
        HistoryWrite::Unconfirmed(_)
    ));
    assert_eq!(
        ledger
            .vocabulary_publication(&publication().entry.target)
            .unwrap(),
        Some(publication())
    );
}

pub(crate) fn failed_known_terms_commit_leaves_no_bytes_intent_or_history(
    ledger: &(impl RunHistory + VocabularyLedger + PublicationJournal),
) {
    let run = committed(ledger.begin_run(RunKind::Publish, UtcInstant { secs: 10 })).run;
    let mut facts = facts();
    facts.push((
        UtcInstant { secs: 20 },
        HistoryFact::HookFailed {
            event: EventId::new(run.namespace(), 999),
            failure: HookFailureClass::Spawn,
        },
    ));
    assert!(matches!(
        ledger.commit_step(run, &delta(), &facts),
        HistoryWrite::Unconfirmed(_)
    ));
    assert!(
        ledger
            .vocabulary_publication(&publication().entry.target)
            .unwrap()
            .is_none()
    );
    assert!(ledger.pending_publications().unwrap().is_empty());
    assert_eq!(ledger.pending_exports().unwrap().len(), 1);
}

pub(crate) fn known_terms_completion_removes_saved_bytes_with_the_publication(
    ledger: &(impl RunHistory + VocabularyLedger + PublicationJournal),
) {
    let run = committed(ledger.begin_run(RunKind::Publish, UtcInstant { secs: 10 })).run;
    let saved = committed(ledger.commit_step(run, &delta(), &facts()));
    let complete = WorkDelta {
        publication_clears: vec![publication().entry.target.clone()],
        ..Default::default()
    };
    let at = UtcInstant { secs: 30 };
    let fact = HistoryFact::EffectObserved {
        intent: saved.records[0].id,
        outcome: EffectOutcome::Completed,
    };
    let failed = [
        (at, fact.clone()),
        (
            at,
            HistoryFact::HookFailed {
                event: EventId::new(run.namespace(), 999),
                failure: HookFailureClass::Spawn,
            },
        ),
    ];
    assert!(matches!(
        ledger.commit_step(run, &complete, &failed),
        HistoryWrite::Unconfirmed(_)
    ));
    assert_eq!(
        ledger
            .vocabulary_publication(&publication().entry.target)
            .unwrap(),
        Some(publication())
    );
    committed(ledger.commit_step(run, &complete, &[(at, fact)]));
    assert!(
        ledger
            .vocabulary_publication(&publication().entry.target)
            .unwrap()
            .is_none()
    );
    assert!(ledger.pending_publications().unwrap().is_empty());
}

macro_rules! cases {
    ($case:ident) => {
        $case!(tag_and_term_publications_keep_their_kinds_until_acknowledged);
        $case!(invalid_tag_vocabulary_cannot_be_saved_as_a_publication);
        $case!(known_terms_bytes_intent_and_history_commit_together);
        $case!(failed_known_terms_commit_leaves_no_bytes_intent_or_history);
        $case!(known_terms_completion_removes_saved_bytes_with_the_publication);
        $case!(invalid_known_term_intents_leave_state_unchanged);
        $case!(known_term_publications_reserve_their_target_until_cleared);
    };
}
pub(crate) use cases;
