use super::*;
use vpt_application::ports::{BriefArtifact, BriefPublication, WorkDelta};
use vpt_domain::{run::HistoryFact, time::UtcInstant};

pub(crate) fn check(
    delta: &WorkDelta,
    change: &BriefPublication,
    artifact: &BriefArtifact,
    pending: bool,
    facts: &[(UtcInstant, HistoryFact)],
) -> Result<(), LedgerError> {
    if delta.brief.is_some()
        || artifact.snapshot.id != change.occasion
        || artifact.version != change.expected
        || artifact.published_at.is_some()
        || !validate::instant(change.at)
        || delta
            .publication_clears
            .iter()
            .filter(|path| **path == artifact.target)
            .count()
            != 1
        || !pending
    {
        return Err(conflict());
    }
    let matching: Vec<_> = facts
        .iter()
        .filter(|(_, fact)| matches!(fact, HistoryFact::BriefPublished { .. }))
        .collect();
    if matching.as_slice()
        != [&(
            change.at,
            HistoryFact::BriefPublished {
                occasion: change.occasion.clone(),
            },
        )]
    {
        return Err(conflict());
    }
    Ok(())
}
