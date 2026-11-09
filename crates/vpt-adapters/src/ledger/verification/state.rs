use vpt_application::ports::{GroundingBasis, LedgerError, NoteStage, ReviewSnapshot, WorkDelta};
use vpt_domain::{
    digest::Sha256Digest, identity::RecordingId, review::FlagContent, time::UtcInstant,
};

#[derive(Clone)]
pub(in crate::ledger) struct VerifiedArtifact {
    pub recording: RecordingId,
    pub artifact: NoteStage,
    pub input_digest: Sha256Digest,
    pub claims_digest: Option<Sha256Digest>,
    pub grounding_window_secs: Option<u32>,
    pub basis: GroundingBasis,
    pub at: UtcInstant,
}

pub(in crate::ledger) fn partition(
    review: &mut ReviewSnapshot,
    saved: &[VerifiedArtifact],
    mut claims: impl FnMut(
        NoteStage,
        &[vpt_domain::review::ReviewFlag],
    ) -> Result<Option<Sha256Digest>, LedgerError>,
) -> Result<(), LedgerError> {
    let basis = review
        .accepted
        .as_ref()
        .map(|accepted| super::super::proposal::basis::compute(accepted, &review.flags))
        .transpose()?;
    for stage in [NoteStage::Transcript, NoteStage::Analysis] {
        let state = saved.iter().find(|row| row.artifact == stage);
        let has_findings = review.flags.iter().any(|flag| {
            matches!(&flag.content,
            FlagContent::Verification(finding) if finding.artifact == stage)
        });
        let current = if let Some(row) = state.filter(|row| Some(row.basis) == basis) {
            row.grounding_window_secs.is_some()
                && row.claims_digest.is_some()
                && row.claims_digest == claims(stage, &review.flags)?
        } else {
            false
        };
        if current {
            let row = state.ok_or_else(super::invalid)?;
            review
                .verified_claims
                .push(vpt_application::ports::VerifiedClaims {
                    artifact: stage,
                    digest: row.claims_digest.ok_or_else(super::invalid)?,
                    grounding_window_secs: row.grounding_window_secs.ok_or_else(super::invalid)?,
                });
        } else if state.is_some() || has_findings {
            review.stale_artifacts.push(stage);
        }
    }
    let mut current = Vec::with_capacity(review.flags.len());
    for flag in std::mem::take(&mut review.flags) {
        if matches!(&flag.content, FlagContent::Verification(finding) if review.stale_artifacts.contains(&finding.artifact))
        {
            review.stale_findings.push(flag);
        } else {
            current.push(flag);
        }
    }
    review.flags = current;
    Ok(())
}

pub(in crate::ledger) fn affected(delta: &WorkDelta) -> Vec<&RecordingId> {
    let mut ids: Vec<_> = [
        delta
            .transcription
            .as_ref()
            .map(|change| change.recording()),
        delta.resolution.as_ref().map(|change| &change.recording),
        delta.verification.as_ref().map(|change| &change.recording),
        delta.proposal.as_ref().map(|change| change.recording()),
    ]
    .into_iter()
    .flatten()
    .collect();
    ids.extend(delta.notes.iter().map(|note| &note.artifact.recording));
    ids.sort_by_key(|id| id.as_str());
    ids.dedup();
    ids
}

pub(in crate::ledger) fn claims(
    note: &vpt_application::ports::NoteArtifact,
    flags: &[vpt_domain::review::ReviewFlag],
) -> Result<Sha256Digest, LedgerError> {
    let document =
        crate::note_document::Document::parse(&note.text).map_err(|_| super::invalid())?;
    crate::markdown_claims::extract_owned(
        document.content().map_err(|_| super::invalid())?,
        note.stage,
        flags,
    )
    .map(|parsed| parsed.digest())
    .map_err(|_| super::invalid())
}
