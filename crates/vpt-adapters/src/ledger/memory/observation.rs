use super::{MemoryLedger, with};
use crate::ledger::observation::inventory::{Inventory, MAX_RECORDINGS, capacity};
use crate::ledger::observation::{artifacts_by_recording, assemble};
use vpt_application::ports::{LedgerError, ObservationLedger, RecordingObservation};
use vpt_domain::identity::RecordingId;

impl ObservationLedger for MemoryLedger {
    fn observe_recordings(
        &self,
        id: Option<&RecordingId>,
    ) -> Result<Vec<RecordingObservation>, LedgerError> {
        with(self, |state| {
            let mut records: Vec<_> = state
                .recordings
                .iter()
                .filter(|row| id.is_none_or(|id| id == &row.id))
                .take(MAX_RECORDINGS + 1)
                .collect();
            if records.len() > MAX_RECORDINGS {
                return Err(capacity());
            }
            records.sort_by(|a, b| {
                (a.captured_at, a.id.as_str()).cmp(&(b.captured_at, b.id.as_str()))
            });
            if records.is_empty() {
                return Ok(Vec::new());
            }
            let mut artifacts = artifacts_by_recording(super::retention_artifacts::read(state)?);
            let mut observed = Inventory::default();
            for record in records {
                let review = super::verification::state::review(state, &record.id)?;
                let owned = artifacts.remove(record.id.as_str()).unwrap_or_default();
                observed.push(assemble(record.clone(), review, owned))?;
            }
            Ok(observed.finish())
        })?
    }
}

#[cfg(test)]
mod tests;
