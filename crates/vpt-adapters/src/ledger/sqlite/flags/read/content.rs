use super::{corrupt, optional, required};
use rusqlite::Row;
use vpt_application::ports::LedgerError;
use vpt_domain::{
    review::FlagContent,
    uncertainty::{Class, Diagnostic, LexicalFlag},
};

pub(super) fn decode(row: &Row<'_>) -> Result<FlagContent, LedgerError> {
    let shape = required(row, 1, 16)?;
    let class = required(row, 2, 32)?;
    let confidence: Option<f64> = row.get(6).map_err(|_| corrupt())?;
    let ranges = optional(row, 3, 2_097_152)?;
    let record = optional(row, 4, 16_777_216)?;
    let alternative = optional(row, 5, 16_777_216)?;
    match shape {
        "lexical" => {
            let ranges: Vec<std::ops::Range<usize>> =
                serde_json::from_str(ranges.ok_or_else(corrupt)?).map_err(|_| corrupt())?;
            if ranges.is_empty() || ranges.len() > 65_536 {
                return Err(corrupt());
            }
            Ok(FlagContent::Lexical(LexicalFlag {
                class: match class {
                    "formatting" => Class::Formatting,
                    "numeric" => Class::Numeric,
                    "proper-noun" => Class::ProperNoun,
                    "low-confidence" => Class::LowConfidence,
                    "agreed-unverified" => Class::AgreedUnverified,
                    "other" => Class::Other,
                    _ => return Err(corrupt()),
                },
                ranges,
                record_text: record.ok_or_else(corrupt)?.into(),
                alternative_text: alternative.map(str::to_owned),
                confidence,
            }))
        }
        "diagnostic"
            if ranges.is_none()
                && record.is_none()
                && alternative.is_none()
                && confidence.is_none() =>
        {
            Ok(FlagContent::Diagnostic(match class {
                "single-engine" => Diagnostic::SingleEngine,
                "language-mismatch" => Diagnostic::LanguageMismatch,
                "divergent" => Diagnostic::Divergent,
                _ => return Err(corrupt()),
            }))
        }
        _ => Err(corrupt()),
    }
}
