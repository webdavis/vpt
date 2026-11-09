pub(super) mod resolve;
use super::{MemoryLedger, State, with};
use vpt_application::ports::{
    AcceptedTranscript, FlagLedger, LedgerError, ReviewLedger, ReviewSnapshot,
};
use vpt_domain::{
    identity::RecordingId,
    review::{FlagContent, ReviewFlag},
};

impl FlagLedger for MemoryLedger {
    fn transcript_flags(&self, id: &RecordingId) -> Result<Vec<ReviewFlag>, LedgerError> {
        with(self, |state| {
            state
                .flags
                .iter()
                .filter(|(recording, flag)| {
                    recording == id && !matches!(flag.content, FlagContent::Verification(_))
                })
                .map(|(_, flag)| flag.clone())
                .collect()
        })
    }
}

impl ReviewLedger for MemoryLedger {
    fn review_snapshot(&self, id: &RecordingId) -> Result<Option<ReviewSnapshot>, LedgerError> {
        with(self, |state| {
            state
                .recordings
                .iter()
                .any(|record| record.id == *id)
                .then(|| super::verification::state::review(state, id))
                .transpose()
        })?
    }
}

pub(super) fn replace(state: &mut State, accepted: &AcceptedTranscript) -> Result<(), LedgerError> {
    let mut flags = crate::ledger::flags::prepare(accepted)?;
    let previous: std::collections::HashMap<_, _> = state
        .flags
        .iter()
        .filter(|(recording, _)| recording == &accepted.recording)
        .map(|(_, flag)| (flag.identifier, flag.resolution.clone()))
        .collect();
    for flag in &mut flags {
        if let Some(resolution) = previous.get(&flag.identifier) {
            flag.resolution = resolution.clone()
        }
    }
    state.flags.retain(|(recording, flag)| {
        recording != &accepted.recording || matches!(flag.content, FlagContent::Verification(_))
    });
    state.flags.extend(
        flags
            .into_iter()
            .map(|flag| (accepted.recording.clone(), flag)),
    );
    crate::ledger::flags::budget::check(
        state
            .flags
            .iter()
            .filter(|(id, _)| id == &accepted.recording)
            .map(|(_, flag)| flag),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests;
