use super::*;
use crate::SourceSettings;
use crate::ports::*;
use std::path::{Path, PathBuf};
use vpt_domain::container::ReadFailure;
use vpt_domain::digest::Sha256Digest;
use vpt_domain::history::{EventId, LedgerNamespace};
use vpt_domain::identity::RecordingId;
use vpt_domain::run::{
    ExportCursor, ExportFailure, ExportPosition, HistoryFact, RecoveryEvidence, RecoveryView,
    RunKind,
};
use vpt_domain::time::{FileTime, UtcOffset};

struct Silent;

impl RecorderStore for Silent {
    type Handle = ();
    fn digest(&self, _: &(), _: u64) -> Result<Sha256Digest, RecorderError> {
        panic!("source digest")
    }
    fn candidates(&self) -> Result<Vec<Candidate>, RecorderError> {
        panic!("candidate list")
    }
    fn candidate(&self, _: &Path) -> Result<Candidate, RecorderError> {
        panic!("candidate lookup")
    }
    fn open(&self, _: &Path) -> Result<(), RecorderError> {
        panic!("source open")
    }
    fn metadata(&self, _: &()) -> Result<SourceMetadata, RecorderError> {
        panic!("source metadata")
    }
    fn read_at(&self, _: &(), _: u64, _: &mut [u8]) -> Result<(), ReadFailure> {
        panic!("source read")
    }
    fn clone_into(
        &self,
        _: &(),
        _: &Path,
        _: (u64, u64),
        _: &str,
    ) -> Result<CloneKind, CloneError> {
        panic!("clone")
    }
    fn subdirectory_counts(&self) -> Vec<(String, Option<u64>)> {
        panic!("subdirectories")
    }
    fn title(&self, _: &str) -> TitleLookup {
        panic!("title")
    }
    fn refresh_titles(&self) -> Result<(), RecorderError> {
        panic!("title refresh")
    }
}

impl Archive for Silent {
    type Handle = ();
    fn stage<C>(&self, _: C) -> Result<Staged, StageFailure>
    where
        C: FnOnce(&Path, (u64, u64), &str) -> Result<CloneKind, CloneError>,
    {
        panic!("stage")
    }
    fn open(&self, _: &Path) -> Result<(), ArchiveError> {
        panic!("staged open")
    }
    fn size(&self, _: &()) -> Result<u64, ArchiveError> {
        panic!("staged size")
    }
    fn read_at(&self, _: &(), _: u64, _: &mut [u8]) -> Result<(), ReadFailure> {
        panic!("staged read")
    }
    fn validate_trash_path(&self, _: &Path) -> Result<(), ArchiveError> {
        panic!("cleanup validation")
    }
    fn digest(&self, _: &Path) -> Result<Sha256Digest, ArchiveError> {
        panic!("digest")
    }
    fn digest_handle(&self, _: &()) -> Result<Sha256Digest, ArchiveError> {
        panic!("digest handle")
    }
    fn target(&self, _: &str) -> PathBuf {
        panic!("target")
    }
    fn archived(&self) -> Result<Vec<PathBuf>, ArchiveError> {
        panic!("archive listing")
    }
    fn staged_leftovers(&self) -> Result<Vec<PathBuf>, ArchiveError> {
        panic!("stage listing")
    }
    fn publish(&self, _: &Staged, _: &(), _: &str) -> Result<Published, ArchiveError> {
        panic!("publish")
    }
    fn sync_existing(&self, _: &Path, _: &Sha256Digest) -> Result<(), ArchiveError> {
        panic!("sync")
    }
}

impl RecordingLedger for Silent {
    fn seen(&self, _: &Path) -> Result<Option<SeenRow>, LedgerError> {
        panic!("seen read")
    }
    fn seen_all(&self) -> Result<Vec<SeenRow>, LedgerError> {
        panic!("seen listing")
    }
    fn record_seen(&self, _: &SeenRow) -> Result<(), LedgerError> {
        panic!("seen write")
    }
    fn by_digest(&self, _: &Sha256Digest) -> Result<Option<RecordingRecord>, LedgerError> {
        panic!("digest lookup")
    }
    fn by_id(&self, _: &RecordingId) -> Result<Option<RecordingRecord>, LedgerError> {
        panic!("id lookup")
    }
    fn recordings(&self) -> Result<Vec<RecordingRecord>, LedgerError> {
        panic!("recording listing")
    }
    fn commit(&self, _: &LedgerCommit) -> Result<(), LedgerError> {
        panic!("commit")
    }
    fn set_source_path(&self, _: &RecordingId, _: &Path) -> Result<(), LedgerError> {
        panic!("source update")
    }
    fn set_audio_trashed(&self, _: &RecordingId, _: UtcInstant) -> Result<(), LedgerError> {
        panic!("trash mark")
    }
}

