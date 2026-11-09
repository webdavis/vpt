use super::*;
use vpt_domain::review::{Resolution, ResolutionKind};

pub(crate) fn pending_source_publication_and_changed_transcription_refuse_acceptance(
    ledger: &impl Ledger,
) {
    let run = seed(ledger);
    let next = delta(accepted(ledger));
    let source = ledger
        .note(&record(81).id, NoteStage::Transcript)
        .unwrap()
        .unwrap();
    ledger
        .commit(&LedgerCommit {
            publications: vec![DirtyPublication {
                target: source.target.clone(),
                expected_previous: Some(source.digest),
                intended: source.digest,
                recorded_at: UtcInstant { secs: 60 },
            }],
            ..Default::default()
        })
        .unwrap();
    assert!(matches!(
        ledger.commit_step(run, &next, &[fact(70, WorkOutcome::Completed)]),
        HistoryWrite::Unconfirmed(_)
    ));
    ledger.clear_publication(&source.target).unwrap();
    let mut changed = ledger.accepted_transcript(&record(81).id).unwrap().unwrap();
    changed.accepted_at.secs = 65;
    changed.transcript.engine.version = "new version".into();
    committed(ledger.commit_step(
        run,
        &WorkDelta {
            transcription: Some(TranscriptionChange::Accept(Box::new(changed))),
            ..Default::default()
        },
        &[fact(65, WorkOutcome::Completed)],
    ));
    assert!(matches!(
        ledger.commit_step(run, &next, &[fact(70, WorkOutcome::Completed)]),
        HistoryWrite::Unconfirmed(_)
    ));
    assert!(ledger.accepted_proposal(&record(81).id).unwrap().is_none());
    assert!(
        ledger
            .note(&record(81).id, NoteStage::Analysis)
            .unwrap()
            .is_none()
    );
}

pub(crate) fn lexical_resolution_changes_basis_and_preserves_prior_proposal(ledger: &impl Ledger) {
    let run = seed(ledger);
    let old = accepted(ledger);
    let next = delta(old.clone());
    committed(ledger.commit_step(run, &next, &[fact(70, WorkOutcome::Completed)]));
    finish(ledger, run, &next.notes[0].artifact.target);
    let flag = ledger.transcript_flags(&old.recording).unwrap().remove(0);
    let at = UtcInstant { secs: 80 };
    let resolution = FlagResolutionChange {
        recording: old.recording.clone(),
        flag: flag.identifier,
        expected: Resolution::Open,
        resolution: Resolution::Confirmed { at },
    };
    let facts = [
        fact(80, WorkOutcome::Completed),
        (
            at,
            HistoryFact::FlagResolved {
                recording: old.recording.clone(),
                flag: flag.identifier,
                resolution: ResolutionKind::Confirmed,
            },
        ),
    ];
    committed(ledger.commit_step(
        run,
        &WorkDelta {
            resolution: Some(resolution),
            ..Default::default()
        },
        &facts,
    ));
    assert_ne!(
        ledger.grounding_basis(&old.recording).unwrap(),
        Some(old.basis)
    );
    assert_eq!(ledger.accepted_proposal(&old.recording).unwrap(), Some(old));
}

pub(crate) fn unknown_recording_has_no_proposal_or_basis_and_cannot_fail_a_stage(
    ledger: &impl Ledger,
) {
    let id = record(81).id;
    assert!(ledger.accepted_proposal(&id).unwrap().is_none());
    assert!(ledger.grounding_basis(&id).unwrap().is_none());
    let run =
        committed(ledger.begin_run(vpt_domain::run::RunKind::Pipeline, UtcInstant { secs: 1 })).run;
    let failed = WorkDelta {
        proposal: Some(ProposalChange::Fail {
            recording: id,
            at: UtcInstant { secs: 70 },
        }),
        ..Default::default()
    };
    assert!(matches!(
        ledger.commit_step(run, &failed, &[fact(70, WorkOutcome::Failed)]),
        HistoryWrite::Unconfirmed(_)
    ));
    assert_eq!(ledger.pending_exports().unwrap().len(), 1);
}

pub(crate) fn pending_source_retention_refuses_acceptance(ledger: &impl Ledger) {
    let run = seed(ledger);
    let next = delta(accepted(ledger));
    let source = ledger
        .note(&record(81).id, NoteStage::Transcript)
        .unwrap()
        .unwrap();
    ledger
        .record_intent(&NewIntent {
            kind: ArtifactKind::Audio,
            owner: (source.recording).into(),
            path: source.target,
            expected: source.digest,
            recorded_at: UtcInstant { secs: 60 },
        })
        .unwrap();
    assert!(matches!(
        ledger.commit_step(run, &next, &[fact(70, WorkOutcome::Completed)]),
        HistoryWrite::Unconfirmed(_)
    ));
    assert!(ledger.accepted_proposal(&record(81).id).unwrap().is_none());
    assert_eq!(ledger.pending_intents().unwrap().len(), 1);
}
