use super::*;
use vpt_domain::review::{FlagContent, Resolution, ReviewFlag};

fn flags(ledger: &impl Ledger) -> Vec<ReviewFlag> {
    let mut flags = ledger
        .note_inputs(&record(81).id)
        .unwrap()
        .unwrap()
        .review
        .flags;
    flags.sort_by_key(|flag| flag.identifier.0);
    flags
}

fn next(ledger: &impl Ledger, old: &NoteArtifact) -> WorkDelta {
    let mut proposal = accepted(ledger);
    proposal.accepted_at.secs = 90;
    let mut delta = delta(proposal);
    delta.notes[0].expected_saved = Some(old.digest);
    delta.notes[0].artifact.first_written = old.first_written;
    delta.commit.publications[0].expected_previous = Some(old.digest);
    delta
}

fn set_content(delta: &mut WorkDelta, content: &str) {
    replace_content(&mut delta.notes[0].artifact, content);
    delta.commit.publications[0].intended = delta.notes[0].artifact.digest;
}

pub(crate) fn analysis_preserves_reviewed_claims_and_other_artifact_findings(ledger: &impl Ledger) {
    let run = seed(ledger);
    let initial = delta(accepted(ledger));
    committed(ledger.commit_step(run, &initial, &[fact(70, WorkOutcome::Completed)]));
    finish(ledger, run, &initial.notes[0].artifact.target);
    let transcript = super::super::verification::change(ledger);
    committed(super::super::verification::save(ledger, run, transcript));
    let reviewed = flags(ledger).into_iter().find(|flag| matches!(&flag.content,
        FlagContent::Verification(value) if value.artifact == NoteStage::Analysis && value.claim_index == 1)).unwrap();
    let at = UtcInstant { secs: 81 };
    let mut old = initial.notes[0].artifact.clone();
    for resolution in [
        Resolution::Confirmed { at },
        Resolution::Corrected {
            text: "operator correction <private>".into(),
            at,
        },
        Resolution::Dismissed { at },
    ] {
        let expected = flags(ledger)
            .into_iter()
            .find(|flag| flag.identifier == reviewed.identifier)
            .unwrap()
            .resolution;
        committed(ledger.commit_step(
            run,
            &WorkDelta {
                resolution: Some(FlagResolutionChange {
                    recording: record(81).id,
                    flag: reviewed.identifier,
                    expected,
                    resolution: resolution.clone(),
                }),
                ..Default::default()
            },
            &[
                fact(81, WorkOutcome::Completed),
                (
                    at,
                    HistoryFact::FlagResolved {
                        recording: record(81).id,
                        flag: reviewed.identifier,
                        resolution: resolution.kind().unwrap(),
                    },
                ),
            ],
        ));
        let before = flags(ledger);
        let mut next = next(ledger, &old);
        // Fresh typed output still contains the finding marker. Acceptance must require
        // the published annotation to respect the saved operator decision.
        assert!(matches!(
            ledger.commit_step(run, &next, &[fact(90, WorkOutcome::Completed)]),
            HistoryWrite::Unconfirmed(_)
        ));
        assert_eq!(flags(ledger), before);
        let content = crate::note_document::Document::parse(&next.notes[0].artifact.text)
            .unwrap()
            .content()
            .unwrap()
            .to_owned();
        let parsed = crate::markdown_claims::extract_content(&content).unwrap();
        let content = parsed
            .annotate_resolved(
                &[
                    Some(vpt_domain::grounding::Finding::BuiltOnFlaggedText),
                    Some(vpt_domain::grounding::Finding::Unsourced),
                ],
                &[Resolution::Open, resolution.clone()],
            )
            .unwrap();
        set_content(&mut next, &content);
        committed(ledger.commit_step(run, &next, &[fact(90, WorkOutcome::Completed)]));
        finish(ledger, run, &next.notes[0].artifact.target);
        assert_eq!(flags(ledger), before);
        old = next.notes[0].artifact.clone();
    }
    // Editing the claim must discard its old resolution without changing transcript findings.
    let mut next = next(ledger, &old);
    let Some(ProposalChange::Accept(proposal)) = &mut next.proposal else {
        unreachable!()
    };
    proposal.proposal.actions[0].claim.claim.text = "a changed task".into();
    let content = vpt_domain::note::analysis_content(&proposal.proposal).unwrap();
    set_content(&mut next, &content);
    committed(ledger.commit_step(run, &next, &[fact(90, WorkOutcome::Completed)]));
    let saved = flags(ledger);
    assert!(
        !saved
            .iter()
            .any(|flag| flag.identifier == reviewed.identifier)
    );
    assert_eq!(
        saved
            .iter()
            .filter(|flag| matches!(flag.content, FlagContent::Verification(_)))
            .count(),
        3
    );
    assert!(saved.iter().all(|flag| flag.resolution == Resolution::Open));
}