impl RunHistory for Silent {
    fn begin_run(&self, _: RunKind, _: UtcInstant) -> HistoryWrite<HistoryReceipt> {
        panic!("run start")
    }
    fn begin_recovery(&self, _: UtcInstant, _: RecoveryEvidence) -> HistoryWrite<HistoryReceipt> {
        panic!("recovery start")
    }
    fn commit_step(
        &self,
        _: RunId,
        _: &WorkDelta,
        _: &[(UtcInstant, HistoryFact)],
    ) -> HistoryWrite<HistoryReceipt> {
        panic!("history write")
    }
    fn recovery_view(&self) -> Result<RecoveryView, LedgerError> {
        panic!("recovery")
    }
    fn pending_exports(&self) -> Result<Vec<vpt_domain::run::HistoryRecord>, LedgerError> {
        panic!("pending exports")
    }
    fn acknowledge_export(
        &self,
        _: EventId,
        _: Option<ExportCursor>,
        _: ExportPosition,
        _: UtcInstant,
    ) -> HistoryWrite<()> {
        panic!("export acknowledge")
    }
    fn note_export_failure(&self, _: &ExportFailure) -> HistoryWrite<()> {
        panic!("export failure")
    }
    fn restart_export(
        &self,
        _: ExportCursor,
        _: ExportPosition,
        _: UtcInstant,
    ) -> HistoryWrite<()> {
        panic!("export restart")
    }
}

impl Clock for Silent {
    fn now(&self) -> UtcInstant {
        panic!("clock")
    }
    fn offset_at(&self, _: UtcInstant) -> UtcOffset {
        panic!("offset")
    }
}

impl Trash for Silent {
    fn trash(&self, _: &Path) -> Result<PathBuf, TrashError> {
        panic!("cleanup")
    }
    fn take_diagnostics(&self) -> Vec<String> {
        panic!("cleanup diagnostics")
    }
}

impl EventSink for Silent {
    fn after_commit(&mut self, _: &HistoryReceipt) -> SinkOutcome {
        panic!("sink")
    }
    fn recover(&mut self, _: &RecoveryView) -> SinkOutcome {
        panic!("sink recovery")
    }
}

#[test]
fn private_dry_deferral_adds_report_without_any_port_action() {
    let recorder = Silent;
    let archive = Silent;
    let ledger = Silent;
    let clock = Silent;
    let trash = Silent;
    let mut sink = Silent;
    let settings = SourceSettings {
        recordings_dir: PathBuf::from("/synthetic/recordings"),
        read_titles: false,
        quiet_period_secs: 30,
        deferral_page_threshold: 2,
        max_audio_bytes: 1024,
    };
    let mut ingest = Ingest {
        recorder: &recorder,
        archive: &archive,
        ledger: &ledger,
        clock: &clock,
        trash: &trash,
        sink: &mut sink,
        settings: &settings,
    };
    let candidate = Candidate {
        path: PathBuf::from("/synthetic/recordings/private.m4a"),
        file_name: "private.m4a".into(),
        size: 100,
        mtime: FileTime { secs: 1, nanos: 0 },
        flags: 0,
    };
    let mut report = IngestReport::default();
    let mode = Mode {
        dry_run: true,
        once: None,
        import: None,
    };

    ingest
        .defer(
            RunId::new(LedgerNamespace::new([1; 16]), 1),
            Deferral {
                candidate: &candidate,
                seen: None,
                reason: DeferralReason::NotAtRest,
            },
            UtcInstant { secs: 2 },
            &mode,
            &mut report,
        )
        .unwrap();

    assert_eq!(
        report.deferred,
        vec![crate::ingest::Deferred {
            path: candidate.path,
            reason: DeferralReason::NotAtRest,
        }]
    );
}
