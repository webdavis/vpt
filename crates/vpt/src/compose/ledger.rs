mod observation;
mod occasions;
mod retention_artifacts;
use std::path::Path;
use vpt_adapters::{MemoryLedger, SqliteLedger};
use vpt_application::ports::{
    LedgerCommit, LedgerError, NewIntent, RecordingLedger, RecordingRecord, RetentionIntent,
    RetentionJournal, SeenRow,
};
use vpt_domain::digest::Sha256Digest;
use vpt_domain::identity::RecordingId;
use vpt_domain::time::UtcInstant;
use vpt_protocol::error::ErrorDocument;
mod notes;
mod proposals;
mod review;

pub(crate) enum RuntimeLedger {
    Sqlite(SqliteLedger),
    Empty(MemoryLedger),
}

impl RuntimeLedger {
    pub(crate) fn read_only(path: &Path) -> Result<Self, Box<ErrorDocument>> {
        match SqliteLedger::open_read_only(path)
            .map_err(|error| Box::new(super::errors::open_error(error)))?
        {
            Some(ledger) => Ok(Self::Sqlite(ledger)),
            None => Ok(Self::Empty(MemoryLedger::new())),
        }
    }

    fn port(&self) -> &dyn RecordingLedger {
        match self {
            Self::Sqlite(ledger) => ledger,
            Self::Empty(ledger) => ledger,
        }
    }
}

impl RecordingLedger for RuntimeLedger {
    fn seen(&self, path: &Path) -> Result<Option<SeenRow>, LedgerError> {
        self.port().seen(path)
    }
    fn seen_all(&self) -> Result<Vec<SeenRow>, LedgerError> {
        self.port().seen_all()
    }
    fn record_seen(&self, _row: &SeenRow) -> Result<(), LedgerError> {
        Err(LedgerError::Io("read-only ledger".into()))
    }
    fn by_digest(&self, digest: &Sha256Digest) -> Result<Option<RecordingRecord>, LedgerError> {
        self.port().by_digest(digest)
    }
    fn by_id(&self, id: &RecordingId) -> Result<Option<RecordingRecord>, LedgerError> {
        self.port().by_id(id)
    }
    fn recordings(&self) -> Result<Vec<RecordingRecord>, LedgerError> {
        self.port().recordings()
    }
    fn commit(&self, _batch: &LedgerCommit) -> Result<(), LedgerError> {
        Err(LedgerError::Io("read-only ledger".into()))
    }
    fn set_source_path(&self, _id: &RecordingId, _path: &Path) -> Result<(), LedgerError> {
        Err(LedgerError::Io("read-only ledger".into()))
    }
    fn set_audio_trashed(&self, _id: &RecordingId, _at: UtcInstant) -> Result<(), LedgerError> {
        Err(LedgerError::Io("read-only ledger".into()))
    }
}

impl RetentionJournal for RuntimeLedger {
    fn record_intent(&self, intent: &NewIntent) -> Result<i64, LedgerError> {
        match self {
            Self::Sqlite(ledger) => ledger.record_intent(intent),
            Self::Empty(_) => Err(LedgerError::Io("read-only ledger".into())),
        }
    }

    fn pending_intents(&self) -> Result<Vec<RetentionIntent>, LedgerError> {
        match self {
            Self::Sqlite(ledger) => ledger.pending_intents(),
            Self::Empty(ledger) => ledger.pending_intents(),
        }
    }

    fn complete_intent(&self, id: i64, at: UtcInstant) -> Result<(), LedgerError> {
        match self {
            Self::Sqlite(ledger) => ledger.complete_intent(id, at),
            Self::Empty(_) => Err(LedgerError::Io("read-only ledger".into())),
        }
    }
}

#[cfg(test)]
mod tests {
    mod retention;

    use super::*;
    use vpt_adapters::RootDir;
    use vpt_application::ports::{ProposalLedger, StageStates, TitleOrigin};
    use vpt_domain::time::UtcOffset;

    #[test]
    fn read_only_runtime_returns_empty_memory_without_creating_absent_state() {
        let temp = tempfile::tempdir().unwrap();
        let absent = RuntimeLedger::read_only(&temp.path().join("absent")).unwrap();
        assert_eq!(absent.recordings(), Ok(vec![]));
        let id = RecordingId::parse("2026-08-24T144736-4f3ab19c02de").unwrap();
        assert_eq!(absent.accepted_proposal(&id), Ok(None));
        assert_eq!(absent.grounding_basis(&id), Ok(None));
        assert!(!temp.path().join("absent").exists());
    }

    #[test]
    fn read_only_runtime_refuses_mutating_trait_calls() {
        let temp = tempfile::tempdir().unwrap();
        let ledger = RuntimeLedger::read_only(&temp.path().join("absent")).unwrap();
        assert_eq!(
            ledger.commit(&LedgerCommit::default()),
            Err(LedgerError::Io("read-only ledger".into()))
        );
    }

    #[test]
    fn read_only_runtime_delegates_to_existing_sqlite() {
        let temp = tempfile::tempdir().unwrap();
        let root = RootDir::open(temp.path()).unwrap();
        let ledger = SqliteLedger::open(&root).unwrap();
        let digest = Sha256Digest([0x21; 32]);
        let captured = UtcInstant {
            secs: 1_787_604_456,
        };
        let offset = UtcOffset { secs: 0 };
        let record = RecordingRecord {
            id: RecordingId::derive(captured, offset, &digest).unwrap(),
            source_path: None,
            digest,
            captured_at: captured,
            captured_offset: offset,
            duration_secs: 1,
            title: None,
            title_source: TitleOrigin::Unavailable,
            ingested_at: captured,
            audio_path: temp.path().join("a.m4a"),
            stages: StageStates::fresh(),
            audio_trashed_at: None,
        };
        ledger
            .commit(&LedgerCommit {
                recordings: vec![record.clone()],
                ..Default::default()
            })
            .unwrap();
        drop(ledger);
        let observed = RuntimeLedger::read_only(temp.path()).unwrap();
        assert_eq!(observed.accepted_proposal(&record.id), Ok(None));
        assert_eq!(observed.grounding_basis(&record.id), Ok(None));
        assert_eq!(observed.by_id(&record.id), Ok(Some(record)));
    }
}