pub(crate) fn analysis_findings_roll_back_on_late_failure(ledger: &impl Ledger) {
    let run = seed(ledger);
    let initial = delta(accepted(ledger));
    committed(ledger.commit_step(run, &initial, &[fact(70, WorkOutcome::Completed)]));
    finish(ledger, run, &initial.notes[0].artifact.target);
    let before = flags(ledger);
    let history = ledger.pending_exports().unwrap();
    let mut next = next(ledger, &initial.notes[0].artifact);
    let Some(ProposalChange::Accept(proposal)) = &mut next.proposal else {
        unreachable!()
    };
    proposal.proposal.actions.clear();
    let content = vpt_domain::note::analysis_content(&proposal.proposal).unwrap();
    set_content(&mut next, &content);
    let bad = HistoryFact::HookFailed {
        event: vpt_domain::history::EventId::new(run.namespace(), 999_999),
        failure: vpt_domain::run::HookFailureClass::Spawn,
    };
    assert!(matches!(
        ledger.commit_step(
            run,
            &next,
            &[
                fact(90, WorkOutcome::Completed),
                (UtcInstant { secs: 90 }, bad)
            ]
        ),
        HistoryWrite::Unconfirmed(_)
    ));
    assert_eq!(flags(ledger), before);
    assert_eq!(ledger.pending_exports().unwrap(), history);
    assert_eq!(
        ledger.note(&record(81).id, NoteStage::Analysis).unwrap(),
        Some(initial.notes[0].artifact.clone())
    );
    assert!(ledger.pending_publications().unwrap().is_empty());
    // The same valid proposal succeeds once the late failure is removed.
    committed(ledger.commit_step(run, &next, &[fact(90, WorkOutcome::Completed)]));
    assert_eq!(flags(ledger).len(), before.len() - 1);
}

pub(crate) fn analysis_refuses_competing_verification_in_the_same_transaction(
    ledger: &impl Ledger,
) {
    let run = seed(ledger);
    let before = flags(ledger);
    let history = ledger.pending_exports().unwrap();
    let mut next = delta(accepted(ledger));
    let mut verification = super::super::verification::change(ledger);
    verification.at.secs = 70;
    next.verification = Some(verification);
    assert!(matches!(
        ledger.commit_step(run, &next, &[fact(70, WorkOutcome::Completed)]),
        HistoryWrite::Unconfirmed(_)
    ));
    assert_eq!(flags(ledger), before);
    assert_eq!(ledger.pending_exports().unwrap(), history);
    assert!(ledger.accepted_proposal(&record(81).id).unwrap().is_none());
}

pub(crate) fn analysis_findings_follow_rendered_markdown_and_recheck_idempotently(
    ledger: &impl Ledger,
) {
    use vpt_domain::grounding::Finding;
    let run = seed(ledger);
    let mut old: Option<NoteArtifact> = None;
    for (source, expected) in [
        ("private invalid source", Finding::Unsourced),
        ("00:00-00:99", Finding::BadReference),
        ("bad]\n<script>secret</script>", Finding::Unsourced),
    ] {
        let mut proposal = accepted(ledger);
        let transcript = ledger
            .accepted_transcript(&proposal.recording)
            .unwrap()
            .unwrap();
        let previous = flags(ledger);
        let claim = &mut proposal.proposal.summary[0];
        claim.claim.source_references = vec![source.into()];
        claim.grounding =
            Grounder::for_transcript(&transcript.transcript, &previous, transcript.duration, 0)
                .unwrap()
                .check(&claim.claim)
                .unwrap();
        assert_eq!(claim.grounding.finding, Some(Finding::BadReference));
        let mut next = delta(proposal.clone());
        if let Some(old) = &old {
            next.notes[0].expected_saved = Some(old.digest);
            next.notes[0].artifact.first_written = old.first_written;
            next.commit.publications[0].expected_previous = Some(old.digest);
        }
        let raw = vpt_domain::note::analysis_content(&proposal.proposal).unwrap();
        let checked =
            crate::verification::check(&raw, Some(NoteStage::Analysis), &transcript, &previous, 0)
                .unwrap();
        assert_eq!(checked.findings.len(), 2);
        assert_eq!(checked.findings[0].finding, expected);
        set_content(&mut next, &checked.content);
        committed(ledger.commit_step(run, &next, &[fact(70, WorkOutcome::Completed)]));
        finish(ledger, run, &next.notes[0].artifact.target);
        let saved = flags(ledger);
        let rechecked = crate::verification::check(
            &checked.content,
            Some(NoteStage::Analysis),
            &transcript,
            &saved,
            0,
        )
        .unwrap();
        assert_eq!(checked.content, rechecked.content);
        assert_eq!(checked.findings, rechecked.findings);
        assert_eq!(
            ledger.accepted_proposal(&proposal.recording).unwrap(),
            Some(proposal)
        );
        let mut persisted: Vec<_> = saved
            .into_iter()
            .filter_map(|flag| match flag.content {
                FlagContent::Verification(finding) => Some(finding),
                _ => None,
            })
            .collect();
        persisted.sort_by_key(|finding| finding.claim_index);
        assert_eq!(persisted, checked.findings);
        assert!(!next.notes[0].artifact.text.contains("<script>"));
        old = Some(next.notes[0].artifact.clone());
    }
}
