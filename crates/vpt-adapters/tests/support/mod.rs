#![allow(
    dead_code,
    reason = "shared ingest fixture serves focused test targets"
)]

pub mod faults;
pub mod gates;

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};
use vpt_adapters::{ClonefileArchive, RootDir, SqliteLedger, VoiceMemosStore};
use vpt_application::ports::{
    Clock, EventSink, HistoryReceipt, SinkError, SinkOutcome, SinkStop, SinkWarning, Trash,
    TrashError,
};
use vpt_application::{Ingest, SourceSettings};
use vpt_domain::run::{EffectKind, EffectReference, HistoryFact, RecoveryView};
use vpt_domain::time::{UtcInstant, UtcOffset};

pub const CAPTURED: i64 = 1_787_604_456;

pub struct Fixture {
    pub temp: tempfile::TempDir,
    pub recordings: PathBuf,
    pub audio: PathBuf,
    pub state: PathBuf,
}

impl Fixture {
    pub fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let recordings = base.join("voice-memos/Recordings");
        let audio = base.join("home/audio");
        let state = base.join("state/vpt");
        for directory in [&recordings, &audio, &state] {
            std::fs::create_dir_all(directory).unwrap();
        }
        Self {
            temp,
            recordings,
            audio,
            state,
        }
    }

    pub fn add_recording(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.recordings.join(name);
        std::fs::write(&path, bytes).unwrap();
        let modified = UNIX_EPOCH + Duration::from_secs(u64::try_from(CAPTURED).unwrap());
        std::fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(modified))
            .unwrap();
        path
    }

    pub fn add_title(&self, file_name: &str, title: &str) {
        let database = rusqlite::Connection::open(
            self.recordings.parent().unwrap().join("CloudRecordings.db"),
        )
        .unwrap();
        database.execute_batch("CREATE TABLE IF NOT EXISTS ZCLOUDRECORDING (Z_PK INTEGER PRIMARY KEY, ZPATH TEXT, ZCUSTOMLABEL TEXT)").unwrap();
        database
            .execute(
                "INSERT INTO ZCLOUDRECORDING (ZPATH, ZCUSTOMLABEL) VALUES (?1, ?2)",
                [file_name, title],
            )
            .unwrap();
    }

    pub fn parts(&self) -> Parts {
        Parts {
            store: VoiceMemosStore::open(&self.recordings).unwrap(),
            archive: ClonefileArchive::open(&self.audio).unwrap(),
            ledger: SqliteLedger::open(&RootDir::open(&self.state).unwrap()).unwrap(),
            clock: FixedClock::default(),
            trash: RecordingTrash::new(self.temp.path().join("trash")),
            sink: RecordingSink::default(),
            settings: SourceSettings {
                recordings_dir: self.recordings.clone(),
                read_titles: false,
                quiet_period_secs: 30,
                deferral_page_threshold: 4,
                max_audio_bytes: 2_147_483_648,
            },
        }
    }
}

#[derive(Clone, Copy)]
pub struct FixedClock {
    pub now: UtcInstant,
    pub offset: UtcOffset,
}

impl Default for FixedClock {
    fn default() -> Self {
        Self {
            now: UtcInstant {
                secs: CAPTURED + 3_600,
            },
            offset: UtcOffset { secs: -21_600 },
        }
    }
}

impl Clock for FixedClock {
    fn now(&self) -> UtcInstant {
        self.now
    }

    fn offset_at(&self, _at: UtcInstant) -> UtcOffset {
        self.offset
    }
}

pub struct RecordingTrash {
    directory: PathBuf,
    pub attempted: RefCell<Vec<PathBuf>>,
    pub moved: RefCell<Vec<PathBuf>>,
    pub failure: RefCell<Option<TrashError>>,
    pub diagnostics: RefCell<Vec<String>>,
}

impl RecordingTrash {
    fn new(directory: PathBuf) -> Self {
        std::fs::create_dir_all(&directory).unwrap();
        Self {
            directory,
            attempted: RefCell::new(Vec::new()),
            moved: RefCell::new(Vec::new()),
            failure: RefCell::new(None),
            diagnostics: RefCell::new(Vec::new()),
        }
    }
}

impl Trash for RecordingTrash {
    fn trash(&self, path: &Path) -> Result<PathBuf, TrashError> {
        self.attempted.borrow_mut().push(path.to_path_buf());
        if let Some(error) = self.failure.borrow_mut().take() {
            return Err(error);
        }
        let target = self.directory.join(path.file_name().unwrap());
        std::fs::rename(path, &target)
            .map_err(|error| TrashError::Failed(error.kind().to_string()))?;
        self.moved.borrow_mut().push(path.to_path_buf());
        Ok(target)
    }

