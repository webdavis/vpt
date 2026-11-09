use super::State;
use crate::ledger::retention_artifacts::{MAX_ARTIFACTS, invalid};
use vpt_application::ports::*;

pub(super) fn overlay(state: &State, rows: &mut [RetentionArtifact]) -> Result<(), LedgerError> {
    if state.expirations.len() > MAX_ARTIFACTS {
        return Err(invalid());
    }
    for marker in &state.expirations {
        let index = rows
            .binary_search_by(|row| row.path.cmp(&marker.path))
            .map_err(|_| invalid())?;
        let row = &mut rows[index];
        if row.kind != marker.kind || row.owner != marker.owner || row.digest != marker.digest {
            return Err(invalid());
        }
        row.trashed_at = marker.trashed_at;
        if row.status != RetentionStatus::PublicationPending {
            row.status = RetentionStatus::Expired;
        }
    }
    Ok(())
}

pub(super) fn regenerated(state: &mut State, delta: &WorkDelta) {
    let paths: std::collections::HashSet<_> =
        crate::ledger::retention_artifacts::completion::regenerated(delta).collect();
    state
        .expirations
        .retain(|row| !paths.contains(row.path.as_path()));
}
