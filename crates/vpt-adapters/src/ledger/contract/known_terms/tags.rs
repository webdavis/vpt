use super::*;

pub(crate) fn tag_publication() -> VocabularyPublication {
    let bytes = b"Private_Tag\n".to_vec();
    let mut value = publication();
    value.kind = VocabularyKind::Tags;
    value.entry.target = "/config/known-tags.txt".into();
    value.entry.intended = Sha256Digest(Sha256::digest(&bytes).into());
    value.bytes = bytes;
    value
}

pub(crate) fn tag_facts() -> Vec<(UtcInstant, HistoryFact)> {
    let value = tag_publication();
    vec![(
        value.entry.recorded_at,
        HistoryFact::EffectIntent {
            kind: EffectKind::Publication,
            reference: EffectReference::PublicationTarget(value.entry.target),
        },
    )]
}

pub(crate) fn tag_and_term_publications_keep_their_kinds_until_acknowledged(
    ledger: &(impl RunHistory + VocabularyLedger + PublicationJournal),
) {
    let run = committed(ledger.begin_run(RunKind::Publish, UtcInstant { secs: 10 })).run;
    committed(ledger.commit_step(run, &delta(), &facts()));
    let tags = tag_publication();
    let delta = WorkDelta {
        vocabulary: Some(tags.clone()),
        ..Default::default()
    };
    let mut failed = tag_facts();
    failed.push((
        UtcInstant { secs: 20 },
        HistoryFact::HookFailed {
            event: EventId::new(run.namespace(), 999),
            failure: HookFailureClass::Spawn,
        },
    ));
    assert!(matches!(
        ledger.commit_step(run, &delta, &failed),
        HistoryWrite::Unconfirmed(_)
    ));
    assert_eq!(
        ledger.vocabulary_publication(&tags.entry.target).unwrap(),
        None
    );
    let receipt = committed(ledger.commit_step(run, &delta, &tag_facts()));
    assert_eq!(
        ledger.vocabulary_publication(&tags.entry.target).unwrap(),
        Some(tags.clone())
    );
    assert_eq!(
        ledger
            .vocabulary_publication(&publication().entry.target)
            .unwrap(),
        Some(publication())
    );
    assert_eq!(ledger.pending_publications().unwrap().len(), 2);
    committed(ledger.commit_step(
        run,
        &WorkDelta {
            publication_clears: vec![tags.entry.target.clone()],
            ..Default::default()
        },
        &[(
            UtcInstant { secs: 30 },
            HistoryFact::EffectObserved {
                intent: receipt.records[0].id,
                outcome: EffectOutcome::Completed,
            },
        )],
    ));
    assert_eq!(
        ledger.vocabulary_publication(&tags.entry.target).unwrap(),
        None
    );
    assert_eq!(
        ledger
            .vocabulary_publication(&publication().entry.target)
            .unwrap(),
        Some(publication())
    );
}

pub(crate) fn invalid_tag_vocabulary_cannot_be_saved_as_a_publication(
    ledger: &(impl RunHistory + VocabularyLedger + PublicationJournal),
) {
    let run = committed(ledger.begin_run(RunKind::Publish, UtcInstant { secs: 10 })).run;
    for bytes in [b"#private\n".as_slice(), b"1234\n", b"two words\n"] {
        let mut tags = tag_publication();
        tags.bytes = bytes.to_vec();
        tags.entry.intended = Sha256Digest(Sha256::digest(bytes).into());
        let delta = WorkDelta {
            vocabulary: Some(tags.clone()),
            ..Default::default()
        };
        assert!(matches!(
            ledger.commit_step(run, &delta, &tag_facts()),
            HistoryWrite::Unconfirmed(_)
        ));
        assert_eq!(
            ledger.vocabulary_publication(&tags.entry.target).unwrap(),
            None
        );
        assert!(ledger.pending_publications().unwrap().is_empty());
    }
}
