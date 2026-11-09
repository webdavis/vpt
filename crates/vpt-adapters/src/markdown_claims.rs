use std::ops::Range;
use vpt_domain::proposal::Claim;
mod blocks;
mod corrections;
mod extract;
mod render;
mod sentences;
mod tokens;
const MAX_ITEMS: usize = 65_536;
const MAX_TEXT: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimError {
    CapacityExceeded,
    InvalidAnnotations,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ExtractedClaim {
    pub index: u32,
    pub claim: Claim,
    pub identity_text: String,
    start: usize,
    end: usize,
    line: usize,
}
impl ExtractedClaim {
    pub fn source_range(&self) -> Range<usize> {
        self.start..self.end
    }
    pub fn line(&self) -> usize {
        self.line
    }
}
impl std::fmt::Debug for ExtractedClaim {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExtractedClaim")
            .field("index", &self.index)
            .field("end", &self.end)
            .finish()
    }
}

pub struct Claims<'a> {
    source: &'a str,
    claims: Vec<ExtractedClaim>,
    removals: Vec<Range<usize>>,
}

impl Claims<'_> {
    pub fn annotation_ranges(&self) -> &[Range<usize>] {
        &self.removals
    }
    pub fn claims(&self) -> &[ExtractedClaim] {
        &self.claims
    }
    pub fn digest(&self) -> vpt_domain::digest::Sha256Digest {
        use sha2::{Digest, Sha256};
        let mut digest = Sha256::new();
        digest.update(b"vpt.claims/1\0");
        for claim in &self.claims {
            digest.update(claim.index.to_le_bytes());
            digest.update((claim.identity_text.len() as u64).to_le_bytes());
            digest.update(claim.identity_text.as_bytes());
        }
        vpt_domain::digest::Sha256Digest(digest.finalize().into())
    }
}

pub fn extract(source: &str) -> Result<Claims<'_>, ClaimError> {
    parse(source, true, None)
}

pub fn extract_content(source: &str) -> Result<Claims<'_>, ClaimError> {
    parse(source, false, None)
}

pub fn extract_owned<'a>(
    source: &'a str,
    stage: vpt_domain::note::NoteStage,
    flags: &[vpt_domain::review::ReviewFlag],
) -> Result<Claims<'a>, ClaimError> {
    if flags.len() > 65_539 {
        return Err(ClaimError::CapacityExceeded);
    }
    parse(
        source,
        false,
        (stage == vpt_domain::note::NoteStage::Transcript).then_some(flags),
    )
}

fn parse<'a>(
    source: &'a str,
    frontmatter: bool,
    flags: Option<&[vpt_domain::review::ReviewFlag]>,
) -> Result<Claims<'a>, ClaimError> {
    if source.len() > vpt_domain::note::MAX_RENDERED
        || source
            .bytes()
            .filter(|byte| byte.is_ascii_punctuation() || *byte == b'\n')
            .take(MAX_ITEMS * 16 + 1)
            .count()
            > MAX_ITEMS * 16
    {
        return Err(ClaimError::CapacityExceeded);
    }
    extract::claims(source, frontmatter, flags)
}

#[cfg(test)]
mod tests;
