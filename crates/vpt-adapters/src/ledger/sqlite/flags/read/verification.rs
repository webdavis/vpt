use super::{corrupt, optional, required};
use rusqlite::Row;
use sha2::{Digest, Sha256};
use vpt_application::ports::LedgerError;
use vpt_domain::{
    digest::Sha256Digest,
    grounding::{Finding, SourceRange},
    note::NoteStage,
    review::{FlagContent, VerificationFlag},
};

pub(super) fn decode(row: &Row<'_>) -> Result<FlagContent, LedgerError> {
    if row.get_ref(6).map_err(|_| corrupt())? != rusqlite::types::ValueRef::Null {
        return Err(corrupt());
    }
    let token = optional(row, 5, 262_144)?;
    let finding = match (required(row, 2, 32)?, token) {
        ("unsourced", None) => Finding::Unsourced,
        ("bad-reference", None) => Finding::BadReference,
        ("built-on-flagged-text", None) => Finding::BuiltOnFlaggedText,
        ("unsupported", Some(token)) => Finding::Unsupported(token.into()),
        _ => return Err(corrupt()),
    };
    let ranges = optional(row, 3, 16_384)?;
    let sources = match &finding {
        Finding::Unsourced | Finding::BadReference if ranges.is_none() => vec![],
        Finding::Unsourced | Finding::BadReference => return Err(corrupt()),
        _ => {
            let ranges: Vec<[u64; 2]> =
                serde_json::from_str(ranges.ok_or_else(corrupt)?).map_err(|_| corrupt())?;
            if ranges.len() > 256 {
                return Err(corrupt());
            }
            ranges
                .into_iter()
                .map(|[start_secs, end_secs]| SourceRange {
                    start_secs,
                    end_secs,
                })
                .collect()
        }
    };
    let flag = VerificationFlag {
        artifact: NoteStage::parse(required(row, 10, 16)?).ok_or_else(corrupt)?,
        claim_index: row.get(11).map_err(|_| corrupt())?,
        claim_digest: Sha256Digest::from_hex(required(row, 12, 64)?).ok_or_else(corrupt)?,
        record_text: required(row, 4, 262_144)?.into(),
        finding,
        sources,
    };
    if !flag.valid()
        || flag.claim_digest.0 != <[u8; 32]>::from(Sha256::digest(flag.record_text.as_bytes()))
    {
        return Err(corrupt());
    }
    Ok(FlagContent::Verification(flag))
}
