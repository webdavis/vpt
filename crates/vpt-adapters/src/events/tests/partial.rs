use super::*;
use crate::{AppendEffects, AppendFailure, AppendSync, LogError, TailRepair};
use std::cell::Cell;
use std::io::ErrorKind;
use vpt_application::ports::SinkWarning;
use vpt_domain::run::{ExportFailureClass, ObservedLogFile, PartialAppend};

#[test]
fn failed_append_preserves_observed_file_and_partial_effects_without_ack() {
    let fixture = Fixture::new();
    let receipt = fixture.saved_event(EventKind::BriefWritten, EventState::Done);
    fs::write(fixture.log_path(), b"complete\nunfinished").unwrap();
    fs::set_permissions(fixture.log_path(), fs::Permissions::from_mode(0o600)).unwrap();
    let inspection = fixture.state.open_event_log().unwrap().inspect().unwrap();
    let calls = Cell::new(0);
    let mut second = 20;
    let mut sink = LoggedEvents::new(&fixture.state, &fixture.history, &[], &[], || {
        second += 1;
        at(second)
    });

    for _ in 0..2 {
        let mut warnings = BTreeSet::new();
        assert_eq!(
            sink.export_pending_with_append(Some(&receipt), &mut warnings, |_, _| {
                calls.set(calls.get() + 1);
                Err(Box::new(AppendFailure {
                    error: LogError::Io(ErrorKind::BrokenPipe),
                    effects: AppendEffects {
                        inspection: Some(inspection),
                        repair: Some(TailRepair {
                            original_end: inspection.physical_end_offset,
                            retained_end: inspection.complete_end_offset,
                            synced: true,
                        }),
                        written_bytes: 5,
                        synced: AppendSync::File,
                    },
                }))
            }),
            Ok(())
        );
        assert_eq!(warnings, BTreeSet::from([SinkWarning::ExportUnavailable]));
    }

    let view = fixture.history.recovery_view().unwrap();
    assert_eq!(view.records.len(), 2);
    assert!(
        view.records
            .iter()
            .all(|record| record.export_position.is_none())
    );
    assert_eq!(calls.get(), 2);
    assert_eq!(fixture.history.pending_exports().unwrap().len(), 2);
    assert_eq!(view.failures.len(), 2);
    assert_eq!(view.failures[0].id.sequence(), 1);
    assert_eq!(view.failures[1].id.sequence(), 2);
    assert!(view.failures[0].failure.observed_at < view.failures[1].failure.observed_at);
    for saved in &view.failures {
        assert_eq!(saved.failure.event, view.records[0].id);
        assert_eq!(saved.failure.class, ExportFailureClass::Io);
        assert_eq!(saved.failure.last_confirmed, None);
        assert_eq!(
            saved.failure.observed_file,
            Some(ObservedLogFile {
                device: inspection.device,
                inode: inspection.inode,
                complete_end_offset: inspection.complete_end_offset,
                physical_end_offset: inspection.physical_end_offset,
            })
        );
        assert_eq!(
            saved.failure.partial,
            Some(PartialAppend {
                original_end: inspection.physical_end_offset,
                retained_end: inspection.complete_end_offset,
                tail_repair_synced: true,
                bytes_written: 5,
                file_synced: true,
                directory_synced: false,
                identity_revalidated: false,
            })
        );
    }
    assert_eq!(
        fs::read(fixture.log_path()).unwrap(),
        b"complete\nunfinished"
    );
}
