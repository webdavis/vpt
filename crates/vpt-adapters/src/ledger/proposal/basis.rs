use super::invalid;
use sha2::{Digest, Sha256};
use vpt_application::ports::{AcceptedTranscript, GroundingBasis, LedgerError};
use vpt_domain::{
    digest::Sha256Digest,
    review::{FlagContent, Resolution, ReviewFlag},
};

pub fn compute(
    accepted: &AcceptedTranscript,
    flags: &[ReviewFlag],
) -> Result<GroundingBasis, LedgerError> {
    check_limits(flags)?;
    let transcript = super::super::transcript::encode(accepted)?;
    let mut lexical: Vec<_> = flags
        .iter()
        .filter(|flag| matches!(flag.content, FlagContent::Lexical(_)))
        .collect();
    lexical.sort_by_key(|flag| flag.identifier.0);
    if lexical
        .windows(2)
        .any(|pair| pair[0].identifier == pair[1].identifier)
    {
        return Err(invalid());
    }
    let mut hash = Sha256::new();
    field(&mut hash, b"vpt.grounding-flags/1");
    for flag in lexical {
        let FlagContent::Lexical(content) = &flag.content else {
            unreachable!()
        };
        if flag.identifier != crate::lexical_flag_id(&accepted.recording, content)
            || !flag.resolution.valid_text()
        {
            return Err(invalid());
        }
        hash.update(flag.identifier.0);
        hash.update([u8::from(content.confidence.is_some())]);
        if let Some(confidence) = content.confidence {
            hash.update(confidence.to_bits().to_le_bytes());
        }
        match &flag.resolution {
            Resolution::Open => field(&mut hash, b"open"),
            Resolution::Confirmed { .. } => field(&mut hash, b"confirmed"),
            Resolution::Dismissed { .. } => field(&mut hash, b"dismissed"),
            Resolution::Corrected { text, .. } => {
                field(&mut hash, b"corrected");
                field(&mut hash, text.as_bytes());
            }
        }
        if let Some(at) = flag.resolution.at() {
            hash.update(at.secs.to_le_bytes());
        }
    }
    Ok(GroundingBasis {
        transcript: Sha256Digest(Sha256::digest(transcript.as_bytes()).into()),
        lexical_flags: Sha256Digest(hash.finalize().into()),
    })
}

fn field(hash: &mut Sha256, bytes: &[u8]) {
    hash.update((bytes.len() as u64).to_le_bytes());
    hash.update(bytes);
}

fn check_limits(flags: &[ReviewFlag]) -> Result<(), LedgerError> {
    if flags.len() > 65_539 {
        return Err(invalid());
    }
    let mut bytes = 0usize;
    let mut ranges = 0usize;
    for flag in flags {
        if let FlagContent::Lexical(content) = &flag.content {
            bytes = bytes
                .saturating_add(content.record_text.len())
                .saturating_add(content.alternative_text.as_ref().map_or(0, String::len));
            ranges = ranges.saturating_add(content.ranges.len());
            if content
                .confidence
                .is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
            {
                return Err(invalid());
            }
        }
        if let Resolution::Corrected { text, .. } = &flag.resolution {
            bytes = bytes.saturating_add(text.len());
        }
        if bytes > 48 * 1024 * 1024 || ranges > 65_536 {
            return Err(invalid());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
