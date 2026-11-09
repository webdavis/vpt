use super::invalid;
use serde::{Deserialize, Serialize};
use vpt_application::ports::{LedgerError, ReleaseProvenance};
use vpt_domain::{
    time::{UtcInstant, UtcOffset},
    uncertainty::Diagnostic,
};
pub(crate) const MAX: usize = 65_536;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Saved {
    version: u32,
    captured_at: i64,
    captured_offset: i32,
    engines: Vec<String>,
    open_lexical_flags: u32,
    open_flags: u32,
    diagnostics: Vec<String>,
    reviewed: bool,
}
pub(crate) fn encode(value: &ReleaseProvenance) -> Result<String, LedgerError> {
    super::validate::provenance(value)?;
    crate::ledger::json::encode(
        &Saved {
            version: 1,
            captured_at: value.captured_at.secs,
            captured_offset: value.captured_offset.secs,
            engines: value.engines.clone(),
            open_lexical_flags: value.open_lexical_flags,
            open_flags: value.open_flags,
            diagnostics: value
                .diagnostics
                .iter()
                .map(|d| d.as_str().into())
                .collect(),
            reviewed: value.reviewed,
        },
        MAX,
    )
    .map_err(|_| invalid())
}
pub(crate) fn decode(text: &str) -> Result<ReleaseProvenance, LedgerError> {
    if text.len() > MAX {
        return Err(invalid());
    }
    let raw: Saved = serde_json::from_str(text).map_err(|_| invalid())?;
    if raw.version != 1 {
        return Err(invalid());
    }
    let result = ReleaseProvenance {
        captured_at: UtcInstant {
            secs: raw.captured_at,
        },
        captured_offset: UtcOffset {
            secs: raw.captured_offset,
        },
        engines: raw.engines,
        open_lexical_flags: raw.open_lexical_flags,
        open_flags: raw.open_flags,
        diagnostics: raw
            .diagnostics
            .iter()
            .map(|s| match s.as_str() {
                "single-engine" => Ok(Diagnostic::SingleEngine),
                "language-mismatch" => Ok(Diagnostic::LanguageMismatch),
                "divergent" => Ok(Diagnostic::Divergent),
                _ => Err(invalid()),
            })
            .collect::<Result<_, _>>()?,
        reviewed: raw.reviewed,
    };
    super::validate::provenance(&result)?;
    Ok(result)
}
