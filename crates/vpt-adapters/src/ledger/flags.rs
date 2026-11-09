pub(super) mod budget;
pub(super) mod resolution;
mod validate;
use crate::{diagnostic_flag_id, lexical_flag_id};
use vpt_application::ports::{AcceptedTranscript, LedgerError};
use vpt_domain::{
    review::{FlagContent, Resolution, ReviewFlag},
    uncertainty::{Classification, Diagnostic},
};

pub(super) fn prepare(accepted: &AcceptedTranscript) -> Result<Vec<ReviewFlag>, LedgerError> {
    validate::classification(accepted)?;
    Ok(accepted
        .uncertainty
        .lexical
        .iter()
        .map(|flag| ReviewFlag {
            identifier: lexical_flag_id(&accepted.recording, flag),
            content: FlagContent::Lexical(flag.clone()),
            resolution: Resolution::Open,
        })
        .chain(
            accepted
                .uncertainty
                .diagnostics
                .iter()
                .map(|diagnostic| ReviewFlag {
                    identifier: diagnostic_flag_id(&accepted.recording, *diagnostic),
                    content: FlagContent::Diagnostic(*diagnostic),
                    resolution: Resolution::Open,
                }),
        )
        .collect())
}

pub(super) fn classification(flags: &[ReviewFlag]) -> Classification {
    let mut result = Classification::default();
    for flag in flags {
        match &flag.content {
            FlagContent::Lexical(flag) => result.lexical.push(flag.clone()),
            FlagContent::Diagnostic(diagnostic) => result.diagnostics.push(*diagnostic),
            FlagContent::Verification(_) => {}
        }
    }
    result
        .lexical
        .sort_by_key(|flag| (flag.class, flag.ranges.first().map(|range| range.start)));
    result
        .diagnostics
        .sort_by_key(|diagnostic| match diagnostic {
            Diagnostic::Divergent => 0,
            Diagnostic::SingleEngine => 1,
            Diagnostic::LanguageMismatch => 2,
        });
    result
}

pub(super) fn corrupt() -> LedgerError {
    LedgerError::Corrupt("invalid transcript flags".into())
}
