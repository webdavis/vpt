use super::*;
use vpt_application::ports::{BriefArtifact, BriefChange, OccasionRecord, WorkDelta};
use vpt_domain::{run::HistoryFact, time::UtcInstant};

pub(crate) fn check(
    delta: &WorkDelta,
    change: &BriefChange,
    current: &OccasionRecord,
    old: Option<&BriefArtifact>,
    facts: &[(UtcInstant, HistoryFact)],
) -> Result<String, LedgerError> {
    let artifact = &change.artifact;
    let encoded = validate::validate(artifact)?;
    if &change.expected_occasion != current
        || artifact.snapshot.id != current.id
        || artifact.snapshot.occasion != current.details
        || artifact.snapshot.source != current.key.source()
        || change.expected != old.map(|row| row.version)
        || artifact.published_at.is_some()
        || delta.brief_published.is_some()
        || delta.publication_clears.contains(&artifact.target)
        || current
            .brief_path
            .as_ref()
            .is_some_and(|path| *path != artifact.target)
        || (old.is_none() && current.brief_path.is_some())
        || old.is_some_and(|row| {
            row.target != artifact.target
                || row.first_written != artifact.first_written
                || row.write_offset != artifact.write_offset
        })
        || delta
            .notes
            .iter()
            .any(|row| row.artifact.target == artifact.target)
        || delta
            .engine_outputs
            .iter()
            .any(|row| row.target == artifact.target)
        || delta
            .vocabulary
            .as_ref()
            .is_some_and(|row| row.entry.target == artifact.target)
    {
        return Err(conflict());
    }
    let matching: Vec<_> = facts
        .iter()
        .filter(|(_, fact)| matches!(fact, HistoryFact::BriefSaved { .. }))
        .collect();
    if matching.as_slice()
        != [&(
            artifact.snapshot.generated_at,
            HistoryFact::BriefSaved {
                occasion: current.id.clone(),
            },
        )]
    {
        return Err(conflict());
    }
    let entries: Vec<_> = delta
        .commit
        .publications
        .iter()
        .filter(|entry| entry.target == artifact.target)
        .collect();
    if entries.len() != 1
        || entries[0].intended != artifact.version.content
        || entries[0].recorded_at != artifact.snapshot.generated_at
    {
        return Err(conflict());
    }
    Ok(encoded)
}
