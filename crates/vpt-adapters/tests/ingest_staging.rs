mod support;

use support::faults::{ArchiveFault, FaultArchive, altered_staged_bytes};
use support::{CAPTURED, Fixture};
use vpt_application::ports::{Archive, RecordingLedger, RunHistory};
use vpt_application::{Ingest, IngestFailure, Mode};
use vpt_domain::fixtures::m4a;
use vpt_domain::history::EventKind;
use vpt_domain::run::HistoryFact;

#[test]
fn staging_changed_after_hashing_is_preserved() {
    refuses_changed_staging(ArchiveFault::ChangedAfterStage);
}

#[test]
fn staging_changed_during_inspection_is_preserved() {
    refuses_changed_staging(ArchiveFault::ChangedAfterStagedOpen);
}

#[test]
fn staging_replaced_during_inspection_is_preserved() {
    refuses_changed_staging(ArchiveFault::ReplacedAfterStagedOpen);
}

#[test]
fn staging_changed_after_inspection_is_preserved() {
    refuses_changed_staging(ArchiveFault::ChangedBeforePublish);
}

#[test]
fn staging_replaced_with_identical_bytes_after_inspection_is_preserved() {
    refuses_changed_staging(ArchiveFault::ReplacedBeforePublish);
}

fn refuses_changed_staging(fault: ArchiveFault) {
    let fixture = Fixture::new();
    let original = m4a(CAPTURED, 1, b"original bytes");
    let source = fixture.add_recording("source.m4a", &original);
    let mut parts = fixture.parts();
    let archive = FaultArchive {
        base: parts.archive,
        fault,
    };
    let result = Ingest {
        recorder: &parts.store,
        archive: &archive,
        ledger: &parts.ledger,
        clock: &parts.clock,
        trash: &parts.trash,
        sink: &mut parts.sink,
        settings: &parts.settings,
    }
    .run(&Mode::default());
    let error = result.expect_err("changed staging must never become a recording");
    assert!(
        matches!(error.failure, IngestFailure::Sync(_)),
        "{fault:?}: {:?}",
        error.failure
    );
    assert!(parts.ledger.recordings().unwrap().is_empty());
    assert!(parts.ledger.seen_all().unwrap().is_empty());
    assert!(archive.archived().unwrap().is_empty());
    assert!(parts.trash.attempted.borrow().is_empty());
    let staged = archive.staged_leftovers().unwrap();
    assert_eq!(
        staged.len(),
        if matches!(
            fault,
            ArchiveFault::ReplacedAfterStagedOpen | ArchiveFault::ReplacedBeforePublish
        ) {
            2
        } else {
            1
        }
    );
    let expected = if matches!(
        fault,
        ArchiveFault::ReplacedAfterStagedOpen | ArchiveFault::ReplacedBeforePublish
    ) {
        original.clone()
    } else {
        altered_staged_bytes()
    };
    for path in &staged {
        assert_eq!(std::fs::read(path).unwrap(), expected);
    }
    assert_eq!(std::fs::read(source).unwrap(), original);
    assert!(
        error
            .log
            .iter()
            .any(|line| line.contains("staged cleanup pending"))
    );
    let history = parts.ledger.recovery_view().unwrap();
    assert_eq!(history.records.iter().filter(|record| matches!(&record.fact, HistoryFact::Event(event) if event.kind == EventKind::IngestFailed)).count(), 1);
}

#[test]
fn a_staging_file_left_after_publication_still_saves_the_recording() {
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"audio"));
    let mut parts = fixture.parts();
    let archive = FaultArchive {
        base: parts.archive,
        fault: ArchiveFault::StagingLeft,
    };
    let report = Ingest {
        recorder: &parts.store,
        archive: &archive,
        ledger: &parts.ledger,
        clock: &parts.clock,
        trash: &parts.trash,
        sink: &mut parts.sink,
        settings: &parts.settings,
    }
    .run(&Mode::default())
    .expect("the archived recording is saved");
    assert_eq!(report.ingested.len(), 1);
    assert_eq!(parts.ledger.recordings().unwrap().len(), 1);
    assert_eq!(archive.archived().unwrap().len(), 1);
    assert!(parts.trash.attempted.borrow().is_empty());
    assert!(
        report
            .log
            .iter()
            .any(|line| line.contains("staged cleanup pending"))
    );
}
