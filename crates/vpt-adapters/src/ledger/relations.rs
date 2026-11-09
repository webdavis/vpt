pub(super) mod ambiguities;
pub(super) mod codec;
mod note;
pub(super) mod ranges;
mod validate;
pub(super) use note::resolve;
use vpt_application::ports::{LedgerError, RelationChange};
use vpt_domain::{
    relations::Relation,
    run::{HistoryFact, WorkOutcome},
    time::UtcInstant,
};
pub(super) const MAX_BYTES: usize = 16 * 1024 * 1024;
pub(super) const MAX_ITEMS: usize = 65_536;
pub(super) fn invalid() -> LedgerError {
    LedgerError::Corrupt("invalid saved relations".into())
}
pub(super) fn conflict() -> LedgerError {
    LedgerError::Conflict("relation state conflict".into())
}

pub(super) fn ordered(mut relations: Vec<Relation>) -> Result<Vec<Relation>, LedgerError> {
    validate::relations(&relations)?;
    relations.sort_by(|a, b| (a.kind, a.target.as_str()).cmp(&(b.kind, b.target.as_str())));
    Ok(relations)
}

pub(super) fn check(
    change: &RelationChange,
    current: &[Relation],
    facts: &[(UtcInstant, HistoryFact)],
) -> Result<Vec<Relation>, LedgerError> {
    validate::relations(&change.expected)?;
    validate::relations(&change.relations)?;
    if ordered(change.expected.clone())? != current || facts.iter().filter(|(at, fact)|
        *at == change.at && matches!(fact, HistoryFact::WorkStep { recording: Some(id), outcome: WorkOutcome::Completed } if *id == change.recording)
    ).count() != 1 { return Err(conflict()); }
    let relations = ordered(change.relations.clone())?;
    if relations
        .iter()
        .any(|relation| relation.target.as_str() == change.recording.as_str())
    {
        return Err(invalid());
    }
    Ok(relations)
}
