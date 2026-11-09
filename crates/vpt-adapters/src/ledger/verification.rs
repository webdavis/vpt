mod content;
pub(super) mod state;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use vpt_application::ports::*;
use vpt_domain::{
    review::{FlagContent, Resolution, ReviewFlag},
    run::{HistoryFact, WorkOutcome},
    time::UtcInstant,
};

pub(super) fn conflict() -> LedgerError {
    LedgerError::Conflict("verification state conflict".into())
}
pub(super) fn invalid() -> LedgerError {
    LedgerError::Corrupt("invalid verification findings".into())
}

pub(super) fn prepare(
    change: &VerificationChange,
    delta: &WorkDelta,
    facts: &[(UtcInstant, HistoryFact)],
    note: &NoteArtifact,
    transcript: &AcceptedTranscript,
    flags: &[ReviewFlag],
) -> Result<(Vec<ReviewFlag>, state::VerifiedArtifact), LedgerError> {
    if note.recording != change.recording
        || note.stage != change.artifact
        || super::proposal::basis::compute(transcript, flags)? != change.basis
        || delta.transcription.is_some()
        || delta.resolution.is_some()
    {
        return Err(conflict());
    }
    let claims_digest = content::check(change, delta, note, flags)?;
    let mut work = facts.iter().filter(|(_, fact)| matches!(fact, HistoryFact::WorkStep { recording: Some(id), .. } if id == &change.recording));
    if work.next().is_none_or(|(at, fact)| {
        *at != change.at
            || !matches!(
                fact,
                HistoryFact::WorkStep {
                    outcome: WorkOutcome::Completed,
                    ..
                }
            )
    }) || work.next().is_some()
    {
        return Err(conflict());
    }
    let findings = prepare_findings(
        &change.recording,
        change.artifact,
        &change.findings,
        transcript,
        flags,
    )?;
    Ok((
        findings,
        state::VerifiedArtifact {
            recording: change.recording.clone(),
            artifact: change.artifact,
            input_digest: change.input_digest,
            claims_digest: Some(claims_digest),
            grounding_window_secs: Some(change.grounding_window_secs),
            basis: change.basis,
            at: change.at,
        },
    ))
}

pub(crate) fn prepare_findings(
    recording: &vpt_domain::identity::RecordingId,
    artifact: NoteStage,
    findings: &[vpt_domain::review::VerificationFlag],
    transcript: &AcceptedTranscript,
    flags: &[ReviewFlag],
) -> Result<Vec<ReviewFlag>, LedgerError> {
    if findings.len() > 65_536 || flags.len() > 65_539 {
        return Err(invalid());
    }
    let previous: HashMap<_, _> = flags
        .iter()
        .map(|flag| (flag.identifier, &flag.resolution))
        .collect();
    let mut indices = HashSet::new();
    let mut bytes = 0usize;
    let mut ranges = 0usize;
    let mut result = Vec::new();
    for finding in findings {
        bytes = bytes.saturating_add(finding.record_text.len());
        if let vpt_domain::grounding::Finding::Unsupported(token) = &finding.finding {
            bytes = bytes.saturating_add(token.len());
        }
        ranges = ranges.saturating_add(finding.sources.len());
        if bytes > 16 * 1024 * 1024
            || ranges > 65_536
            || !finding.valid()
            || finding.artifact != artifact
            || !indices.insert(finding.claim_index)
            || finding.claim_digest.0
                != <[u8; 32]>::from(Sha256::digest(finding.record_text.as_bytes()))
            || finding
                .sources
                .iter()
                .any(|source| source.end_secs as f64 > transcript.duration)
        {
            return Err(invalid());
        }
        let identifier = crate::verification_flag_id(recording, finding);
        let resolution = previous
            .get(&identifier)
            .map_or(Resolution::Open, |value| (*value).clone());
        result.push(ReviewFlag {
            identifier,
            content: FlagContent::Verification(finding.clone()),
            resolution,
        });
    }
    let retained = flags.iter().filter(|flag| !matches!(&flag.content, FlagContent::Verification(finding) if finding.artifact == artifact));
    super::flags::budget::check(retained.chain(result.iter()))?;
    Ok(result)
}
