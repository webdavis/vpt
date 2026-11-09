use super::super::{State, apply_commit};
use super::state::conflict;
use vpt_application::ports::{HistoryReceipt, LedgerError, WorkDelta};
use vpt_domain::history::RunId;
use vpt_domain::run::HistoryFact;
use vpt_domain::time::UtcInstant;

pub(super) fn apply(
    state: &mut State,
    run: RunId,
    delta: &WorkDelta,
    facts: &[(UtcInstant, HistoryFact)],
) -> Result<HistoryReceipt, LedgerError> {
    if !state.history.has_run(run)
        || facts.is_empty()
        || facts.iter().any(|(_, fact)| {
            matches!(fact, HistoryFact::RunStarted(_)) || !fact.referenced_ids_are_nonzero()
        })
    {
        return Err(conflict());
    }
    if facts
        .iter()
        .any(|(_, fact)| matches!(fact, HistoryFact::RecoveryProgress { .. }))
    {
        let expected = state.history.expected_baseline(run);
        if facts.iter().any(|(_, fact)| {
            matches!(fact,
            HistoryFact::RecoveryProgress { baseline, .. } if *baseline != expected)
        }) {
            return Err(conflict());
        }
    }
    apply_commit(state, &delta.commit)?;
    for (id, path) in &delta.source_updates {
        let recording = state
            .recordings
            .iter_mut()
            .find(|recording| recording.id == *id)
            .ok_or_else(conflict)?;
        recording.source_path = Some(path.clone());
    }
    for target in &delta.publication_clears {
        state.vocabulary.retain(|row| &row.entry.target != target);
        state
            .publications
            .retain(|publication| publication.target != *target);
    }
    for (id, at) in &delta.audio_trash_marks {
        let recording = state
            .recordings
            .iter_mut()
            .find(|recording| recording.id == *id)
            .ok_or_else(conflict)?;
        recording.audio_trashed_at = Some(*at);
    }
    super::super::expirations::regenerated(state, delta);
    let records = facts
        .iter()
        .map(|(at, fact)| state.history.append(run, *at, fact.clone()))
        .collect::<Result<Vec<_>, _>>()?;
    let completed = delta.completed_recordings();
    Ok(HistoryReceipt {
        run,
        records,
        completed,
    })
}
