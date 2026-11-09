use super::{ClaimError, blocks::Block, tokens::Token};
use std::{collections::HashMap, ops::Range};
use vpt_domain::{
    note::NoteStage,
    review::{FlagContent, Resolution, ReviewFlag},
};

pub(super) struct Saved<'a>(HashMap<u32, &'a ReviewFlag>);

impl<'a> Saved<'a> {
    pub fn new(flags: &'a [ReviewFlag]) -> Result<Self, ClaimError> {
        let mut saved = HashMap::new();
        for flag in flags {
            if let FlagContent::Verification(finding) = &flag.content
                && finding.artifact == NoteStage::Transcript
                && matches!(flag.resolution, Resolution::Corrected { .. })
                && saved.insert(finding.claim_index, flag).is_some()
            {
                return Err(ClaimError::InvalidAnnotations);
            }
        }
        Ok(Self(saved))
    }

    pub fn recognize(
        &self,
        block: &Block,
        range: Range<usize>,
        tokens: &mut [Token],
        source: &str,
        index: u32,
    ) -> Result<(), ClaimError> {
        let Some(flag) = self.0.get(&index) else {
            return Ok(());
        };
        let Some(candidate) = tokens.last() else {
            return Ok(());
        };
        if !candidate.correction
            || candidate.annotation
            || !block.text[candidate.range.end..range.end].trim().is_empty()
        {
            return Ok(());
        }
        let FlagContent::Verification(finding) = &flag.content else {
            unreachable!()
        };
        let annotation =
            vpt_domain::note::resolved_finding_annotation(&finding.finding, &flag.resolution)
                .map_err(|_| ClaimError::InvalidAnnotations)?
                .ok_or(ClaimError::InvalidAnnotations)?;
        let legacy = annotation
            .strip_suffix(vpt_domain::note::FINDING_CORRECTION_MARKER)
            .ok_or(ClaimError::InvalidAnnotations)?;
        if source[candidate.raw.clone()] != *legacy {
            return Ok(());
        }
        let last = tokens.len() - 1;
        // Only a suffix whose removal restores the saved claim is a finding correction.
        tokens[last].annotation = true;
        let matches = super::extract::assemble(block, range, tokens, source)?
            .is_some_and(|claim| claim.identity_text == finding.record_text);
        tokens[last].annotation = matches;
        Ok(())
    }
}
