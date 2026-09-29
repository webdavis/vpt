use std::cell::Cell;
use std::fs::File;
use std::path::{Path, PathBuf};
use vpt_adapters::{ClonefileArchive, SqliteLedger, VoiceMemosStore};
use vpt_application::ports::{
    Archive, ArchiveError, Candidate, CloneError, CloneKind, HistoryReceipt, HistoryWrite,
    LedgerCommit, LedgerError, Published, RecorderError, RecorderStore, RecordingLedger,
    RecordingRecord, RunHistory, SeenRow, SourceMetadata, StageFailure, Staged, TitleLookup,
    WorkDelta,
};
use vpt_domain::container::ReadFailure;
use vpt_domain::digest::Sha256Digest;
use vpt_domain::history::{EventId, RunId};
use vpt_domain::identity::RecordingId;
use vpt_domain::run::{
    ExportCursor, ExportFailure, ExportPosition, HistoryFact, RecoveryView, RunKind,
};
use vpt_domain::sweep::SF_DATALESS;
use vpt_domain::time::UtcInstant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryFault {
    Begin,
    Intent,
    RecordUnconfirmed,
    RecordPostCommit,
    FailureEvent,
    DeferredUnconfirmed,
    DeferredPostCommit,
}

pub struct FaultLedger {
    pub base: SqliteLedger,
    pub fault: HistoryFault,
    pub failed_calls: Cell<u32>,
}

impl FaultLedger {
    pub fn new(base: SqliteLedger, fault: HistoryFault) -> Self {
        Self {
            base,
            fault,
            failed_calls: Cell::new(0),
        }
    }

    fn failure(&self) -> LedgerError {
        self.failed_calls.set(self.failed_calls.get() + 1);
        LedgerError::Io("private ledger canary".into())
    }
}

impl RunHistory for FaultLedger {
    fn begin_run(&self, kind: RunKind, at: UtcInstant) -> HistoryWrite<HistoryReceipt> {
        if self.fault == HistoryFault::Begin {
            return HistoryWrite::Unconfirmed(self.failure());
        }
        self.base.begin_run(kind, at)
    }

    fn commit_step(
        &self,
        run: RunId,
        delta: &WorkDelta,
        facts: &[(UtcInstant, HistoryFact)],
    ) -> HistoryWrite<HistoryReceipt> {
        let intent = facts
            .iter()
            .any(|(_, fact)| matches!(fact, HistoryFact::EffectIntent { .. }));
        let failure_event = facts
            .iter()
            .any(|(_, fact)| matches!(fact, HistoryFact::Event(_)));
        let deferred = facts.iter().any(|(_, fact)| {
            matches!(
                fact,
                HistoryFact::WorkStep {
                    outcome: vpt_domain::run::WorkOutcome::Deferred,
                    ..
                }
            )
        });
        if (self.fault == HistoryFault::Intent && intent)
            || (self.fault == HistoryFault::RecordUnconfirmed
                && !delta.commit.recordings.is_empty())
            || (self.fault == HistoryFault::FailureEvent && failure_event)
            || (self.fault == HistoryFault::DeferredUnconfirmed && deferred)
        {
            return HistoryWrite::Unconfirmed(self.failure());
        }
        let write = self.base.commit_step(run, delta, facts);
        if (self.fault == HistoryFault::RecordPostCommit && !delta.commit.recordings.is_empty())
            || (self.fault == HistoryFault::DeferredPostCommit && deferred)
        {
            return match write {
                HistoryWrite::Committed(receipt) => HistoryWrite::CommittedWithFailure {
                    receipt,
                    error: self.failure(),
                },
                other => other,
            };
        }
        write
    }

    fn recovery_view(&self) -> Result<RecoveryView, LedgerError> {
        self.base.recovery_view()
    }
    fn pending_exports(&self) -> Result<Vec<vpt_domain::run::HistoryRecord>, LedgerError> {
        self.base.pending_exports()
    }
    fn acknowledge_export(
        &self,
        event: EventId,
        expected: Option<ExportCursor>,
        position: ExportPosition,
        appended_at: UtcInstant,
    ) -> HistoryWrite<()> {
        self.base
            .acknowledge_export(event, expected, position, appended_at)
    }
    fn note_export_failure(&self, failure: &ExportFailure) -> HistoryWrite<()> {
        self.base.note_export_failure(failure)
    }
    fn restart_export(
        &self,
        expected: ExportCursor,
        observed: ExportPosition,
        observed_at: UtcInstant,
    ) -> HistoryWrite<()> {
        self.base.restart_export(expected, observed, observed_at)
    }
}

