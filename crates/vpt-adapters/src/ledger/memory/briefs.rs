use super::{MemoryLedger, State, with};
use crate::ledger::brief::{self as validation, conflict};
use std::path::Path;
use vpt_application::ports::{BriefArtifact, BriefLedger, LedgerError, WorkDelta};
use vpt_domain::{occasion::OccasionId, run::HistoryFact, time::UtcInstant};

impl BriefLedger for MemoryLedger {
    fn brief(&self, id: &OccasionId) -> Result<Option<BriefArtifact>, LedgerError> {
        with(self, |state| {
            state
                .briefs
                .iter()
                .find(|row| row.snapshot.id == *id)
                .cloned()
        })
    }
    fn brief_at(&self, target: &Path) -> Result<Option<BriefArtifact>, LedgerError> {
        with(self, |state| {
            state
                .briefs
                .iter()
                .find(|row| row.target == target)
                .cloned()
        })
    }
}

pub(super) fn apply(
    state: &mut State,
    delta: &WorkDelta,
    facts: &[(UtcInstant, HistoryFact)],
) -> Result<(), LedgerError> {
    validation::check_facts(delta, facts)?;
    validation::reservations::work(delta, |path| {
        Ok(state.briefs.iter().any(|row| row.target == path))
    })?;
    for path in &delta.publication_clears {
        if let Some(artifact) = state.briefs.iter().find(|row| row.target == *path)
            && delta
                .brief_published
                .as_ref()
                .is_none_or(|change| change.occasion != artifact.snapshot.id)
        {
            return Err(conflict());
        }
    }
    if let Some(change) = &delta.brief {
        let artifact = &change.artifact;
        let current = state
            .occasions
            .iter()
            .find(|row| row.id == artifact.snapshot.id)
            .ok_or_else(conflict)?;
        let old = state
            .briefs
            .iter()
            .find(|row| row.snapshot.id == artifact.snapshot.id);
        validation::change::check(delta, change, current, old, facts)?;
        if state
            .publications
            .iter()
            .any(|row| row.target == artifact.target)
            || state
                .intents
                .iter()
                .any(|(row, done)| done.is_none() && row.path == artifact.target)
            || state.notes.iter().any(|row| row.target == artifact.target)
            || state
                .engine_outputs
                .iter()
                .any(|row| row.target == artifact.target)
            || state
                .briefs
                .iter()
                .any(|row| row.target == artifact.target && row.snapshot.id != artifact.snapshot.id)
            || artifact.snapshot.pack.sources.iter().any(|source| {
                !state.notes.iter().any(|row| {
                    row.recording == source.recording
                        && row.stage == vpt_application::ports::NoteStage::Transcript
                        && row.target == source.note
                })
            })
        {
            return Err(conflict());
        }
        let row = state
            .occasions
            .iter_mut()
            .find(|row| row.id == artifact.snapshot.id)
            .ok_or_else(conflict)?;
        row.brief_path = Some(artifact.target.clone());
        if let Some(old) = state
            .briefs
            .iter_mut()
            .find(|row| row.snapshot.id == artifact.snapshot.id)
        {
            *old = artifact.clone();
        } else {
            state.briefs.push(artifact.clone());
        }
    }
    if let Some(change) = &delta.brief_published {
        let artifact = state
            .briefs
            .iter()
            .find(|row| row.snapshot.id == change.occasion)
            .ok_or_else(conflict)?;
        let entry = state
            .publications
            .iter()
            .find(|row| row.target == artifact.target);
        validation::published::check(
            delta,
            change,
            artifact,
            entry.is_some_and(|entry| entry.intended == artifact.version.content),
            facts,
        )?;
        state
            .occasions
            .iter_mut()
            .find(|row| row.id == change.occasion)
            .ok_or_else(conflict)?
            .briefed_at = Some(change.at);
        state
            .briefs
            .iter_mut()
            .find(|row| row.snapshot.id == change.occasion)
            .ok_or_else(conflict)?
            .published_at = Some(change.at);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    macro_rules! case {
        ($name:ident) => {
            #[test]
            fn $name() {
                crate::ledger::contract::briefs::$name(&super::MemoryLedger::new());
            }
        };
    }
    crate::ledger::contract::briefs::cases!(case);
}
