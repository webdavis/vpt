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
    super::super::occasions::apply(state, delta, facts)?;
    super::super::briefs::apply(state, delta, facts)?;
    super::super::eligibility::apply(state, delta, facts)?;
    super::super::verification::apply(state, delta, facts)?;
    super::super::proposals::apply(state, delta, facts)?;
    super::super::engine_outputs::check(state, delta)?;
    super::super::vocabulary::apply(state, delta, facts)?;
    super::super::notes::apply(state, delta, facts)?;
    super::super::tags::apply(state, delta, facts)?;
    super::super::tags::apply_rename(state, delta)?;
    super::super::tags::apply_learned(state, delta)?;
    super::super::actions::apply(state, delta)?;
    super::super::relations::apply(state, delta, facts)?;
    apply_commit(state, &delta.commit)?;
    super::super::engine_outputs::apply(state, delta);
    for (id, path) in &delta.source_updates {
        let recording = state
            .recordings
            .iter_mut()
            .find(|recording| recording.id == *id)
            .ok_or_else(conflict)?;
        recording.source_path = Some(path.clone());
    }
    for target in &delta.publication_clears {
        if super::super::releases::owns(state, target) {
            return Err(crate::ledger::release::conflict());
        }
        state.vocabulary.retain(|row| &row.entry.target != target);
        super::super::notes::published(state, target)?;
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
    if let Some(change) = &delta.transcription {
        crate::ledger::transcript::check_history(change, facts)?;
        super::super::transcript::apply(state, change)?;
    }
    super::super::expirations::regenerated(state, delta);
    super::super::flags::resolve::apply(state, delta, facts)?;
    super::super::verification::state::refresh(state, delta)?;
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