impl RecordingLedger for FaultLedger {
    fn seen(&self, path: &Path) -> Result<Option<SeenRow>, LedgerError> {
        self.base.seen(path)
    }
    fn seen_all(&self) -> Result<Vec<SeenRow>, LedgerError> {
        self.base.seen_all()
    }
    fn record_seen(&self, row: &SeenRow) -> Result<(), LedgerError> {
        self.base.record_seen(row)
    }
    fn by_digest(&self, digest: &Sha256Digest) -> Result<Option<RecordingRecord>, LedgerError> {
        self.base.by_digest(digest)
    }
    fn by_id(&self, id: &RecordingId) -> Result<Option<RecordingRecord>, LedgerError> {
        self.base.by_id(id)
    }
    fn recordings(&self) -> Result<Vec<RecordingRecord>, LedgerError> {
        self.base.recordings()
    }
    fn commit(&self, batch: &LedgerCommit) -> Result<(), LedgerError> {
        self.base.commit(batch)
    }
    fn set_source_path(&self, id: &RecordingId, path: &Path) -> Result<(), LedgerError> {
        self.base.set_source_path(id, path)
    }
    fn set_audio_trashed(&self, id: &RecordingId, at: UtcInstant) -> Result<(), LedgerError> {
        self.base.set_audio_trashed(id, at)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArchiveFault {
    StageOwned,
    StageUnowned,
    BeforePublish,
    EscapeBeforePublish,
    PlacedUnsynced,
}

pub struct FaultArchive {
    pub base: ClonefileArchive,
    pub fault: ArchiveFault,
}

impl Archive for FaultArchive {
    type Handle = File;

    fn stage<C>(&self, clone: C) -> Result<Staged, StageFailure>
    where
        C: FnOnce(&Path, (u64, u64), &str) -> Result<CloneKind, CloneError>,
    {
        if self.fault == ArchiveFault::StageUnowned {
            return Err(StageFailure {
                cause: ArchiveError::NoSpace,
                owned_staging: None,
            });
        }
        let staged = self.base.stage(clone)?;
        if self.fault == ArchiveFault::StageOwned {
            return Err(StageFailure {
                cause: ArchiveError::NoSpace,
                owned_staging: Some(staged.path),
            });
        }
        Ok(staged)
    }

    fn open(&self, path: &Path) -> Result<File, ArchiveError> {
        self.base.open(path)
    }
    fn size(&self, handle: &File) -> Result<u64, ArchiveError> {
        self.base.size(handle)
    }
    fn read_at(&self, handle: &File, offset: u64, buf: &mut [u8]) -> Result<(), ReadFailure> {
        self.base.read_at(handle, offset, buf)
    }
    fn validate_trash_path(&self, path: &Path) -> Result<(), ArchiveError> {
        self.base.validate_trash_path(path)
    }
    fn digest(&self, path: &Path) -> Result<Sha256Digest, ArchiveError> {
        self.base.digest(path)
    }
    fn target(&self, name: &str) -> PathBuf {
        self.base.target(name)
    }
    fn archived(&self) -> Result<Vec<PathBuf>, ArchiveError> {
        self.base.archived()
    }
    fn staged_leftovers(&self) -> Result<Vec<PathBuf>, ArchiveError> {
        self.base.staged_leftovers()
    }
    fn publish(&self, staged: &Path, target_name: &str) -> Result<Published, ArchiveError> {
        if self.fault == ArchiveFault::BeforePublish {
            return Err(ArchiveError::NoSpace);
        }
        if self.fault == ArchiveFault::EscapeBeforePublish {
            return Err(ArchiveError::Escape(staged.to_path_buf()));
        }
        let result = self.base.publish(staged, target_name)?;
        if self.fault == ArchiveFault::PlacedUnsynced
            && let Published::Placed(path) = &result
        {
            return Err(ArchiveError::PlacedUnsynced(path.clone()));
        }
        Ok(result)
    }
    fn sync_existing(&self, path: &Path) -> Result<(), ArchiveError> {
        self.base.sync_existing(path)
    }
}

pub struct DatalessRecorder {
    pub base: VoiceMemosStore,
    pub opens: Cell<u32>,
}

impl RecorderStore for DatalessRecorder {
    type Handle = File;

    fn candidates(&self) -> Result<Vec<Candidate>, RecorderError> {
        self.base.candidates().map(|mut candidates| {
            for candidate in &mut candidates {
                candidate.flags |= SF_DATALESS;
            }
            candidates
        })
    }
    fn candidate(&self, path: &Path) -> Result<Candidate, RecorderError> {
        self.base.candidate(path).map(|mut candidate| {
            candidate.flags |= SF_DATALESS;
            candidate
        })
    }
    fn open(&self, path: &Path) -> Result<File, RecorderError> {
        self.opens.set(self.opens.get() + 1);
        self.base.open(path)
    }
    fn metadata(&self, handle: &File) -> Result<SourceMetadata, RecorderError> {
        self.base.metadata(handle)
    }
    fn read_at(&self, handle: &File, offset: u64, buf: &mut [u8]) -> Result<(), ReadFailure> {
        self.base.read_at(handle, offset, buf)
    }
    fn clone_into(
        &self,
        handle: &File,
        directory: &Path,
        identity: (u64, u64),
        name: &str,
    ) -> Result<CloneKind, CloneError> {
        self.base.clone_into(handle, directory, identity, name)
    }
    fn subdirectory_counts(&self) -> Vec<(String, Option<u64>)> {
        self.base.subdirectory_counts()
    }
    fn title(&self, file_name: &str) -> TitleLookup {
        self.base.title(file_name)
    }
    fn refresh_titles(&self) -> Result<(), RecorderError> {
        self.base.refresh_titles()
    }
}