    fn take_diagnostics(&self) -> Vec<String> {
        self.diagnostics.take()
    }
}

#[derive(Default)]
pub struct RecordingSink {
    pub receipts: Vec<HistoryReceipt>,
    pub title_copy: Option<PathBuf>,
    pub title_absent_at_start: Option<bool>,
    pub watched_audio: Option<PathBuf>,
    pub intent_before_publication: Option<bool>,
    pub owned_stage_present_at_failure_event: Option<bool>,
    pub owned_stage_present_at_deferred_export: Option<bool>,
    pub stop_at: Option<StopAt>,
    pub warnings: BTreeSet<SinkWarning>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopAt {
    Start,
    Intent,
    Work,
}

impl EventSink for RecordingSink {
    fn after_commit(&mut self, committed: &HistoryReceipt) -> SinkOutcome {
        if committed
            .records
            .iter()
            .any(|record| matches!(record.fact, HistoryFact::RunStarted(_)))
        {
            self.title_absent_at_start = self.title_copy.as_ref().map(|path| !path.exists());
        }
        for record in &committed.records {
            if matches!(
                record.fact,
                HistoryFact::WorkStep {
                    outcome: vpt_domain::run::WorkOutcome::Deferred,
                    ..
                }
            ) {
                self.owned_stage_present_at_deferred_export =
                    self.watched_audio.as_ref().map(|audio| {
                        std::fs::read_dir(audio).unwrap().any(|entry| {
                            entry
                                .unwrap()
                                .file_name()
                                .to_string_lossy()
                                .starts_with(".vpt-staging-")
                        })
                    });
            }
            if let HistoryFact::EffectIntent {
                kind: EffectKind::Archive,
                reference: EffectReference::ArchiveTarget(target),
            } = &record.fact
            {
                self.intent_before_publication = self.watched_audio.as_ref().map(|audio| {
                    !target.exists()
                        && std::fs::read_dir(audio).unwrap().any(|entry| {
                            entry
                                .unwrap()
                                .file_name()
                                .to_string_lossy()
                                .starts_with(".vpt-staging-")
                        })
                });
            }
            if matches!(record.fact, HistoryFact::Event(_)) {
                self.owned_stage_present_at_failure_event =
                    self.watched_audio.as_ref().map(|audio| {
                        std::fs::read_dir(audio).unwrap().any(|entry| {
                            entry
                                .unwrap()
                                .file_name()
                                .to_string_lossy()
                                .starts_with(".vpt-staging-")
                        })
                    });
            }
        }
        self.receipts.push(committed.clone());
        let phase = if committed
            .records
            .iter()
            .any(|record| matches!(record.fact, HistoryFact::RunStarted(_)))
        {
            Some(StopAt::Start)
        } else if committed
            .records
            .iter()
            .any(|record| matches!(record.fact, HistoryFact::EffectIntent { .. }))
        {
            Some(StopAt::Intent)
        } else if committed
            .records
            .iter()
            .any(|record| matches!(record.fact, HistoryFact::WorkStep { .. }))
        {
            Some(StopAt::Work)
        } else {
            None
        };
        if self.stop_at.is_some() && phase == self.stop_at {
            return SinkOutcome::Stop(Box::new(SinkStop {
                reason: SinkError::UnsafeState,
                committed: Some(committed.clone()),
                warnings: self.warnings.clone(),
            }));
        }
        SinkOutcome::Continue {
            warnings: self.warnings.clone(),
        }
    }

    fn recover(&mut self, _view: &RecoveryView) -> SinkOutcome {
        SinkOutcome::Continue {
            warnings: Default::default(),
        }
    }
}

pub struct Parts {
    pub store: VoiceMemosStore,
    pub archive: ClonefileArchive,
    pub ledger: SqliteLedger,
    pub clock: FixedClock,
    pub trash: RecordingTrash,
    pub sink: RecordingSink,
    pub settings: SourceSettings,
}

impl Parts {
    pub fn ingest(
        &mut self,
    ) -> Ingest<
        '_,
        VoiceMemosStore,
        ClonefileArchive,
        SqliteLedger,
        FixedClock,
        RecordingTrash,
        RecordingSink,
    > {
        Ingest {
            recorder: &self.store,
            archive: &self.archive,
            ledger: &self.ledger,
            clock: &self.clock,
            trash: &self.trash,
            sink: &mut self.sink,
            settings: &self.settings,
        }
    }
}
