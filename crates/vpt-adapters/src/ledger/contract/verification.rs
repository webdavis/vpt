use super::{
    engine_outputs::committed,
    notes,
    proposals::{self, fixture},
    record,
};
use vpt_application::ports::*;
use vpt_domain::{
    grounding::Finding, review::VerificationFlag, run::WorkOutcome, time::UtcInstant,
};

pub(crate) fn change(ledger: &impl proposals::Ledger) -> VerificationChange {
    let recording = record(81).id;
    VerificationChange {
        grounding_window_secs: 15,
        observed: None,
        input_digest: ledger
            .note(&recording, NoteStage::Transcript)
            .unwrap()
            .unwrap()
            .content_digest,
        basis: ledger.grounding_basis(&recording).unwrap().unwrap(),
        recording,
        artifact: NoteStage::Transcript,
        findings: vec![VerificationFlag {
            artifact: NoteStage::Transcript,
            claim_index: 0,
            claim_digest: notes::hash(b"private claim"),
            record_text: "private claim".into(),
            finding: Finding::Unsourced,
            sources: vec![],
        }],
        at: UtcInstant { secs: 80 },
    }
}

pub(crate) fn saved_findings_join_review_but_not_transcript_flags(
    ledger: &(impl proposals::Ledger + ReviewLedger),
) {
    let run = fixture::seed(ledger);
    let change = change(ledger);
    let id = change.recording.clone();
    let before = ledger.review_snapshot(&id).unwrap().unwrap().flags;
    committed(ledger.commit_step(
        run,
        &WorkDelta {
            verification: Some(change),
            ..Default::default()
        },
        &[fixture::fact(80, WorkOutcome::Completed)],
    ));
    let after = ledger.review_snapshot(&id).unwrap().unwrap().flags;
    assert_eq!(after.len(), before.len() + 1);
    assert_eq!(ledger.transcript_flags(&id).unwrap(), before);
}

mod claims;
mod window;
pub(crate) use window::*;
mod freshness;
pub(crate) use claims::*;
mod observed;
mod refusals;
mod replacement;
pub(crate) use freshness::*;
pub(crate) use observed::*;
pub(crate) use refusals::*;
pub(crate) use replacement::*;

pub(crate) fn save(
    ledger: &impl RunHistory,
    run: vpt_domain::history::RunId,
    change: VerificationChange,
) -> HistoryWrite<HistoryReceipt> {
    ledger.commit_step(
        run,
        &WorkDelta {
            verification: Some(change),
            ..Default::default()
        },
        &[fixture::fact(80, WorkOutcome::Completed)],
    )
}

pub(crate) fn findings(ledger: &impl ReviewLedger) -> Vec<vpt_domain::review::ReviewFlag> {
    ledger
        .review_snapshot(&record(81).id)
        .unwrap()
        .unwrap()
        .flags
        .into_iter()
        .filter(|flag| {
            matches!(
                flag.content,
                vpt_domain::review::FlagContent::Verification(_)
            )
        })
        .collect()
}

pub(crate) fn analysis(ledger: &impl proposals::Ledger, run: vpt_domain::history::RunId) {
    let accepted = fixture::accepted(ledger);
    let delta = fixture::delta(accepted);
    committed(ledger.commit_step(run, &delta, &[fixture::fact(70, WorkOutcome::Completed)]));
    fixture::finish(ledger, run, &delta.notes[0].artifact.target);
}

macro_rules! cases {
    ($case:ident) => {
        $case!(verification_windows_survive_empty_results_and_artifact_replacement);
        $case!(saved_findings_join_review_but_not_transcript_flags);
        $case!(lexical_changes_hide_stale_findings_but_rechecking_preserves_resolutions);
        $case!(transcript_changes_hide_stale_findings_from_both_snapshot_readers);
        $case!(stale_verification_cannot_be_resolved_until_rechecked);
        $case!(empty_verification_results_still_record_their_basis);
        $case!(changed_owned_claims_invalidate_only_the_changed_artifact);
        $case!(changed_claims_invalidate_an_empty_verification_result);
        $case!(replacement_is_artifact_scoped_and_preserves_only_unchanged_claim_resolutions);
        $case!(transcription_preserves_owned_findings_and_keeps_them_out_of_classification);
        $case!(every_finding_shape_round_trips_without_private_debug);
        $case!(invalid_or_stale_findings_leave_state_and_history_unchanged);
        $case!(late_history_failure_rolls_back_finding_replacement);
        $case!(same_batch_content_change_refuses_stale_findings);
        $case!(annotation_only_content_update_commits_with_findings);
        $case!(observed_claim_edits_commit_only_with_the_checked_source_and_matching_publication);
        $case!(pending_publication_and_retention_refuse_verification);
    };
}
pub(crate) use cases;
