use super::*;
use std::{fs, os::unix::fs::PermissionsExt};
use vpt_adapters::{FilesystemStores, RootDir, SqliteLedger};
use vpt_application::{Settings, ports::*};
use vpt_domain::{
    digest::Sha256Digest,
    identity::RecordingId,
    note::NoteStage,
    run::{HistoryFact, RunKind},
    time::{UtcInstant, UtcOffset},
};

fn committed<T: std::fmt::Debug>(write: HistoryWrite<T>) -> T {
    match write {
        HistoryWrite::Committed(value) => value,
        other => panic!("{other:?}"),
    }
}
fn fixture() -> (tempfile::TempDir, Settings, Roots, SqliteLedger) {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let mut settings = vpt_adapters::config::from_table(
        &vpt_adapters::config::load_text("config_version=1").unwrap(),
        &base,
    )
    .unwrap();
    settings.home = base.join("home");
    settings.state_dir = base.join("state");
    settings.source.recordings_dir = base.join("memos/Recordings");
    for key in vpt_domain::layout::StoreKey::all() {
        settings
            .stores
            .set(key, settings.home.join(key.default_leaf()));
    }
    let roots = vpt_adapters::config::resolve(&settings, &base.join("config")).unwrap();
    roots.create_leaves().unwrap();
    roots.create_state_dir().unwrap();
    let ledger = SqliteLedger::open(&RootDir::open(&roots.state_dir).unwrap()).unwrap();
    (temp, settings, roots, ledger)
}
fn prepare(ledger: &SqliteLedger, roots: &Roots, sequence: u8) -> ReleasePayload {
    let at = UtcInstant {
        secs: 1_787_604_456,
    };
    let offset = UtcOffset { secs: -21_600 };
    let digest = Sha256Digest([sequence; 32]);
    let id = RecordingId::derive(at, offset, &digest).unwrap();
    ledger
        .commit(&LedgerCommit {
            recordings: vec![RecordingRecord {
                id: id.clone(),
                digest,
                captured_at: at,
                captured_offset: offset,
                duration_secs: 12,
                title: Some("Private title".into()),
                title_source: TitleOrigin::VoiceMemos,
                source_path: Some(roots.recordings_dir.join(format!("{id}.m4a"))),
                ingested_at: at,
                audio_path: roots.stores.audio.join(format!("{id}.m4a")),
                stages: StageStates::fresh(),
                audio_trashed_at: None,
            }],
            ..Default::default()
        })
        .unwrap();
    let run = committed(ledger.begin_run(RunKind::Redact, at)).run;
    let text = format!("Public extract {sequence}\n");
    let stores = FilesystemStores::open(&[]).unwrap();
    let report = serde_json::json!({"schema":"vpt.redaction-report/1",
        "private":"Private mask mapping", "source_artifact_sha256":digest.hex(),
        "released_content_sha256":stores.digest_bytes(text.as_bytes()).hex()})
    .to_string();
    let row = committed(
        ledger.prepare_release(
            run,
            &NewRelease {
                recording: id,
                stage: NoteStage::Transcript,
                destination: roots
                    .home
                    .parent()
                    .unwrap()
                    .join(format!("external/{sequence}.md")),
                report_root: roots.stores.drafts.clone(),
                source_digest: digest,
                text,
                report,
                provenance: ReleaseProvenance {
                    captured_at: at,
                    captured_offset: offset,
                    engines: vec!["apple:SpeechTranscriber".into()],
                    open_lexical_flags: 0,
                    open_flags: 0,
                    diagnostics: vec![],
                    reviewed: true,
                },
                created_at: at,
            },
        ),
    );
    ledger
        .release_payload(row.release.sequence)
        .unwrap()
        .unwrap()
}
#[derive(Default)]
struct Sink {
    records: Vec<HistoryFact>,
    stop_at_completion: bool,
}
impl EventSink for Sink {
    fn after_commit(&mut self, receipt: &HistoryReceipt) -> SinkOutcome {
        self.records
            .extend(receipt.records.iter().map(|r| r.fact.clone()));
        if self.stop_at_completion && !receipt.completed.is_empty() {
            return SinkOutcome::Stop(Box::new(SinkStop {
                reason: SinkError::UnsafeState,
                committed: Some(receipt.clone()),
                warnings: BTreeSet::new(),
            }));
        }
        SinkOutcome::Continue {
            warnings: BTreeSet::new(),
        }
    }
    fn recover(&mut self, _: &vpt_domain::run::RecoveryView) -> SinkOutcome {
        unreachable!()
    }
}
#[test]
fn release_recovery_uses_saved_override_and_finalizes_only_after_both_files_exist() {
    let (_temp, _settings, roots, ledger) = fixture();
    let payload = prepare(&ledger, &roots, 1);
    let mut sink = Sink::default();
    let completed = repair(&ledger, &roots, &mut sink, &mut BTreeSet::new()).unwrap();
    assert_eq!(completed, [payload.release.recording.to_string()]);
    assert_eq!(
        fs::read_to_string(&payload.release.destination).unwrap(),
        payload.text
    );
    assert_eq!(
        fs::read_to_string(&payload.release.report_path).unwrap(),
        payload.report
    );
    assert_eq!(
        fs::metadata(&payload.release.report_path)
            .unwrap()
            .permissions()
            .mode()
            & 0o7777,
        0o600
    );
    assert_eq!(
        ledger.releases(&payload.release.recording).unwrap(),
        [payload.release.sequence]
    );
    assert!(ledger.pending_releases().unwrap().is_empty());
    assert!(
        !sink
            .records
            .iter()
            .any(|f| matches!(f, HistoryFact::Event(_)))
    );
    let count = sink.records.len();
    assert!(
        repair(&ledger, &roots, &mut sink, &mut BTreeSet::new())
            .unwrap()
            .is_empty()
    );
    assert_eq!(sink.records.len(), count);
}
#[test]
fn release_recovery_preserves_completed_ids_when_later_copy_was_replaced() {
    let (_temp, _settings, roots, ledger) = fixture();
    let first = prepare(&ledger, &roots, 1);
    let second = prepare(&ledger, &roots, 2);
    fs::create_dir_all(second.release.destination.parent().unwrap()).unwrap();
    fs::write(&second.release.destination, b"operator replacement").unwrap();
    let error = repair(&ledger, &roots, &mut Sink::default(), &mut BTreeSet::new()).unwrap_err();
    assert_eq!(error.rule.as_deref(), Some("publication_target_modified"));
    assert_eq!(error.completed, [first.release.recording.to_string()]);
    assert_eq!(
        ledger.pending_releases().unwrap(),
        [second.release.sequence]
    );
    assert!(!second.release.report_path.exists());
    assert!(!format!("{error:?}").contains("Private"));
}
#[test]
fn release_recovery_sink_stop_keeps_durable_completion_in_error() {
    let (_temp, _settings, roots, ledger) = fixture();
    let payload = prepare(&ledger, &roots, 1);
    let mut sink = Sink {
        stop_at_completion: true,
        ..Default::default()
    };
    let error = repair(&ledger, &roots, &mut sink, &mut BTreeSet::new()).unwrap_err();
    assert_eq!(error.completed, [payload.release.recording.to_string()]);
    assert!(ledger.pending_releases().unwrap().is_empty());
    assert!(payload.release.destination.exists());
    assert!(payload.release.report_path.exists());
}

#[test]
fn mutating_startup_repairs_releases_before_retention_and_reports_completed_recordings() {
    let (_temp, settings, roots, ledger) = fixture();
    let payload = prepare(&ledger, &roots, 1);
    let report = crate::compose::startup::repair(
        &ledger,
        &mut Sink::default(),
        &vpt_adapters::HelperClient::new(settings.helper_path.clone()),
        &settings,
        &roots,
        &mut BTreeSet::new(),
    )
    .unwrap();
    assert_eq!(report.publications, [payload.release.recording.to_string()]);
    assert!(ledger.pending_releases().unwrap().is_empty());
    assert_eq!(
        fs::read_to_string(&payload.release.report_path).unwrap(),
        payload.report
    );
}
