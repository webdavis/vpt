pub(super) mod codec;
mod report;
pub(super) mod validate;
use vpt_application::ports::{LedgerError, ReleaseRecord};
use vpt_domain::run::{EffectKind, EffectReference, HistoryFact};
pub(super) fn invalid() -> LedgerError {
    LedgerError::Corrupt("invalid saved release".into())
}
pub(super) fn conflict() -> LedgerError {
    LedgerError::Conflict("release state mismatch".into())
}
pub(super) fn intents(row: &ReleaseRecord) -> [HistoryFact; 2] {
    [&row.destination, &row.report_path].map(|path| HistoryFact::EffectIntent {
        kind: EffectKind::Publication,
        reference: EffectReference::PublicationTarget(path.clone()),
    })
}
