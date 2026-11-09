use super::*;
use vpt_domain::{
    grounding::SourceRange,
    review::{FlagContent, Resolution},
    run::HistoryFact,
};

pub(crate) fn replacement_is_artifact_scoped_and_preserves_only_unchanged_claim_resolutions(
    ledger: &(impl proposals::Ledger + ReviewLedger),
) {
    let run = fixture::seed(ledger);
    analysis(ledger, run);
    let mut selected = change(ledger);
    committed(save(ledger, run, selected.clone()));
    let mut other = selected.clone();
    other.artifact = NoteStage::Analysis;
    other.input_digest = ledger
        .note(&other.recording, other.artifact)
        .unwrap()
        .unwrap()
        .content_digest;
    other.findings[0].artifact = NoteStage::Analysis;
    committed(save(ledger, run, other));
    let identifier = crate::verification_flag_id(&selected.recording, &selected.findings[0]);
    let at = UtcInstant { secs: 81 };
    for resolution in [
        Resolution::Confirmed { at },
        Resolution::Corrected {
            text: "private corrected claim".into(),
            at,
        },
        Resolution::Dismissed { at },
    ] {
        let expected = findings(ledger)
            .into_iter()
            .find(|flag| flag.identifier == identifier)
            .unwrap()
            .resolution;
        let delta = WorkDelta {
            resolution: Some(FlagResolutionChange {
                recording: selected.recording.clone(),
                flag: identifier,
                expected,
                resolution: resolution.clone(),
            }),
            ..Default::default()
        };
        committed(ledger.commit_step(
            run,
            &delta,
            &[
                fixture::fact(81, WorkOutcome::Completed),
                (
                    at,
                    HistoryFact::FlagResolved {
                        recording: selected.recording.clone(),
                        flag: identifier,
                        resolution: resolution.kind().unwrap(),
                    },
                ),
            ],
        ));
        selected.findings[0].finding = Finding::BadReference;
        committed(save(ledger, run, selected.clone()));
        assert_eq!(
            findings(ledger)
                .into_iter()
                .find(|flag| flag.identifier == identifier)
                .unwrap()
                .resolution,
            resolution
        );
    }
    assert_eq!(findings(ledger).len(), 2);
    selected.findings[0].record_text = "edited private claim".into();
    selected.findings[0].claim_digest = notes::hash(selected.findings[0].record_text.as_bytes());
    committed(save(ledger, run, selected.clone()));
    assert!(
        findings(ledger)
            .iter()
            .all(|flag| flag.identifier != identifier && flag.resolution == Resolution::Open)
    );
    selected.findings.clear();
    committed(save(ledger, run, selected));
    let remaining = findings(ledger);
    assert_eq!(remaining.len(), 1);
    assert!(
        matches!(&remaining[0].content, FlagContent::Verification(finding) if finding.artifact == NoteStage::Analysis)
    );
}

pub(crate) fn transcription_preserves_owned_findings_and_keeps_them_out_of_classification(
    ledger: &(impl proposals::Ledger + ReviewLedger),
) {
    let run = fixture::seed(ledger);
    let change = change(ledger);
    committed(save(ledger, run, change.clone()));
    let before = findings(ledger);
    let mut transcript = ledger
        .accepted_transcript(&change.recording)
        .unwrap()
        .unwrap();
    transcript.accepted_at = UtcInstant { secs: 90 };
    committed(ledger.commit_step(
        run,
        &WorkDelta {
            transcription: Some(TranscriptionChange::Accept(Box::new(transcript.clone()))),
            ..Default::default()
        },
        &[fixture::fact(90, WorkOutcome::Completed)],
    ));
    assert_eq!(findings(ledger), before);
    assert_eq!(
        ledger.accepted_transcript(&change.recording).unwrap(),
        Some(transcript)
    );
    assert!(
        ledger
            .transcript_flags(&change.recording)
            .unwrap()
            .iter()
            .all(|flag| !matches!(flag.content, FlagContent::Verification(_)))
    );
}

pub(crate) fn every_finding_shape_round_trips_without_private_debug(
    ledger: &(impl proposals::Ledger + ReviewLedger),
) {
    let run = fixture::seed(ledger);
    let mut change = change(ledger);
    let original = change.findings[0].clone();
    change.findings = [
        Finding::Unsourced,
        Finding::BadReference,
        Finding::Unsupported("private token".into()),
        Finding::BuiltOnFlaggedText,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, finding)| VerificationFlag {
        claim_index: index as u32,
        sources: if index < 2 {
            vec![]
        } else {
            vec![SourceRange {
                start_secs: 0,
                end_secs: 1,
            }]
        },
        finding,
        ..original.clone()
    })
    .collect();
    committed(save(ledger, run, change.clone()));
    let mut saved = findings(ledger)
        .into_iter()
        .map(|flag| match flag.content {
            FlagContent::Verification(value) => value,
            _ => unreachable!(),
        })
        .collect::<Vec<_>>();
    saved.sort_by_key(|flag| flag.claim_index);
    assert_eq!(saved, change.findings);
    assert!(!format!("{change:?} {saved:?}").contains("private"));
    let report = vpt_application::review::review(ledger, &change.recording).unwrap();
    assert_eq!(report.findings.len(), 4);
    assert_eq!(report.flags.len(), 1);
    let before = ledger.accepted_transcript(&change.recording).unwrap();
    let basis = ledger.grounding_basis(&change.recording).unwrap();
    assert_eq!(basis, Some(change.basis));
    assert_eq!(before.unwrap().uncertainty.lexical.len(), 1);
}

pub(crate) fn annotation_only_content_update_commits_with_findings(
    ledger: &(impl proposals::Ledger + ReviewLedger),
) {
    let run = fixture::seed(ledger);
    let mut change = change(ledger);
    let old = ledger
        .note(&change.recording, change.artifact)
        .unwrap()
        .unwrap();
    let document = crate::note_document::Document::parse(&old.text).unwrap();
    let content = document.content().unwrap();
    let parsed = crate::markdown_claims::extract_content(content).unwrap();
    let claim = &parsed.claims()[0];
    change.findings[0].record_text = claim.identity_text.clone();
    change.findings[0].claim_digest = notes::hash(claim.identity_text.as_bytes());
    let updated = parsed
        .annotate(&vec![Some(Finding::Unsourced); parsed.claims().len()])
        .unwrap();
    let mut next = old.clone();
    fixture::replace_content(&mut next, &updated);
    next.updated_at = change.at;
    let delta = WorkDelta {
        verification: Some(change.clone()),
        notes: vec![NoteChange {
            artifact: next.clone(),
            expected_saved: Some(old.digest),
            relocated_from: None,
        }],
        commit: LedgerCommit {
            publications: vec![DirtyPublication {
                target: next.target.clone(),
                expected_previous: Some(old.digest),
                intended: next.digest,
                recorded_at: next.updated_at,
            }],
            ..Default::default()
        },
        ..Default::default()
    };
    committed(ledger.commit_step(run, &delta, &[fixture::fact(80, WorkOutcome::Completed)]));
    assert_eq!(
        ledger.note(&next.recording, next.stage).unwrap(),
        Some(next.clone())
    );
    assert_eq!(findings(ledger).len(), 1);
    assert_eq!(
        ledger.pending_publications().unwrap(),
        delta.commit.publications
    );
    fixture::finish(ledger, run, &next.target);
    change.input_digest = next.content_digest;
    committed(save(ledger, run, change));
    assert_eq!(findings(ledger).len(), 1);
}
