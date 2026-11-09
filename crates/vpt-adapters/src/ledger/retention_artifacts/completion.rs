use vpt_application::ports::*;
use vpt_domain::run::{EffectKind, EffectReference, HistoryFact};

pub(in crate::ledger) fn conflict() -> LedgerError {
    LedgerError::Conflict("retention artifact state mismatch".into())
}

pub(in crate::ledger) fn binding(
    rows: Vec<RetentionArtifact>,
    intent: &RetentionIntent,
) -> Result<RetentionArtifact, LedgerError> {
    let row = rows
        .into_iter()
        .find(|row| row.path == intent.path)
        .ok_or_else(conflict)?;
    if !row.matches_intent(intent) {
        return Err(conflict());
    }
    Ok(row)
}

pub(in crate::ledger) fn fact(intent: &RetentionIntent) -> Result<HistoryFact, LedgerError> {
    Ok(HistoryFact::EffectIntent {
        kind: EffectKind::RetentionTrash,
        reference: EffectReference::RetentionIntent {
            id: u64::try_from(intent.id).map_err(|_| conflict())?,
            path: intent.path.clone(),
        },
    })
}

pub(in crate::ledger) fn regenerated(delta: &WorkDelta) -> impl Iterator<Item = &std::path::Path> {
    delta
        .notes
        .iter()
        .flat_map(|change| {
            std::iter::once(change.artifact.target.as_path())
                .chain(change.relocated_from.as_deref())
        })
        .chain(
            delta
                .brief
                .iter()
                .map(|change| change.artifact.target.as_path()),
        )
        .chain(delta.engine_outputs.iter().map(|row| row.target.as_path()))
}
