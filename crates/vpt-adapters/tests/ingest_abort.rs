mod support;

use support::faults::{ArchiveFault, FaultArchive};
use support::gates::GateRecorder;
use support::{CAPTURED, Fixture};
use vpt_application::ports::{Archive, RecordingLedger, RunHistory, TrashError};
use vpt_application::{Ingest, IngestFailure, Mode};
use vpt_domain::fixtures::m4a;
use vpt_domain::history::EventKind;
use vpt_domain::run::{HistoryFact, RunOutcome};

fn populate(fixture: &Fixture) -> Vec<std::path::PathBuf> {
    ["a.m4a", "b.m4a", "c.m4a"]
        .into_iter()
        .enumerate()
        .map(|(index, name)| {
            fixture.add_recording(name, &m4a(CAPTURED + index as i64, 3, name.as_bytes()))
        })
        .collect()
}

fn assert_saved_progress(
    parts: &support::Parts,
    sources: &[std::path::PathBuf],
    completed: &[vpt_domain::identity::RecordingId],
) {
    let records = parts.ledger.recordings().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(completed, [records[0].id.clone()]);
    assert_eq!(records[0].source_path.as_ref(), Some(&sources[0]));
    assert_eq!(
        std::fs::read(&records[0].audio_path).unwrap(),
        std::fs::read(&sources[0]).unwrap()
    );
    assert_eq!(parts.ledger.seen_all().unwrap().len(), 1);
    for (index, path) in sources.iter().enumerate() {
        assert_eq!(
            std::fs::read(path).unwrap(),
            m4a(
                CAPTURED + index as i64,
                3,
                path.file_name().unwrap().as_encoded_bytes()
            )
        );
    }
    let history = parts.ledger.recovery_view().unwrap();
    assert_eq!(history.records.iter().filter(|record| matches!(&record.fact, HistoryFact::Event(event) if event.kind == EventKind::IngestFailed)).count(), 1);
    assert!(matches!(
        history.records.last().unwrap().fact,
        HistoryFact::RunFinished(RunOutcome::Failed)
    ));
    assert!(!format!("{history:?}").contains("private source failure"));
    assert!(!format!("{history:?}").contains("private sync failure"));
}

#[test]
fn unreadable_later_source_preserves_first_commit_and_stops_before_third() {
    let fixture = Fixture::new();
    let sources = populate(&fixture);
    let mut parts = fixture.parts();
    let recorder = GateRecorder::new(&fixture.recordings);
    recorder.fail_open_after.set(Some(1));
    let error = Ingest {
        recorder: &recorder,
        archive: &parts.archive,
        ledger: &parts.ledger,
        clock: &parts.clock,
        trash: &parts.trash,
        sink: &mut parts.sink,
        settings: &parts.settings,
    }
    .run(&Mode::default())
    .unwrap_err();
    assert!(matches!(error.failure, IngestFailure::StoreUnreadable(_)));
    assert_saved_progress(&parts, &sources, &error.completed);
    assert_eq!(recorder.opens.get(), 2);
    assert_eq!(recorder.clones.get(), 1);
    assert!(parts.trash.attempted.borrow().is_empty());
}

#[test]
fn later_space_and_sync_failures_keep_completed_work_and_only_clean_owned_staging() {
    for fault in [
        ArchiveFault::NoSpaceAfterFirst,
        ArchiveFault::SyncAfterFirst,
        ArchiveFault::PlacedUnsyncedAfterFirst,
    ] {
        for helper_absent in [false, true] {
            let fixture = Fixture::new();
            let sources = populate(&fixture);
            let mut parts = fixture.parts();
            let archive = FaultArchive {
                base: vpt_adapters::ClonefileArchive::open(&fixture.audio).unwrap(),
                fault,
            };
            let recorder = GateRecorder::new(&fixture.recordings);
            if helper_absent {
                *parts.trash.failure.borrow_mut() = Some(TrashError::HelperAbsent);
            }
            let error = Ingest {
                recorder: &recorder,
                archive: &archive,
                ledger: &parts.ledger,
                clock: &parts.clock,
                trash: &parts.trash,
                sink: &mut parts.sink,
                settings: &parts.settings,
            }
            .run(&Mode::default())
            .unwrap_err();
            if fault == ArchiveFault::NoSpaceAfterFirst {
                assert_eq!(error.failure, IngestFailure::NoSpace);
            } else {
                assert!(matches!(error.failure, IngestFailure::Sync(_)));
            }
            assert_saved_progress(&parts, &sources, &error.completed);
            assert_eq!(recorder.opens.get(), 2);
            assert_eq!(recorder.clones.get(), 2);
            let placed = fault == ArchiveFault::PlacedUnsyncedAfterFirst;
            assert_eq!(
                archive.archived().unwrap().len(),
                if placed { 2 } else { 1 }
            );
            assert_eq!(parts.trash.attempted.borrow().len(), usize::from(!placed));
            assert_eq!(
                archive.staged_leftovers().unwrap().len(),
                usize::from(!placed && helper_absent)
            );
            if !placed && helper_absent {
                assert!(
                    error
                        .log
                        .iter()
                        .any(|line| line.contains("helper unavailable"))
                );
            }
        }
    }
}
