use super::{engine_outputs::committed, notes, record, transcript};
use vpt_application::ports::*;
use vpt_domain::{
    grounding::Grounder,
    history::RunId,
    proposal::{Claim, Proposal, Related, Tag},
    run::{HistoryFact, WorkOutcome},
    time::UtcInstant,
    uncertainty::{Class, LexicalFlag},
};
mod durability;
mod vocabulary;
pub(crate) use vocabulary::*;
mod findings;
pub(crate) use findings::*;
mod republication;
pub(crate) use republication::*;
pub(crate) mod fixture;
mod freshness;
pub(crate) use durability::*;
use fixture::*;
pub(crate) use freshness::*;

pub(crate) trait Ledger:
    RecordingLedger
    + RunHistory
    + ProposalLedger
    + NoteLedger
    + NoteInputsLedger
    + TranscriptLedger
    + FlagLedger
    + PublicationJournal
    + RetentionJournal
{
}
impl<
    T: RecordingLedger
        + RunHistory
        + ProposalLedger
        + NoteLedger
        + NoteInputsLedger
        + TranscriptLedger
        + FlagLedger
        + PublicationJournal
        + RetentionJournal,
> Ledger for T
{
}

pub(crate) fn acceptance_commits_proposal_analysis_and_history_before_publication(
    ledger: &impl Ledger,
) {
    let run = seed(ledger);
    let accepted = accepted(ledger);
    assert!(
        ledger
            .note_inputs(&accepted.recording)
            .unwrap()
            .unwrap()
            .proposal
            .is_none()
    );
    let delta = delta(accepted.clone());
    let receipt = committed(ledger.commit_step(run, &delta, &[fact(70, WorkOutcome::Completed)]));
    assert_eq!(
        ledger.accepted_proposal(&accepted.recording).unwrap(),
        Some(accepted.clone())
    );
    let inputs = ledger.note_inputs(&accepted.recording).unwrap().unwrap();
    assert_eq!(inputs.proposal, Some(accepted.clone()));
    let mut findings: Vec<_> = inputs
        .review
        .flags
        .iter()
        .filter_map(|flag| match &flag.content {
            vpt_domain::review::FlagContent::Verification(finding) => {
                assert_eq!(finding.artifact, NoteStage::Analysis);
                assert_eq!(flag.resolution, vpt_domain::review::Resolution::Open);
                Some(finding)
            }
            _ => None,
        })
        .collect();
    findings.sort_by_key(|finding| finding.claim_index);
    assert_eq!(findings.len(), 2, "initial analysis must save its findings");
    assert_eq!(
        findings[0].finding,
        vpt_domain::grounding::Finding::BuiltOnFlaggedText
    );
    assert_eq!(
        findings[1].finding,
        vpt_domain::grounding::Finding::Unsourced
    );
    assert!(inputs.notes.contains(&delta.notes[0].artifact));
    assert_eq!(receipt.completed, vec![accepted.recording.clone()]);
    assert_eq!(
        ledger
            .by_id(&accepted.recording)
            .unwrap()
            .unwrap()
            .stages
            .analysis,
        StageState::Pending
    );
    assert_eq!(
        ledger
            .note(&accepted.recording, NoteStage::Analysis)
            .unwrap(),
        Some(delta.notes[0].artifact.clone())
    );
    assert_eq!(
        ledger.pending_publications().unwrap(),
        delta.commit.publications
    );
    assert!(!format!("{receipt:?} {delta:?}").contains("Privé"));
    finish(ledger, run, &delta.notes[0].artifact.target);
    assert_eq!(
        ledger
            .by_id(&accepted.recording)
            .unwrap()
            .unwrap()
            .stages
            .analysis,
        StageState::Succeeded
    );
}

