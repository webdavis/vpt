use super::NoteError;
use crate::markdown_claims::{self, ClaimError};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use vpt_domain::{
    digest::Sha256Digest,
    note::NoteStage,
    review::{FlagContent, Resolution, ReviewFlag},
};

pub(super) fn clear(content: &str) -> Result<String, NoteError> {
    let parsed = markdown_claims::extract_content(content).map_err(error)?;
    parsed
        .annotate(&vec![None; parsed.claims().len()])
        .map_err(error)
}

pub(super) fn claims_digest(
    content: &str,
    stage: NoteStage,
    flags: &[ReviewFlag],
) -> Result<Sha256Digest, NoteError> {
    markdown_claims::extract_owned(content, stage, flags)
        .map(|parsed| parsed.digest())
        .map_err(error)
}

pub(super) fn render(
    content: &str,
    stage: NoteStage,
    flags: &[ReviewFlag],
) -> Result<String, NoteError> {
    if content.len() > vpt_domain::note::MAX_RENDERED || flags.len() > 65_539 {
        return Err(NoteError::CapacityExceeded);
    }
    let previous: HashMap<_, _> = flags
        .iter()
        .filter_map(|flag| match &flag.content {
            FlagContent::Verification(finding) if finding.artifact == stage => Some((
                (finding.claim_index, finding.claim_digest),
                (&finding.finding, &flag.resolution),
            )),
            _ => None,
        })
        .collect();
    if previous.is_empty() {
        return Ok(content.to_owned());
    }
    let parsed = markdown_claims::extract_owned(content, stage, flags).map_err(error)?;
    let mut findings = Vec::new();
    let mut resolutions = Vec::new();
    for claim in parsed.claims() {
        let digest = Sha256Digest(Sha256::digest(claim.identity_text.as_bytes()).into());
        match previous.get(&(claim.index, digest)) {
            Some((finding, resolution)) => {
                findings.push(Some((*finding).clone()));
                resolutions.push((*resolution).clone());
            }
            None => {
                findings.push(None);
                resolutions.push(Resolution::Open);
            }
        }
    }
    parsed
        .annotate_resolved(&findings, &resolutions)
        .map_err(error)
}
fn error(error: ClaimError) -> NoteError {
    match error {
        ClaimError::CapacityExceeded => NoteError::CapacityExceeded,
        ClaimError::InvalidAnnotations => NoteError::Markers,
    }
}

#[cfg(test)]
mod tests;
