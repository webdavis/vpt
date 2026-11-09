use super::*;
use sha2::{Digest, Sha256};
use vpt_domain::{digest::Sha256Digest, grounding::Finding};
pub(super) fn plan(source: &str, flags: &[ReviewFlag], policy: FlagPolicy) -> Result<Plan, Error> {
    if flags.len() > MAX_ITEMS + 3 {
        return Err(Error::Capacity);
    }
    let parsed = crate::markdown_claims::extract_owned(source, NoteStage::Analysis, flags)
        .map_err(|_| Error::InvalidText)?;
    let mut edits = Vec::new();
    for flag in flags {
        let FlagContent::Verification(finding) = &flag.content else {
            continue;
        };
        if finding.artifact != NoteStage::Analysis
            || finding.finding != Finding::BuiltOnFlaggedText
            || flag.resolution != Resolution::Open
        {
            continue;
        }
        if !finding.valid() {
            return Err(Error::InvalidText);
        }
        let claim = parsed
            .claims()
            .get(finding.claim_index as usize)
            .ok_or(Error::InvalidText)?;
        if claim.identity_text != finding.record_text
            || Sha256Digest(Sha256::digest(claim.identity_text.as_bytes()).into())
                != finding.claim_digest
        {
            return Err(Error::InvalidText);
        }
        let range = claim.source_range();
        let removals = parsed.annotation_ranges();
        let first = removals.partition_point(|r| r.end <= range.start);
        let edits_within = removals[first..]
            .iter()
            .take_while(|r| r.start < range.end)
            .map(|r| map::Edit {
                range: r.start.max(range.start) - range.start..r.end.min(range.end) - range.start,
                text: String::new(),
                marker: None,
            })
            .collect();
        let accepted = map::Mapped::apply(&source[range.clone()], edits_within)?.text;
        edits.push(edit(range, accepted, policy));
    }
    Ok(Plan {
        count: edits.len(),
        edits,
    })
}
