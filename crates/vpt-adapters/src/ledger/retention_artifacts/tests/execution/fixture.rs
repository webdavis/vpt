use super::*;
use crate::FilesystemStores;
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
};
use vpt_application::{RetentionSettings, StorePaths};
use vpt_domain::{
    layout::StoreKey,
    retention::Hold,
    run::{EffectReference, HistoryFact},
    time::UtcOffset,
};

pub trait Ledger:
    RetentionArtifacts
    + RecordingLedger
    + RunHistory
    + PublicationJournal
    + OccasionLedger
    + ReleaseLedger
    + ReleaseHistory
    + RetentionHistory
    + RetentionJournal
{
}
impl<
    T: RetentionArtifacts
        + RecordingLedger
        + RunHistory
        + PublicationJournal
        + OccasionLedger
        + ReleaseLedger
        + ReleaseHistory
        + RetentionHistory
        + RetentionJournal,
> Ledger for T
{
}

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
pub struct Sink;
impl EventSink for Sink {
    fn after_commit(&mut self, _: &HistoryReceipt) -> SinkOutcome {
        SinkOutcome::Continue {
            warnings: Default::default(),
        }
    }
    fn recover(&mut self, _: &vpt_domain::run::RecoveryView) -> SinkOutcome {
        panic!("unexpected export replay")
    }
}
pub struct World {
    _temp: tempfile::TempDir,
    pub paths: StorePaths,
    pub stores: FilesystemStores,
    pub settings: RetentionSettings,
    pub trash: PathBuf,
}
impl World {
    pub fn new() -> Self {
        let temp = crate::test_temp::tempdir().unwrap();
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
            .iter()
            .map(|key| paths.get(*key).to_path_buf())
            .collect();
        for root in &roots {
            std::fs::create_dir(root).unwrap();
        }
        let trash = base.join("trash");
        std::fs::create_dir(&trash).unwrap();
        Self {
            _temp: temp,
            paths,
            stores: FilesystemStores::open(&roots).unwrap(),
            settings: RetentionSettings {
                enabled: true,
                include_audio: false,
                holds: StoreKey::all()
                    .iter()
                    .map(|key| (*key, Hold::of_seconds(1)))
                    .collect(),
            },
            trash,
        }
    }
}
pub struct MovingTrash<'a, L> {
    ledger: &'a L,
    world: &'a World,
    pub moved: RefCell<Vec<PathBuf>>,
}
impl<'a, L> MovingTrash<'a, L> {
    pub fn new(ledger: &'a L, world: &'a World) -> Self {
        Self {
            ledger,
            world,
            moved: RefCell::new(vec![]),
        }
    }
}
impl<L: Ledger> Trash for MovingTrash<'_, L> {
    fn trash(&self, path: &Path) -> Result<PathBuf, TrashError> {
        let pending = self.ledger.pending_intents().unwrap();
        let intent = pending
            .iter()
            .find(|row| row.path == path)
            .expect("intent committed before helper");
        assert_eq!(
            self.world.stores.digest_of(path).unwrap(),
            Some(intent.expected)
        );
        assert!(self.ledger.pending_exports().unwrap().iter().any(|row|matches!(&row.fact,HistoryFact::EffectIntent { reference:EffectReference::RetentionIntent { id,path:p },.. } if *id==intent.id as u64 && p==path)));
        let destination = self.world.trash.join(format!(
            "{}-{}",
            self.moved.borrow().len(),
            path.file_name().unwrap().to_str().unwrap()
        ));
        std::fs::rename(path, &destination).unwrap();
        self.moved.borrow_mut().push(path.into());
        Ok(destination)
    }
    fn take_diagnostics(&self) -> Vec<String> {
        vec![]
    }
}

pub fn seed(ledger: &impl Ledger, world: &World) {
    let run = notes::seed(ledger);
    let mut output = engine::delta();
    output.engine_outputs[0].target = world
        .paths
        .engine_outputs
        .join(output.engine_outputs[0].target.file_name().unwrap());
    output.commit.publications[0].target = output.engine_outputs[0].target.clone();
    engine::committed(ledger.commit_step(run, &output, &engine::fact()));
    std::fs::write(
        &output.engine_outputs[0].target,
        &output.engine_outputs[0].bytes,
    )
    .unwrap();
    ledger
        .clear_publication(&output.engine_outputs[0].target)
        .unwrap();
    for stage in [NoteStage::Transcript, NoteStage::Analysis] {
        let mut delta = notes::delta();
        let note = &mut delta.notes[0].artifact;
        note.stage = stage;
        note.target = if stage == NoteStage::Transcript {
            world.paths.transcripts.join("owned.md")
        } else {
            world.paths.analysis.join("owned.md")
        };
        note.text = note.text.replace(
            "vpt-stage: transcript",
            &format!("vpt-stage: {}", stage.as_str()),
        );
        note.digest = notes::hash(note.text.as_bytes());
        delta.commit.publications[0].target = note.target.clone();
        delta.commit.publications[0].intended = note.digest;
        engine::committed(ledger.commit_step(run, &delta, &notes::facts()));
        let note = &delta.notes[0].artifact;
        std::fs::write(&note.target, format!("{}\nNew operator prose\n", note.text)).unwrap();
        ledger.clear_publication(&note.target).unwrap();
    }
    let (run, occasion) = brief::occasion(ledger);
    let mut brief = brief::artifact(&occasion, 10);
    brief.target = world.paths.briefs.join(brief.target.file_name().unwrap());
    engine::committed(brief::save(ledger, run, &occasion, None, &brief));
    std::fs::write(
        &brief.target,
        format!("{}\nOperator addition\n", brief.text),
    )
    .unwrap();
    engine::committed(brief::publish(ledger, run, &brief, 11));
    let (run, mut draft) = releases::setup(ledger);
    draft.destination = world.paths.released.join("copy.md");
    draft.report_root = world.paths.drafts.clone();
    let release = engine::committed(ledger.prepare_release(run, &draft)).release;
    let payload = ledger.release_payload(release.sequence).unwrap().unwrap();
    std::fs::write(&release.destination, &payload.text).unwrap();
    std::fs::write(&release.report_path, &payload.report).unwrap();
    engine::committed(ledger.complete_release(
        run,
        release.sequence,
        release.version,
        draft.created_at,
    ));
}
