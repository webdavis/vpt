use sha2::{Digest, Sha256};
use vpt_domain::{
    digest::Sha256Digest,
    identity::RecordingId,
    review::VerificationFlag,
    uncertainty::{Diagnostic, LexicalFlag},
};

pub fn lexical_flag_id(recording: &RecordingId, flag: &LexicalFlag) -> Sha256Digest {
    let mut hash = Sha256::new();
    field(&mut hash, b"vpt.lexical-flag/1");
    field(&mut hash, recording.as_str().as_bytes());
    field(&mut hash, flag.class.as_str().as_bytes());
    hash.update((flag.ranges.len() as u64).to_le_bytes());
    for range in &flag.ranges {
        hash.update((range.start as u64).to_le_bytes());
        hash.update((range.end as u64).to_le_bytes());
    }
    field(&mut hash, flag.record_text.as_bytes());
    hash.update([u8::from(flag.alternative_text.is_some())]);
    if let Some(alternative) = &flag.alternative_text {
        field(&mut hash, alternative.as_bytes());
    }
    Sha256Digest(hash.finalize().into())
}

pub fn diagnostic_flag_id(recording: &RecordingId, diagnostic: Diagnostic) -> Sha256Digest {
    let mut hash = Sha256::new();
    field(&mut hash, b"vpt.diagnostic-flag/1");
    field(&mut hash, recording.as_str().as_bytes());
    field(&mut hash, diagnostic.as_str().as_bytes());
    Sha256Digest(hash.finalize().into())
}

pub fn verification_flag_id(recording: &RecordingId, flag: &VerificationFlag) -> Sha256Digest {
    let mut hash = Sha256::new();
    field(&mut hash, b"vpt.verification-flag/1");
    field(&mut hash, recording.as_str().as_bytes());
    field(&mut hash, flag.artifact.as_str().as_bytes());
    hash.update(u64::from(flag.claim_index).to_le_bytes());
    hash.update(flag.claim_digest.0);
    Sha256Digest(hash.finalize().into())
}

fn field(hash: &mut Sha256, bytes: &[u8]) {
    hash.update((bytes.len() as u64).to_le_bytes());
    hash.update(bytes);
}

#[cfg(test)]
mod tests;
