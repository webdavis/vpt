use super::{list, show, storage};
use crate::cli::output::Outcome;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::Path;
use vpt_adapters::{FilesystemStores, MemoryLedger};
use vpt_application::StorePaths;
use vpt_application::ports::{
    LedgerCommit, RecordingLedger, RecordingRecord, StageState, StageStates, StoreEntry,
    StoreError, Stores, TitleOrigin,
};
use vpt_domain::digest::Sha256Digest;
use vpt_domain::identity::RecordingId;
use vpt_domain::time::{UtcInstant, UtcOffset};
use vpt_protocol::error::ErrorKind;

fn record(seed: u8, stages: StageStates) -> RecordingRecord {
    let digest = Sha256Digest([seed; 32]);
    let captured_at = UtcInstant {
        secs: 1_787_604_456 + i64::from(seed),
    };
    let captured_offset = UtcOffset { secs: 0 };
    RecordingRecord {
        id: RecordingId::derive(captured_at, captured_offset, &digest).unwrap(),
        source_path: None,
        digest,
        captured_at,
        captured_offset,
        duration_secs: 1,
        title: Some(format!("title-{seed}")),
        title_source: TitleOrigin::VoiceMemos,
        ingested_at: captured_at,
        audio_path: format!("/owned/audio-{seed}.m4a").into(),
        stages,
        audio_trashed_at: None,
    }
}

struct EmptyStores;

impl Stores for EmptyStores {
    fn entries(&self, _root: &Path) -> Result<Vec<StoreEntry>, StoreError> {
        Ok(vec![])
    }
    fn validate_path(&self, _path: &Path) -> Result<(), StoreError> {
        Ok(())
    }
    fn digest_of(&self, _path: &Path) -> Result<Option<Sha256Digest>, StoreError> {
        Ok(None)
    }
    fn digest_bytes(&self, _bytes: &[u8]) -> Sha256Digest {
        Sha256Digest([0; 32])
    }
    fn publish(
        &self,
        _target: &Path,
        _expected: Option<Sha256Digest>,
        _bytes: &[u8],
    ) -> Result<(), StoreError> {
        Ok(())
    }
    fn sync_directory_of(&self, _path: &Path) -> Result<(), StoreError> {
        Ok(())
    }
}

#[test]
fn show_directly_returns_a_known_record_and_typed_missing_id() {
    let ledger = MemoryLedger::new();
    let row = record(1, StageStates::fresh());
    ledger
        .commit(&LedgerCommit {
            recordings: vec![row.clone()],
            ..Default::default()
        })
        .unwrap();
    match show::run(&ledger, row.id.as_str()) {
        Outcome::Success {
            document, human, ..
        } => {
            assert_eq!(document["id"], row.id.as_str());
            assert!(human.contains(row.id.as_str()));
        }
        _ => panic!("known recording was not shown"),
    }
    let missing = record(2, StageStates::fresh());
    match show::run(&ledger, missing.id.as_str()) {
        Outcome::Failure(error) => {
            assert_eq!(error.kind, ErrorKind::Usage);
            assert_eq!(error.ids, vec![missing.id.to_string()]);
        }
        _ => panic!("missing recording was not a usage failure"),
    }
    assert!(
        matches!(show::run(&ledger, "nonsense"), Outcome::Failure(error) if error.kind == ErrorKind::Usage)
    );
}

#[test]
fn list_directly_filters_only_succeeded_stages() {
    let ledger = MemoryLedger::new();
    let first = record(
        1,
        StageStates {
            transcribe: StageState::Succeeded,
            note: StageState::Pending,
            synthesis: StageState::Disabled,
        },
    );
    let second = record(
        2,
        StageStates {
            transcribe: StageState::Failed,
            note: StageState::Succeeded,
            synthesis: StageState::Pending,
        },
    );
    let done = record(
        3,
        StageStates {
            transcribe: StageState::Succeeded,
            note: StageState::Succeeded,
            synthesis: StageState::Succeeded,
        },
    );
    ledger
        .commit(&LedgerCommit {
            recordings: vec![first.clone(), second.clone(), done],
            ..Default::default()
        })
        .unwrap();
    for (stage, expected) in [
        (None, 3),
        (Some("transcribe"), 1),
        (Some("note"), 1),
        (Some("synthesis"), 2),
    ] {
        match list::run(&ledger, stage) {
            Outcome::Success { document, .. } => {
                assert_eq!(
                    document["recordings"].as_array().unwrap().len(),
                    expected,
                    "stage={stage:?}"
                );
            }
            _ => panic!("stage {stage:?} did not list"),
        }
    }
    assert!(
        matches!(list::run(&ledger, Some("polish")), Outcome::Failure(error) if error.kind == ErrorKind::Usage)
    );
    let expected = [second.id.as_str(), first.id.as_str()];
    for (stage, id) in [("transcribe", expected[0]), ("note", expected[1])] {
        if let Outcome::Success { document, .. } = list::run(&ledger, Some(stage)) {
            assert_eq!(document["recordings"][0]["id"], id);
        }
    }
}

#[test]
fn storage_directly_reports_named_stores_from_entries() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("a.m4a"), b"abc").unwrap();
    let root = temp.path().to_path_buf();
    let paths = StorePaths {
        audio: root.clone(),
        transcripts: root.clone(),
        analysis: root.clone(),
        briefs: root.clone(),
        engine_outputs: root.clone(),
        drafts: root.clone(),
        released: root.clone(),
    };
    let stores = FilesystemStores::open_read_only(std::slice::from_ref(&root)).unwrap();
    match storage::run(&paths, &stores) {
        Outcome::Success {
            document, human, ..
        } => {
            assert_eq!(document["stores"].as_object().unwrap().len(), 7);
            assert_eq!(document["stores"]["audio"]["files"], 1);
            assert_eq!(document["stores"]["audio"]["bytes"], 3);
            assert!(human.contains("audio"));
        }
        _ => panic!("storage inventory did not succeed"),
    }
}

#[test]
fn storage_refuses_non_utf8_root_without_lossy_output() {
    let root: std::path::PathBuf = OsString::from_vec(b"/owned/private-\xff".to_vec()).into();
    let paths = StorePaths {
        audio: root.clone(),
        transcripts: root.clone(),
        analysis: root.clone(),
        briefs: root.clone(),
        engine_outputs: root.clone(),
        drafts: root.clone(),
        released: root.clone(),
    };
    match storage::run(&paths, &EmptyStores) {
        Outcome::Failure(error) => {
            assert_eq!(error.kind, ErrorKind::Store);
            assert!(!error.message.contains("private"));
        }
        _ => panic!("unrepresentable root was not refused"),
    }
}
