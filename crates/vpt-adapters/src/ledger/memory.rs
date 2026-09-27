use std::path::Path;
use std::sync::Mutex;
use vpt_application::ports::{
    DirtyPublication, LedgerCommit, LedgerError, NewIntent, PublicationJournal, RecordingLedger,
    RecordingRecord, RetentionIntent, RetentionJournal, SeenRow,
};
use vpt_domain::digest::Sha256Digest;
use vpt_domain::identity::RecordingId;
use vpt_domain::time::UtcInstant;

#[derive(Clone, Default)]
struct State {
    seen: Vec<SeenRow>,
    recordings: Vec<RecordingRecord>,
    publications: Vec<DirtyPublication>,
    intents: Vec<(RetentionIntent, Option<UtcInstant>)>,
    next_intent_id: i64,
}

#[derive(Default)]
pub struct MemoryLedger {
    state: Mutex<State>,
}

impl MemoryLedger {
    pub fn new() -> Self {
        Self::default()
    }
}

fn with<T>(
    ledger: &MemoryLedger,
    operation: impl FnOnce(&mut State) -> T,
) -> Result<T, LedgerError> {
    let mut state = ledger
        .state
        .lock()
        .map_err(|_| LedgerError::Io("in-memory ledger unavailable".into()))?;
    Ok(operation(&mut state))
}

fn upsert_seen(state: &mut State, row: &SeenRow) {
    match state.seen.iter_mut().find(|stored| stored.path == row.path) {
        Some(stored) => {
            let first_seen = stored.first_seen;
            *stored = row.clone();
            stored.first_seen = first_seen;
        }
        None => state.seen.push(row.clone()),
    }
}

fn upsert_publication(state: &mut State, entry: &DirtyPublication) {
    match state
        .publications
        .iter_mut()
        .find(|stored| stored.target == entry.target)
    {
        Some(stored) => *stored = entry.clone(),
        None => state.publications.push(entry.clone()),
    }
}

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
            let mut next = state.clone();
            for row in &batch.seen {
                upsert_seen(&mut next, row);
            }
            for entry in &batch.publications {
                upsert_publication(&mut next, entry);
            }
            for recording in &batch.recordings {
                if next
                    .recordings
                    .iter()
                    .any(|stored| stored.id == recording.id || stored.digest == recording.digest)
                {
                    return Err(LedgerError::Conflict(
                        "digest or identity already recorded".into(),
                    ));
                }
                next.recordings.push(recording.clone());
            }
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

impl PublicationJournal for MemoryLedger {
    fn pending_publications(&self) -> Result<Vec<DirtyPublication>, LedgerError> {
        with(self, |state| {
            let mut entries = state.publications.clone();
            entries.sort_by(|left, right| {
                (left.recorded_at, left.target.as_os_str())
                    .cmp(&(right.recorded_at, right.target.as_os_str()))
            });
            entries
        })
    }

    fn clear_publication(&self, target: &Path) -> Result<(), LedgerError> {
        with(self, |state| {
            state.publications.retain(|entry| entry.target != target)
        })
    }
}

impl RetentionJournal for MemoryLedger {
    fn record_intent(&self, intent: &NewIntent) -> Result<i64, LedgerError> {
        with(self, |state| {
            let id = state
                .next_intent_id
                .checked_add(1)
                .ok_or_else(|| LedgerError::Conflict("intent identifier exhausted".into()))?;
            state.intents.push((
                RetentionIntent {
                    id,
                    kind: intent.kind,
                    recording: intent.recording.clone(),
                    path: intent.path.clone(),
                    expected: intent.expected,
                    recorded_at: intent.recorded_at,
                },
                None,
            ));
            state.next_intent_id = id;
            Ok(id)
        })?
    }

    fn pending_intents(&self) -> Result<Vec<RetentionIntent>, LedgerError> {
        with(self, |state| {
            state
                .intents
                .iter()
                .filter(|(_, done)| done.is_none())
                .map(|(intent, _)| intent.clone())
                .collect()
        })
    }

    fn complete_intent(&self, id: i64, at: UtcInstant) -> Result<(), LedgerError> {
        with(self, |state| {
            if let Some((_, done)) = state.intents.iter_mut().find(|(intent, _)| intent.id == id)
                && done.is_none()
            {
                *done = Some(at);
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::MemoryLedger;
    use crate::ledger::contract;
    use vpt_application::ports::{LedgerError, RetentionJournal};
    use vpt_domain::time::UtcInstant;

    mod recording_ledger_contract {
        use super::*;

        macro_rules! case {
            ($name:ident) => {
                #[test]
                fn $name() {
                    let ledger = MemoryLedger::new();
                    contract::recording::$name(&ledger);
                }
            };
        }
        contract::recording_cases!(case);
    }

    mod publication_journal_contract {
        use super::*;

        macro_rules! case {
            ($name:ident) => {
                #[test]
                fn $name() {
                    let ledger = MemoryLedger::new();
                    contract::journal::$name(&ledger);
                }
            };
        }
        contract::journal_cases!(case);
    }

    mod retention_journal_contract {
        use super::*;

        macro_rules! case {
            ($name:ident) => {
                #[test]
                fn $name() {
                    let ledger = MemoryLedger::new();
                    contract::retention::$name(&ledger);
                }
            };
        }
        contract::retention_cases!(case);
    }

    #[test]
    fn repeated_completion_keeps_the_first_recorded_time() {
        let ledger = MemoryLedger::new();
        let new = contract::retention::fixture(6);
        let id = ledger.record_intent(&new).expect("recorded");
        ledger
            .complete_intent(id, UtcInstant { secs: 8_000 })
            .expect("first");
        ledger
            .complete_intent(id, UtcInstant { secs: 8_001 })
            .expect("repeat");
        let state = ledger.state.lock().expect("state");
        assert_eq!(state.intents[0].1, Some(UtcInstant { secs: 8_000 }));
    }

    #[test]
    fn intent_id_exhaustion_is_checked_without_inserting_a_row() {
        let ledger = MemoryLedger::new();
        ledger.state.lock().expect("state").next_intent_id = i64::MAX;
        let result = ledger.record_intent(&contract::retention::fixture(7));
        assert!(
            matches!(result, Err(LedgerError::Conflict(_))),
            "{result:?}"
        );
        assert!(ledger.state.lock().expect("state").intents.is_empty());
    }
}
