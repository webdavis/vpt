use super::{conflict, validate};
use vpt_application::ports::{LedgerError, NoteArtifact, NoteChange, WorkDelta};
use vpt_domain::{
    run::{HistoryFact, WorkOutcome},
    time::UtcInstant,
};

pub(in crate::ledger) fn check(
    delta: &WorkDelta,
    facts: &[(UtcInstant, HistoryFact)],
    previous: impl Fn(&NoteArtifact) -> Result<Option<NoteArtifact>, LedgerError>,
) -> Result<(), LedgerError> {
    if delta.notes.len() > 2 {
        return Err(conflict());
    }
    super::relocation::check_facts(delta, facts)?;
    let mut targets = std::collections::HashSet::new();
    let mut identities = std::collections::BTreeSet::new();
    for NoteChange {
        artifact: note,
        expected_saved,
        relocated_from,
    } in &delta.notes
    {
        validate(note)?;
        if !targets.insert(&note.target)
            || !identities.insert((note.recording.as_str(), note.stage))
        {
            return Err(conflict());
        }
        let old = previous(note)?;
        if old.as_ref().map(|old| old.digest) != *expected_saved {
            return Err(conflict());
        }
        super::relocation::check(note, old.as_ref(), relocated_from.as_deref(), facts)?;
        if let Some(old) = &old
            && (old.first_written != note.first_written || old.write_offset != note.write_offset)
        {
            return Err(conflict());
        }
        let entries: Vec<_> = delta
            .commit
            .publications
            .iter()
            .filter(|row| row.target == note.target)
            .collect();
        if entries.len() != 1
            || entries[0].intended != note.digest
            || entries[0].recorded_at != note.updated_at
            || (old.is_none() && entries[0].expected_previous.is_some())
            || delta.publication_clears.contains(&note.target)
        {
            return Err(conflict());
        }
        let matched: Vec<_> = facts.iter().filter(|(_, fact)| matches!(fact, HistoryFact::WorkStep { recording: Some(id), .. } if id == &note.recording)).collect();
        if matched.is_empty()
            || matched.iter().any(|(at, fact)| {
                *at != note.updated_at
                    || !matches!(
                        fact,
                        HistoryFact::WorkStep {
                            outcome: WorkOutcome::Completed,
                            ..
                        }
                    )
            })
        {
            return Err(conflict());
        }
    }
    Ok(())
}
