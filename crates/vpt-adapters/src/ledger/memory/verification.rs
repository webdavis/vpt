pub(super) mod state;
use super::State;
use crate::ledger::verification::{conflict, prepare};
use vpt_application::ports::{LedgerError, WorkDelta};
use vpt_domain::{review::FlagContent, run::HistoryFact, time::UtcInstant};

pub(super) fn apply(
    state: &mut State,
    delta: &WorkDelta,
    facts: &[(UtcInstant, HistoryFact)],
) -> Result<(), LedgerError> {
    let Some(change) = &delta.verification else {
        return Ok(());
    };
    let note = state
        .notes
        .iter()
        .find(|note| note.recording == change.recording && note.stage == change.artifact)
        .ok_or_else(conflict)?;
    if state
        .publications
        .iter()
        .any(|entry| entry.target == note.target)
        || state
            .intents
            .iter()
            .any(|(intent, done)| done.is_none() && intent.path == note.target)
    {
        return Err(conflict());
    }
    let transcript = state
        .transcripts
        .iter()
        .find(|value| value.recording == change.recording)
        .ok_or_else(conflict)?;
    let flags: Vec<_> = state
        .flags
        .iter()
        .filter(|(id, _)| id == &change.recording)
        .map(|(_, flag)| flag.clone())
        .collect();
    let (replacement, verified) = prepare(change, delta, facts, note, transcript, &flags)?;
    replace(state, &change.recording, change.artifact, replacement);
    self::state::save(state, verified);
    Ok(())
}

pub(super) fn replace(
    state: &mut State,
    recording: &vpt_domain::identity::RecordingId,
    artifact: vpt_application::ports::NoteStage,
    replacement: Vec<vpt_domain::review::ReviewFlag>,
) {
    state.flags.retain(|(id, flag)| id != recording || !matches!(&flag.content, FlagContent::Verification(finding) if finding.artifact == artifact));
    state.flags.extend(
        replacement
            .into_iter()
            .map(|flag| (recording.clone(), flag)),
    );
}
