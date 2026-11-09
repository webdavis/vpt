use super::conflict;
use std::path::Path;
use vpt_application::ports::{LedgerError, NoteArtifact, WorkDelta};
use vpt_domain::{run::HistoryFact, time::UtcInstant};

pub(super) fn check(
    note: &NoteArtifact,
    previous: Option<&NoteArtifact>,
    from: Option<&Path>,
    facts: &[(UtcInstant, HistoryFact)],
) -> Result<(), LedgerError> {
    let moved = previous.filter(|old| old.target != note.target);
    if moved.map(|old| old.target.as_path()) != from
        || moved.is_some_and(|old| old.target.parent() != note.target.parent())
    {
        return Err(conflict());
    }
    let observed: Vec<_> = facts.iter().filter(|(_, fact)| matches!(fact,
        HistoryFact::NoteMoved { recording, to, .. } if recording == &note.recording && to == &note.target
    )).collect();
    match from {
        None if observed.is_empty() => Ok(()),
        Some(from)
            if observed.len() == 1
                && observed[0].0 == note.updated_at
                && matches!(&observed[0].1, HistoryFact::NoteMoved { from: actual, .. } if actual == from) =>
        {
            Ok(())
        }
        _ => Err(conflict()),
    }
}

pub(super) fn check_facts(
    delta: &WorkDelta,
    facts: &[(UtcInstant, HistoryFact)],
) -> Result<(), LedgerError> {
    for (_, fact) in facts {
        if let HistoryFact::NoteMoved {
            recording,
            from,
            to,
        } = fact
            && !delta.notes.iter().any(|change| {
                change.artifact.recording == *recording
                    && change.artifact.target == *to
                    && change.relocated_from.as_ref() == Some(from)
            })
        {
            return Err(conflict());
        }
    }
    Ok(())
}
