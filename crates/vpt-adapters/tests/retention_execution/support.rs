use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use vpt_adapters::{FilesystemStores, MemoryLedger};
use vpt_application::ports::*;
use vpt_application::{RetentionSettings, StorePaths};
use vpt_domain::history::RunId;
use vpt_domain::identity::RecordingId;
use vpt_domain::layout::StoreKey;
use vpt_domain::retention::Hold;
use vpt_domain::run::{HistoryFact, RunKind};
use vpt_domain::time::{UtcInstant, UtcOffset};

pub const NOW: UtcInstant = UtcInstant {
    secs: 3_000_000_000,
};

pub struct FixedClock;
impl Clock for FixedClock {
    fn now(&self) -> UtcInstant {
        NOW
    }
    fn offset_at(&self, _: UtcInstant) -> UtcOffset {
        UtcOffset { secs: 0 }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Export(HistoryReceipt),
    Trash(PathBuf),
}

pub type Trace = Rc<RefCell<Vec<Action>>>;

pub struct Sink {
    pub trace: Trace,
    pub stop_after_intent: bool,
}

impl EventSink for Sink {
    fn after_commit(&mut self, receipt: &HistoryReceipt) -> SinkOutcome {
        self.trace
            .borrow_mut()
            .push(Action::Export(receipt.clone()));
        if self.stop_after_intent
            && receipt
                .records
                .iter()
                .any(|record| matches!(record.fact, HistoryFact::EffectIntent { .. }))
        {
            SinkOutcome::Stop(Box::new(SinkStop {
                reason: SinkError::UnsafeState,
                committed: Some(receipt.clone()),
                warnings: Default::default(),
            }))
        } else {
            SinkOutcome::Continue {
                warnings: Default::default(),
            }
        }
    }
    fn recover(&mut self, _: &vpt_domain::run::RecoveryView) -> SinkOutcome {
        panic!("retention must not replay exports");
    }
}

#[derive(Clone, Copy)]
pub enum TrashMode {
    Move,
    Failed,
    Unknown,
}

pub struct MovingTrash {
    pub destination: PathBuf,
    pub trace: Trace,
    pub mode: TrashMode,
}

impl Trash for MovingTrash {
    fn trash(&self, path: &Path) -> Result<PathBuf, TrashError> {
        self.trace.borrow_mut().push(Action::Trash(path.into()));
        match self.mode {
            TrashMode::Failed => Err(TrashError::Failed("owned helper failed".into())),
            TrashMode::Unknown => Err(TrashError::Unknown("owned helper outcome unknown".into())),
            TrashMode::Move => {
                let target = self.destination.join(path.file_name().unwrap());
                fs::rename(path, &target).unwrap();
                Ok(target)
            }
        }
    }
    fn take_diagnostics(&self) -> Vec<String> {
        vec![]
    }
}

pub struct World {
    pub temp: tempfile::TempDir,
    pub ledger: MemoryLedger,
    pub stores: FilesystemStores,
    pub paths: StorePaths,
    pub settings: RetentionSettings,
    pub trash: MovingTrash,
    pub sink: Sink,
}

impl World {
    pub fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let paths = StorePaths {
            audio: base.join("audio"),
            transcripts: base.join("transcripts"),
            analysis: base.join("analysis"),
            briefs: base.join("briefs"),
            engine_outputs: base.join("engine-outputs"),
            drafts: base.join("drafts"),
            released: base.join("released"),
        };
        let roots: Vec<_> = StoreKey::all()
            .into_iter()
            .map(|key| paths.get(key).to_path_buf())
            .collect();
        for root in &roots {
            fs::create_dir(root).unwrap();
        }
        let destination = base.join("owned-trash");
        fs::create_dir(&destination).unwrap();
        let trace = Rc::new(RefCell::new(vec![]));
        Self {
            temp,
            ledger: MemoryLedger::new(),
            stores: FilesystemStores::open(&roots).unwrap(),
            paths,
            settings: RetentionSettings {
                enabled: true,
                include_audio: true,
                holds: vec![(StoreKey::Audio, Hold::of_seconds(1))],
            },
            trash: MovingTrash {
                destination,
                trace: trace.clone(),
                mode: TrashMode::Move,
            },
            sink: Sink {
                trace,
                stop_after_intent: false,
            },
        }
    }

    pub fn record(&self, name: &str, bytes: &[u8]) -> RecordingRecord {
        let path = self.paths.audio.join(name);
        fs::write(&path, bytes).unwrap();
        let digest = self.stores.digest_bytes(bytes);
        let captured_at = UtcInstant { secs: 100 };
        let captured_offset = UtcOffset { secs: 0 };
        let record = RecordingRecord {
            id: RecordingId::derive(captured_at, captured_offset, &digest).unwrap(),
            digest,
            captured_at,
            captured_offset,
            audio_path: path,
            source_path: None,
            duration_secs: 1,
            title: None,
            title_source: TitleOrigin::Unavailable,
            ingested_at: UtcInstant { secs: 101 },
            stages: StageStates::fresh(),
            audio_trashed_at: None,
        };
        self.ledger
            .commit(&LedgerCommit {
                recordings: vec![record.clone()],
                ..Default::default()
            })
            .unwrap();
        record
    }

    pub fn run(&self) -> RunId {
        committed(
            self.ledger
                .begin_run(RunKind::Recover, UtcInstant { secs: 200 }),
        )
        .run
    }

    pub fn prepare(&self, record: &RecordingRecord) -> PreparedRetention {
        committed(
            self.ledger
                .prepare_retention(self.run(), &new_intent(record)),
        )
    }
}

pub fn new_intent(record: &RecordingRecord) -> NewIntent {
    NewIntent {
        kind: ArtifactKind::Audio,
        recording: record.id.clone(),
        path: record.audio_path.clone(),
        expected: record.digest,
        recorded_at: UtcInstant { secs: 201 },
    }
}

pub fn committed<T: std::fmt::Debug>(value: HistoryWrite<T>) -> T {
    match value {
        HistoryWrite::Committed(value) => value,
        other => panic!("fixture commit: {other:?}"),
    }
}

pub fn report_events(trace: &Trace) -> Vec<vpt_domain::history::Event> {
    trace
        .borrow()
        .iter()
        .flat_map(|action| match action {
            Action::Export(receipt) => receipt
                .records
                .iter()
                .filter_map(|record| match &record.fact {
                    HistoryFact::Event(event)
                        if event.kind == vpt_domain::history::EventKind::Retention =>
                    {
                        Some(event.clone())
                    }
                    _ => None,
                })
                .collect(),
            Action::Trash(_) => vec![],
        })
        .collect()
}
