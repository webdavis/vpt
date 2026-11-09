use super::{basis, codec, conflict, invalid};
use vpt_application::ports::*;
use vpt_domain::{
    grounding::Grounder,
    review::ReviewFlag,
    run::{HistoryFact, WorkOutcome},
    time::UtcInstant,
};

pub(in crate::ledger) fn history(
    delta: &WorkDelta,
    facts: &[(UtcInstant, HistoryFact)],
) -> Result<(), LedgerError> {
    let Some(change) = &delta.proposal else {
        return Ok(());
    };
    if delta
        .verification
        .as_ref()
        .is_some_and(|value| &value.recording == change.recording())
        || delta
            .transcription
            .as_ref()
            .is_some_and(|value| value.recording() == change.recording())
        || delta
            .resolution
            .as_ref()
            .is_some_and(|value| &value.recording == change.recording())
    {
        return Err(conflict());
    }
    let expected = match change {
        ProposalChange::Accept(_) => WorkOutcome::Completed,
        ProposalChange::Fail { .. } => WorkOutcome::Failed,
    };
    let mut work = facts.iter().filter(|(_,fact)| matches!(fact, HistoryFact::WorkStep {recording:Some(id),..} if id==change.recording()));
    if work.next().is_none_or(|(at, fact)| {
        *at != change.at()
            || !matches!(fact, HistoryFact::WorkStep {outcome,..} if *outcome == expected)
    }) || work.next().is_some()
    {
        return Err(conflict());
    }
    Ok(())
}

pub(in crate::ledger) struct Prepared {
    pub body: String,
    pub findings: Vec<ReviewFlag>,
    pub verification: super::super::verification::state::VerifiedArtifact,
}

pub(in crate::ledger) fn check(
    accepted: &AcceptedProposal,
    delta: &WorkDelta,
    transcript: &AcceptedTranscript,
    flags: &[ReviewFlag],
    source: &NoteArtifact,
    source_stage: StageState,
) -> Result<Prepared, LedgerError> {
    let body = codec::encode(accepted)?;
    if accepted.vocabulary_publication.as_ref()
        != delta.vocabulary.as_ref().map(|value| &value.entry)
        || delta
            .vocabulary
            .as_ref()
            .is_some_and(|value| value.kind != VocabularyKind::Tags)
        || source_stage != StageState::Succeeded
        || source.recording != accepted.recording
        || transcript.recording != accepted.recording
        || source.content_digest != accepted.input_digest
        || basis::compute(transcript, flags)? != accepted.basis
    {
        return Err(conflict());
    }
    if delta.notes.iter().any(|change| {
        change.artifact.recording == accepted.recording
            && change.artifact.stage == NoteStage::Transcript
            && change.artifact.content_digest != accepted.input_digest
    }) {
        return Err(conflict());
    }
    let mut checker = Grounder::new(
        transcript.transcript.segments.iter().flat_map(|s| &s.words),
        flags,
        transcript.duration,
        accepted.grounding_window_secs,
    )
    .map_err(|_| invalid())?;
    for checked in accepted
        .proposal
        .summary
        .iter()
        .chain(&accepted.proposal.actions)
    {
        if checker.check(&checked.claim).map_err(|_| invalid())? != checked.grounding {
            return Err(invalid());
        }
    }
    let mut notes = delta.notes.iter().filter(|change| {
        change.artifact.recording == accepted.recording
            && change.artifact.stage == NoteStage::Analysis
    });
    let analysis = &notes.next().ok_or_else(conflict)?.artifact;
    let content = vpt_domain::note::analysis_content(&accepted.proposal).map_err(|_| invalid())?;
    let checked = crate::verification::check(
        &content,
        Some(NoteStage::Analysis),
        transcript,
        flags,
        accepted.grounding_window_secs,
    )
    .map_err(|_| invalid())?;
    let findings = super::super::verification::prepare_findings(
        &accepted.recording,
        NoteStage::Analysis,
        &checked.findings,
        transcript,
        flags,
    )?;
    let parsed = crate::note_document::Document::parse(&analysis.text).map_err(|_| invalid())?;
    if notes.next().is_some()
        || analysis.updated_at != accepted.accepted_at
        || parsed.content().map_err(|_| invalid())? != checked.content
    {
        return Err(conflict());
    }
    Ok(Prepared {
        body,
        findings,
        verification: super::super::verification::state::VerifiedArtifact {
            recording: accepted.recording.clone(),
            artifact: NoteStage::Analysis,
            grounding_window_secs: Some(accepted.grounding_window_secs),
            input_digest: checked.input_digest,
            claims_digest: Some(
                crate::markdown_claims::extract_owned(&content, NoteStage::Analysis, flags)
                    .map_err(|_| invalid())?
                    .digest(),
            ),
            basis: accepted.basis,
            at: accepted.accepted_at,
        },
    })
}
