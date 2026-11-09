use super::*;

pub(crate) fn invalid_known_term_intents_leave_state_unchanged(
    ledger: &(impl RunHistory + VocabularyLedger + PublicationJournal),
) {
    let run = committed(ledger.begin_run(RunKind::Publish, UtcInstant { secs: 10 })).run;
    for variant in 0..10 {
        let mut delta = delta();
        let mut facts = facts();
        let value = delta.vocabulary.as_mut().unwrap();
        match variant {
            0 => {
                value.bytes = b"private\0".to_vec();
                value.entry.intended = Sha256Digest(Sha256::digest(&value.bytes).into());
            }
            1 => value.entry.intended = Sha256Digest([0; 32]),
            2 => value.entry.target = "relative/terms".into(),
            3 => value.entry.expected_previous = Some(value.entry.intended),
            4 => facts[0].0.secs += 1,
            5 => facts.clear(),
            6 => facts.push(facts[0].clone()),
            7 => delta.publication_clears.push(value.entry.target.clone()),
            8 => delta.commit.publications.push(value.entry.clone()),
            _ => {
                facts[0].1 = HistoryFact::EffectIntent {
                    kind: EffectKind::Publication,
                    reference: EffectReference::PublicationTarget("/other/terms".into()),
                }
            }
        }
        // A valid unrelated fact keeps the empty-intent case meaningful to the ledger.
        if facts.is_empty() {
            facts.push((
                UtcInstant { secs: 20 },
                HistoryFact::RunFinished(RunOutcome::Failed),
            ));
        }
        let result = ledger.commit_step(run, &delta, &facts);
        assert!(
            matches!(result, HistoryWrite::Unconfirmed(_)),
            "variant {variant}: {result:?}"
        );
        assert_eq!(
            ledger
                .vocabulary_publication(&publication().entry.target)
                .unwrap(),
            None
        );
        assert!(ledger.pending_publications().unwrap().is_empty());
        assert_eq!(ledger.pending_exports().unwrap().len(), 1);
    }
}

pub(crate) fn known_term_publications_reserve_their_target_until_cleared(
    ledger: &(impl RunHistory + VocabularyLedger + PublicationJournal + RecordingLedger),
) {
    let run = committed(ledger.begin_run(RunKind::Publish, UtcInstant { secs: 10 })).run;
    let mut foreign = publication().entry;
    foreign.intended = Sha256Digest([1; 32]);
    ledger
        .commit(&LedgerCommit {
            publications: vec![foreign.clone()],
            ..Default::default()
        })
        .unwrap();
    assert!(matches!(
        ledger.commit_step(run, &delta(), &facts()),
        HistoryWrite::Unconfirmed(_)
    ));
    assert_eq!(
        ledger.pending_publications().unwrap(),
        vec![foreign.clone()]
    );
    assert!(
        ledger
            .vocabulary_publication(&foreign.target)
            .unwrap()
            .is_none()
    );
    ledger.clear_publication(&foreign.target).unwrap();
    committed(ledger.commit_step(run, &delta(), &facts()));
    assert!(
        ledger
            .commit(&LedgerCommit {
                publications: vec![foreign.clone()],
                ..Default::default()
            })
            .is_err()
    );
    assert_eq!(
        ledger.vocabulary_publication(&foreign.target).unwrap(),
        Some(publication())
    );
    assert_eq!(
        ledger.pending_publications().unwrap(),
        vec![publication().entry]
    );
    ledger.clear_publication(&foreign.target).unwrap();
    assert!(
        ledger
            .vocabulary_publication(&foreign.target)
            .unwrap()
            .is_none()
    );
    committed(ledger.commit_step(run, &delta(), &facts()));
    assert_eq!(
        ledger.vocabulary_publication(&foreign.target).unwrap(),
        Some(publication())
    );
}
