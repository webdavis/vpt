use crate::{digest::Sha256Digest, proposal::Claim, review::ReviewFlag, transcript::Word};
mod context;
mod corrections;
mod evidence;
mod facts;
mod proposal;
mod sources;
pub(crate) use sources::seconds as timecode_seconds;
mod uncertainty;

const MAX_ITEMS: usize = 65_536;
const MAX_TEXT_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceRange {
    pub start_secs: u64,
    pub end_secs: u64,
}

#[derive(Clone, PartialEq, Eq)]
pub enum Finding {
    Unsourced,
    BadReference,
    Unsupported(String),
    BuiltOnFlaggedText,
}
impl Finding {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unsourced => "unsourced",
            Self::BadReference => "bad-reference",
            Self::Unsupported(_) => "unsupported",
            Self::BuiltOnFlaggedText => "built-on-flagged-text",
        }
    }
}
impl std::fmt::Debug for Finding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Unsourced => "Unsourced",
            Self::BadReference => "BadReference",
            Self::Unsupported(_) => "Unsupported",
            Self::BuiltOnFlaggedText => "BuiltOnFlaggedText",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroundedClaim {
    pub sources: Vec<SourceRange>,
    pub finding: Option<Finding>,
    pub inherited_flags: Vec<Sha256Digest>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroundingError {
    InvalidTranscript,
    InvalidFlags,
    CapacityExceeded,
}

pub struct Grounder<'a> {
    words: Vec<&'a Word>,
    evidence: Vec<corrections::EvidenceWord<'a>>,
    flags: &'a [ReviewFlag],
    duration: f64,
    window_secs: u32,
    remaining: usize,
}

impl<'a> Grounder<'a> {
    pub fn check(&mut self, claim: &Claim) -> Result<GroundedClaim, GroundingError> {
        if claim.text.len() > MAX_ITEMS * 4
            || claim.text.chars().count() > MAX_ITEMS
            || claim.source_references.len() > 256
        {
            return Err(GroundingError::CapacityExceeded);
        }
        spend(&mut self.remaining, claim.text.len() + 1)?;
        if claim.source_references.is_empty() {
            return Ok(GroundedClaim::unresolved(Finding::Unsourced));
        }
        let Some(sources) = sources::resolve(&claim.source_references, self.duration) else {
            return Ok(GroundedClaim::unresolved(Finding::BadReference));
        };
        let inherited_flags =
            uncertainty::inherit(&self.words, self.flags, &sources, &mut self.remaining)?;
        let evidence = evidence::collect(
            &self.evidence,
            &sources,
            self.window_secs,
            &mut self.remaining,
        )?;
        let unsupported = facts::extract(&claim.text, &mut self.remaining)?
            .into_iter()
            .find(|fact| fact.required && !evidence.contains(&fact.key));
        let finding = unsupported
            .map(|fact| Finding::Unsupported(fact.text))
            .or_else(|| (!inherited_flags.is_empty()).then_some(Finding::BuiltOnFlaggedText));
        Ok(GroundedClaim {
            sources,
            finding,
            inherited_flags,
        })
    }
}

impl GroundedClaim {
    fn unresolved(finding: Finding) -> Self {
        Self {
            sources: Vec::new(),
            finding: Some(finding),
            inherited_flags: Vec::new(),
        }
    }
}

fn spend(remaining: &mut usize, amount: usize) -> Result<(), GroundingError> {
    let Some(next) = remaining.checked_sub(amount) else {
        *remaining = 0;
        return Err(GroundingError::CapacityExceeded);
    };
    *remaining = next;
    Ok(())
}

#[cfg(test)]
mod tests;
