use vpt_application::ports::{LedgerError, VocabularyPublication, WorkDelta};
use vpt_domain::{
    run::{EffectKind, EffectReference, HistoryFact},
    time::UtcInstant,
};

pub(super) fn invalid() -> LedgerError {
    LedgerError::Corrupt("invalid vocabulary publication".into())
}
pub(super) fn conflict() -> LedgerError {
    LedgerError::Conflict("vocabulary publication conflict".into())
}
pub(super) fn validate(value: &VocabularyPublication) -> Result<(), LedgerError> {
    crate::known_terms::validation::validate(value).map_err(|_| invalid())
}

pub(super) fn check(
    delta: &WorkDelta,
    facts: &[(UtcInstant, HistoryFact)],
) -> Result<(), LedgerError> {
    let Some(value) = &delta.vocabulary else {
        return Ok(());
    };
    validate(value)?;
    let target = &value.entry.target;
    if value.entry.expected_previous == Some(value.entry.intended)
        || delta.publication_clears.contains(target)
        || delta
            .commit
            .publications
            .iter()
            .any(|row| &row.target == target)
        || delta.notes.iter().any(|row| &row.artifact.target == target)
        || delta.engine_outputs.iter().any(|row| &row.target == target)
        || delta
            .commit
            .recordings
            .iter()
            .any(|row| &row.audio_path == target)
    {
        return Err(conflict());
    }
    let intents: Vec<_> = facts.iter().filter(|(_, fact)| matches!(fact,
        HistoryFact::EffectIntent { reference: EffectReference::PublicationTarget(path), .. } if path == target)).collect();
    if intents.len() != 1
        || intents[0].0 != value.entry.recorded_at
        || !matches!(
            intents[0].1,
            HistoryFact::EffectIntent {
                kind: EffectKind::Publication,
                ..
            }
        )
    {
        return Err(conflict());
    }
    Ok(())
}
