use super::*;
use std::path::Path;

impl RecordingLedger for MemoryLedger {
    fn seen(&self, path: &Path) -> Result<Option<SeenRow>, LedgerError> {
        with(self, |state| {
            state.seen.iter().find(|row| row.path == path).cloned()
        })
    }

    fn seen_all(&self) -> Result<Vec<SeenRow>, LedgerError> {
        with(self, |state| {
            let mut rows = state.seen.clone();
            rows.sort_by(|left, right| left.path.as_os_str().cmp(right.path.as_os_str()));
            rows
        })
    }

    fn record_seen(&self, row: &SeenRow) -> Result<(), LedgerError> {
        with(self, |state| upsert_seen(state, row))
    }

    fn by_digest(&self, digest: &Sha256Digest) -> Result<Option<RecordingRecord>, LedgerError> {
        with(self, |state| {
            state
                .recordings
                .iter()
                .find(|row| row.digest == *digest)
                .cloned()
        })
    }

    fn by_id(&self, id: &RecordingId) -> Result<Option<RecordingRecord>, LedgerError> {
        with(self, |state| {
            state.recordings.iter().find(|row| row.id == *id).cloned()
        })
    }

    fn recordings(&self) -> Result<Vec<RecordingRecord>, LedgerError> {
        with(self, |state| {
            let mut rows = state.recordings.clone();
            rows.sort_by(|left, right| {
                (left.captured_at, left.id.as_str()).cmp(&(right.captured_at, right.id.as_str()))
            });
            rows
        })
    }

    fn commit(&self, batch: &LedgerCommit) -> Result<(), LedgerError> {
        with(self, |state| {
            crate::ledger::brief::reservations::commit(batch, |path| {
                Ok(state.briefs.iter().any(|row| row.target == path))
            })?;
            let mut next = state.clone();
            apply_commit(&mut next, batch)?;
            *state = next;
            Ok(())
        })?
    }

    fn set_source_path(&self, id: &RecordingId, path: &Path) -> Result<(), LedgerError> {
        with(self, |state| {
            if let Some(row) = state.recordings.iter_mut().find(|row| row.id == *id) {
                row.source_path = Some(path.to_path_buf());
            }
        })
    }

    fn set_audio_trashed(&self, id: &RecordingId, at: UtcInstant) -> Result<(), LedgerError> {
        with(self, |state| {
            if let Some(row) = state.recordings.iter_mut().find(|row| row.id == *id) {
                row.audio_trashed_at = Some(at);
            }
        })
    }
}
