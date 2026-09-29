use super::*;
use crate::FilesystemStores;
use std::cell::Cell;
use std::path::{Path, PathBuf};
use vpt_application::ports::{
    Clock, EventSink, RecordingLedger, SinkOutcome, Stores, Trash, TrashError,
};
use vpt_application::{
    ReconcileProgress, Retention, RetentionError, RetentionSettings, StorePaths,
};
use vpt_domain::layout::StoreKey;
use vpt_domain::retention::Hold;
use vpt_domain::time::UtcOffset;

struct FixedClock;
impl Clock for FixedClock {
    fn now(&self) -> UtcInstant {
        UtcInstant {
            secs: 3_000_000_000,
        }
    }
    fn offset_at(&self, _: UtcInstant) -> UtcOffset {
        UtcOffset { secs: 0 }
    }
}

struct MovingTrash {
    directory: PathBuf,
    calls: Cell<u32>,
}
impl Trash for MovingTrash {
    fn trash(&self, path: &Path) -> Result<PathBuf, TrashError> {
        self.calls.set(self.calls.get() + 1);
        let target = self.directory.join(path.file_name().unwrap());
        std::fs::rename(path, &target).unwrap();
        Ok(target)
    }
    fn take_diagnostics(&self) -> Vec<String> {
        vec![]
    }
}

#[derive(Default)]
struct Sink {
    receipts: Vec<HistoryReceipt>,
}
impl EventSink for Sink {
    fn after_commit(&mut self, receipt: &HistoryReceipt) -> SinkOutcome {
        self.receipts.push(receipt.clone());
        SinkOutcome::Continue {
            warnings: Default::default(),
        }
    }
    fn recover(&mut self, _: &vpt_domain::run::RecoveryView) -> SinkOutcome {
        panic!("retention cannot replay the log");
    }
}

struct Fixture {
    paths: StorePaths,
    stores: FilesystemStores,
    trash: MovingTrash,
    settings: RetentionSettings,
    record: vpt_application::ports::RecordingRecord,
}

impl Fixture {
    fn new(ledger: &SqliteLedger, root: &RootDir) -> Self {
        let base = root.path().parent().unwrap();
        let audio = base.join("audio");
        let destination = base.join("owned-trash");
        std::fs::create_dir(&audio).unwrap();
        std::fs::create_dir(&destination).unwrap();
        let paths = StorePaths {
            audio: audio.clone(),
            transcripts: base.join("transcripts"),
            analysis: base.join("analysis"),
            briefs: base.join("briefs"),
            engine_outputs: base.join("engine-outputs"),
            drafts: base.join("drafts"),
            released: base.join("released"),
        };
        let stores = FilesystemStores::open(std::slice::from_ref(&audio)).unwrap();
        let mut record = contract::record(126);
        record.audio_path = audio.join("owned.m4a");
        record.digest = stores.digest_bytes(b"owned audio for failure proof");
        record.id = vpt_domain::identity::RecordingId::derive(
            record.captured_at,
            record.captured_offset,
            &record.digest,
        )
        .unwrap();
        std::fs::write(&record.audio_path, b"owned audio for failure proof").unwrap();
        ledger
            .commit(&LedgerCommit {
                recordings: vec![record.clone()],
                ..Default::default()
            })
            .unwrap();
        Self {
            paths,
            stores,
            record,
            trash: MovingTrash {
                directory: destination,
                calls: Cell::new(0),
            },
            settings: RetentionSettings {
                enabled: true,
                include_audio: true,
                holds: vec![(StoreKey::Audio, Hold::of_seconds(1))],
            },
        }
    }

    fn run(
        &self,
        ledger: &super::application_calls::Calls<'_>,
        sink: &mut Sink,
    ) -> Box<vpt_application::RetentionFailure> {
        Retention {
            ledger,
            stores: &self.stores,
            trash: &self.trash,
            clock: &FixedClock,
            sink,
            settings: &self.settings,
            paths: &self.paths,
        }
        .run(false, ReconcileProgress::default())
        .unwrap_err()
    }
}

pub(super) fn preparation(ledger: &SqliteLedger, root: &RootDir) {
    let fixture = Fixture::new(ledger, root);
    abort(ledger, "BEFORE INSERT ON retention_intents");
    let counted = super::application_calls::Calls::new(ledger);
    let mut sink = Sink::default();
    let failed = fixture.run(&counted, &mut sink);
    assert_eq!(
        failed.error,
        RetentionError::RequiredHistory(LedgerError::Conflict(
            "database constraint violated".into()
        ))
    );
    assert!(failed.report.moved.is_empty());
    assert!(failed.report.completed.is_empty());
    assert!(failed.committed.is_none());
    assert_eq!(
        counted.writes.get(),
        2,
        "no history write after refused preparation"
    );
    assert_eq!(
        sink.receipts.len(),
        1,
        "only the committed run start was exported"
    );
    assert_eq!(fixture.trash.calls.get(), 0);
    assert_eq!(
        std::fs::read(&fixture.record.audio_path).unwrap(),
        b"owned audio for failure proof"
    );
    assert!(ledger.pending_intents().unwrap().is_empty());
}

pub(super) fn completion(ledger: &SqliteLedger, root: &RootDir) {
    let fixture = Fixture::new(ledger, root);
    abort(
        ledger,
        "BEFORE INSERT ON history_records WHEN json_extract(NEW.fact, '$.tag') = 'work' AND (SELECT completed_at FROM retention_intents LIMIT 1) IS NOT NULL",
    );
    let counted = super::application_calls::Calls::new(ledger);
    let mut sink = Sink::default();
    let failed = fixture.run(&counted, &mut sink);
    assert_eq!(
        failed.error,
        RetentionError::RequiredHistory(LedgerError::Conflict(
            "database constraint violated".into()
        ))
    );
    assert_eq!(failed.report.moved.len(), 1);
    assert_eq!(failed.report.completed, vec![fixture.record.id.clone()]);
    assert_eq!(
        counted.writes.get(),
        3,
        "no history write after refused completion"
    );
    assert_eq!(
        sink.receipts.len(),
        2,
        "only run start and preparation were exported"
    );
    assert_eq!(fixture.trash.calls.get(), 1);
    assert_eq!(
        std::fs::read(fixture.trash.directory.join("owned.m4a")).unwrap(),
        b"owned audio for failure proof"
    );
    assert!(!fixture.record.audio_path.exists());
    assert_eq!(ledger.pending_intents().unwrap().len(), 1);
    assert!(
        ledger
            .by_id(&fixture.record.id)
            .unwrap()
            .unwrap()
            .audio_trashed_at
            .is_none()
    );
}

pub(super) mod post_commit;