pub(crate) fn failed_analysis_preserves_accepted_proposal_and_analysis(ledger: &impl Ledger) {
    let run = seed(ledger);
    let accepted = accepted(ledger);
    let delta = delta(accepted.clone());
    committed(ledger.commit_step(run, &delta, &[fact(70, WorkOutcome::Completed)]));
    finish(ledger, run, &delta.notes[0].artifact.target);
    let failed = WorkDelta {
        proposal: Some(ProposalChange::Fail {
            recording: accepted.recording.clone(),
            at: UtcInstant { secs: 80 },
        }),
        ..Default::default()
    };
    let receipt = committed(ledger.commit_step(run, &failed, &[fact(80, WorkOutcome::Failed)]));
    assert!(receipt.completed.is_empty());
    assert_eq!(
        ledger.accepted_proposal(&accepted.recording).unwrap(),
        Some(accepted.clone())
    );
    assert_eq!(
        ledger
            .note(&accepted.recording, NoteStage::Analysis)
            .unwrap(),
        Some(delta.notes[0].artifact.clone())
    );
    assert_eq!(
        ledger
            .by_id(&accepted.recording)
            .unwrap()
            .unwrap()
            .stages
            .analysis,
        StageState::Failed
    );
}

pub(crate) fn missing_publication_stale_input_or_forged_grounding_rolls_back_everything(
    ledger: &impl Ledger,
) {
    let run = seed(ledger);
    let before = ledger.pending_exports().unwrap();
    for variant in 0..5 {
        let mut accepted = accepted(ledger);
        match variant {
            1 => accepted.input_digest = super::digest(7),
            2 => accepted.basis.lexical_flags = super::digest(8),
            3 => accepted.proposal.summary[0]
                .grounding
                .inherited_flags
                .clear(),
            4 => accepted.proposal.summary[0].grounding.sources[0].end_secs += 1,
            _ => (),
        }
        let mut delta = delta(accepted);
        if variant == 0 {
            delta.commit.publications.clear();
        }
        assert!(matches!(
            ledger.commit_step(run, &delta, &[fact(70, WorkOutcome::Completed)]),
            HistoryWrite::Unconfirmed(_)
        ));
        assert!(ledger.accepted_proposal(&record(81).id).unwrap().is_none());
        assert!(
            ledger
                .note(&record(81).id, NoteStage::Analysis)
                .unwrap()
                .is_none()
        );
        assert!(ledger.pending_publications().unwrap().is_empty());
        assert_eq!(ledger.pending_exports().unwrap(), before);
    }
}

macro_rules! cases {
    ($case:ident) => {
        $case!(proposal_vocabulary_dependency_survives_note_publication);
        $case!(proposal_vocabulary_dependency_requires_the_exact_same_transaction);
        $case!(proposal_vocabulary_dependency_rolls_back_with_failed_history);
        $case!(analysis_findings_follow_rendered_markdown_and_recheck_idempotently);
        $case!(analysis_preserves_reviewed_claims_and_other_artifact_findings);
        $case!(analysis_findings_roll_back_on_late_failure);
        $case!(analysis_refuses_competing_verification_in_the_same_transaction);
        $case!(analysis_metadata_publication_preserves_a_failed_analysis_attempt);
        $case!(analysis_republication_keeps_changed_transcript_content_pending);
        $case!(analysis_republication_keeps_changed_lexical_basis_pending);
        $case!(acceptance_commits_proposal_analysis_and_history_before_publication);
        $case!(failed_analysis_preserves_accepted_proposal_and_analysis);
        $case!(missing_publication_stale_input_or_forged_grounding_rolls_back_everything);
        $case!(later_success_replaces_accepted_state_and_reports_one_recording);
        $case!(late_history_failure_preserves_prior_proposal_note_stage_and_journal);
        $case!(proposal_requires_exact_matching_history);
        $case!(pending_source_publication_and_changed_transcription_refuse_acceptance);
        $case!(lexical_resolution_changes_basis_and_preserves_prior_proposal);
        $case!(unknown_recording_has_no_proposal_or_basis_and_cannot_fail_a_stage);
        $case!(pending_source_retention_refuses_acceptance);
    };
}
pub(crate) use cases;
