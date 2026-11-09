use super::*;
use vpt_domain::run::{EffectKind, EffectReference};

fn prepared(ledger: &impl Ledger) -> (WorkDelta, Vec<(UtcInstant, HistoryFact)>) {
    let mut accepted = accepted(ledger);
    let bytes = b"private-tag\n".to_vec();
    let entry = DirtyPublication {
        target: "/vocabulary/known-tags.txt".into(),
        expected_previous: None,
        intended: notes::hash(&bytes),
        recorded_at: accepted.accepted_at,
    };
    accepted.vocabulary_publication = Some(entry.clone());
    let mut delta = delta(accepted);
    let facts = vec![
        fact(70, WorkOutcome::Completed),
        (
            entry.recorded_at,
            HistoryFact::EffectIntent {
                kind: EffectKind::Publication,
                reference: EffectReference::PublicationTarget(entry.target.clone()),
            },
        ),
    ];
    delta.vocabulary = Some(VocabularyPublication {
        kind: VocabularyKind::Tags,
        entry,
        bytes,
    });
    (delta, facts)
}

pub(crate) fn proposal_vocabulary_dependency_survives_note_publication(ledger: &impl Ledger) {
    let run = seed(ledger);
    let (delta, facts) = prepared(ledger);
    committed(ledger.commit_step(run, &delta, &facts));
    let expected = delta.vocabulary.as_ref().unwrap().entry.clone();
    finish(ledger, run, &delta.notes[0].artifact.target);
    assert_eq!(
        ledger.pending_publications().unwrap(),
        vec![expected.clone()]
    );
    for saved in [
        ledger.accepted_proposal(&record(81).id).unwrap(),
        ledger
            .note_inputs(&record(81).id)
            .unwrap()
            .unwrap()
            .proposal,
    ] {
        assert_eq!(
            saved.unwrap().vocabulary_publication,
            Some(expected.clone())
        );
    }
    finish(ledger, run, &expected.target);
    assert!(ledger.pending_publications().unwrap().is_empty());
    assert_eq!(
        ledger
            .accepted_proposal(&record(81).id)
            .unwrap()
            .unwrap()
            .vocabulary_publication,
        Some(expected)
    );
}

pub(crate) fn proposal_vocabulary_dependency_requires_the_exact_same_transaction(
    ledger: &impl Ledger,
) {
    let run = seed(ledger);
    let before = ledger.pending_exports().unwrap();
    for variant in 0..6 {
        let (mut delta, facts) = prepared(ledger);
        let Some(ProposalChange::Accept(accepted)) = &mut delta.proposal else {
            unreachable!()
        };
        match variant {
            0 => accepted.vocabulary_publication = None,
            1 => delta.vocabulary = None,
            2 => {
                accepted.vocabulary_publication.as_mut().unwrap().target =
                    "/other/known-tags.txt".into()
            }
            3 => {
                accepted
                    .vocabulary_publication
                    .as_mut()
                    .unwrap()
                    .recorded_at
                    .secs += 1
            }
            4 => {
                accepted.vocabulary_publication.as_mut().unwrap().intended = super::super::digest(9)
            }
            _ => delta.vocabulary.as_mut().unwrap().kind = VocabularyKind::Terms,
        }
        assert!(
            matches!(
                ledger.commit_step(run, &delta, &facts),
                HistoryWrite::Unconfirmed(_)
            ),
            "variant {variant}"
        );
        assert!(ledger.accepted_proposal(&record(81).id).unwrap().is_none());
        assert!(ledger.pending_publications().unwrap().is_empty());
        assert_eq!(ledger.pending_exports().unwrap(), before);
    }
}

pub(crate) fn proposal_vocabulary_dependency_rolls_back_with_failed_history(ledger: &impl Ledger) {
    let run = seed(ledger);
    let before = ledger.pending_exports().unwrap();
    let (delta, mut facts) = prepared(ledger);
    facts.push((
        UtcInstant { secs: 70 },
        HistoryFact::HookFailed {
            event: vpt_domain::history::EventId::new(run.namespace(), 999_999),
            failure: vpt_domain::run::HookFailureClass::Spawn,
        },
    ));
    assert!(matches!(
        ledger.commit_step(run, &delta, &facts),
        HistoryWrite::Unconfirmed(_)
    ));
    assert!(ledger.accepted_proposal(&record(81).id).unwrap().is_none());
    assert!(ledger.pending_publications().unwrap().is_empty());
    assert_eq!(ledger.pending_exports().unwrap(), before);
}
