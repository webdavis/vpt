use super::*;
use vpt_domain::{
    review::{FlagContent, Resolution},
    run::HistoryFact,
};

fn resolve(
    ledger: &impl RunHistory,
    run: vpt_domain::history::RunId,
    flag: vpt_domain::review::ReviewFlag,
) {
    let at = UtcInstant { secs: 81 };
    committed(ledger.commit_step(
        run,
        &WorkDelta {
            resolution: Some(FlagResolutionChange {
                recording: record(81).id,
                flag: flag.identifier,
                expected: flag.resolution,
                resolution: Resolution::Confirmed { at },
            }),
            ..Default::default()
        },
        &[
            fixture::fact(81, WorkOutcome::Completed),
            (
                at,
                HistoryFact::FlagResolved {
                    recording: record(81).id,
                    flag: flag.identifier,
                    resolution: vpt_domain::review::ResolutionKind::Confirmed,
                },
            ),
        ],
    ));
}

pub(crate) fn lexical_changes_hide_stale_findings_but_rechecking_preserves_resolutions(
    ledger: &(impl proposals::Ledger + ReviewLedger),
) {
    let run = fixture::seed(ledger);
    analysis(ledger, run);
    let mut change = change(ledger);
    committed(save(ledger, run, change.clone()));
    let selected = findings(ledger).into_iter().find(|flag| matches!(&flag.content, FlagContent::Verification(v) if v.artifact == NoteStage::Transcript)).unwrap();
    resolve(ledger, run, selected.clone());
    let lexical = ledger
        .transcript_flags(&change.recording)
        .unwrap()
        .into_iter()
        .find(|flag| matches!(flag.content, FlagContent::Lexical(_)))
        .unwrap();
    resolve(ledger, run, lexical);
    assert!(
        findings(ledger).is_empty(),
        "stale verification must leave the active review queue"
    );
    change.basis = ledger.grounding_basis(&change.recording).unwrap().unwrap();
    committed(save(ledger, run, change));
    let current = findings(ledger);
    assert_eq!(
        current.len(),
        1,
        "rechecking one artifact must not revive another"
    );
    assert_eq!(current[0].identifier, selected.identifier);
    assert!(matches!(
        current[0].resolution,
        Resolution::Confirmed { .. }
    ));
}

pub(crate) fn transcript_changes_hide_stale_findings_from_both_snapshot_readers(
    ledger: &(impl proposals::Ledger + ReviewLedger),
) {
    let run = fixture::seed(ledger);
    let change = change(ledger);
    committed(save(ledger, run, change.clone()));
    let mut accepted = ledger
        .accepted_transcript(&change.recording)
        .unwrap()
        .unwrap();
    accepted.transcript.segments[0].words[0].end += 0.1;
    accepted.accepted_at = UtcInstant { secs: 90 };
    committed(ledger.commit_step(
        run,
        &WorkDelta {
            transcription: Some(TranscriptionChange::Accept(Box::new(accepted))),
            ..Default::default()
        },
        &[fixture::fact(90, WorkOutcome::Completed)],
    ));
    assert!(findings(ledger).is_empty());
    assert!(
        ledger
            .note_inputs(&change.recording)
            .unwrap()
            .unwrap()
            .review
            .flags
            .iter()
            .all(|flag| !matches!(flag.content, FlagContent::Verification(_)))
    );
}

pub(crate) fn stale_verification_cannot_be_resolved_until_rechecked(
    ledger: &(impl proposals::Ledger + ReviewLedger),
) {
    let run = fixture::seed(ledger);
    committed(save(ledger, run, change(ledger)));
    let finding = findings(ledger).remove(0);
    let lexical = ledger.transcript_flags(&record(81).id).unwrap().remove(0);
    resolve(ledger, run, lexical);
    let history = ledger.pending_exports().unwrap();
    let at = UtcInstant { secs: 82 };
    let result = ledger.commit_step(
        run,
        &WorkDelta {
            resolution: Some(FlagResolutionChange {
                recording: record(81).id,
                flag: finding.identifier,
                expected: finding.resolution,
                resolution: Resolution::Confirmed { at },
            }),
            ..Default::default()
        },
        &[
            fixture::fact(82, WorkOutcome::Completed),
            (
                at,
                HistoryFact::FlagResolved {
                    recording: record(81).id,
                    flag: finding.identifier,
                    resolution: vpt_domain::review::ResolutionKind::Confirmed,
                },
            ),
        ],
    );
    assert!(matches!(result, HistoryWrite::Unconfirmed(_)));
    assert_eq!(ledger.pending_exports().unwrap(), history);
}

pub(crate) fn empty_verification_results_still_record_their_basis(
    ledger: &(impl proposals::Ledger + ReviewLedger),
) {
    let run = fixture::seed(ledger);
    let mut change = change(ledger);
    change.findings.clear();
    committed(save(ledger, run, change.clone()));
    assert!(
        ledger
            .review_snapshot(&change.recording)
            .unwrap()
            .unwrap()
            .stale_artifacts
            .is_empty()
    );
    let lexical = ledger
        .transcript_flags(&change.recording)
        .unwrap()
        .remove(0);
    resolve(ledger, run, lexical);
    let review = ledger.review_snapshot(&change.recording).unwrap().unwrap();
    assert_eq!(review.stale_artifacts, vec![NoteStage::Transcript]);
    assert!(review.stale_findings.is_empty());
    change.basis = ledger.grounding_basis(&change.recording).unwrap().unwrap();
    let history = ledger.pending_exports().unwrap();
    let result = ledger.commit_step(
        run,
        &WorkDelta {
            verification: Some(change.clone()),
            ..Default::default()
        },
        &[
            fixture::fact(80, WorkOutcome::Completed),
            (
                UtcInstant { secs: 80 },
                HistoryFact::HookFailed {
                    event: vpt_domain::history::EventId::new(run.namespace(), 999_999),
                    failure: vpt_domain::run::HookFailureClass::Spawn,
                },
            ),
        ],
    );
    assert!(matches!(result, HistoryWrite::Unconfirmed(_)));
    assert_eq!(
        ledger.review_snapshot(&change.recording).unwrap().unwrap(),
        review
    );
    assert_eq!(ledger.pending_exports().unwrap(), history);
    committed(save(ledger, run, change.clone()));
    assert!(
        ledger
            .review_snapshot(&change.recording)
            .unwrap()
            .unwrap()
            .stale_artifacts
            .is_empty()
    );
}
