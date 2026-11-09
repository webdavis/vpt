pub(super) mod codec;
pub(super) mod list;
pub(super) mod validate;
use sha2::{Digest, Sha256};
use vpt_application::ports::{LedgerError, OccasionChange, OccasionLedger, OccasionRecord};
use vpt_domain::{
    digest::Sha256Digest,
    occasion::{OccasionDetails, OccasionId, OccasionKey},
    time::UtcInstant,
};

pub fn prepare(
    ledger: &impl OccasionLedger,
    key: OccasionKey,
    details: OccasionDetails,
    at: UtcInstant,
) -> Result<OccasionChange, LedgerError> {
    let expected = ledger.occasion_by_key(&key)?;
    let id = expected
        .as_ref()
        .map(|row| row.id.clone())
        .unwrap_or_else(|| OccasionId::derive(&details.at, &digest(&key)));
    let occasion = OccasionRecord {
        id,
        key,
        details,
        brief_path: expected.as_ref().and_then(|row| row.brief_path.clone()),
        briefed_at: expected.as_ref().and_then(|row| row.briefed_at),
    };
    validate::validate(&occasion)?;
    Ok(OccasionChange {
        expected,
        occasion,
        at,
    })
}

pub(super) fn digest(key: &OccasionKey) -> Sha256Digest {
    Sha256Digest(Sha256::digest(key.encoded().as_bytes()).into())
}
