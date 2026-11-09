use super::optional::{AckFault, FaultHistory};
use super::*;
use std::cell::Cell;
use std::fs::OpenOptions;
use vpt_application::ports::SinkWarning;
use vpt_domain::history::EventPathKey;
use vpt_domain::run::RunOutcome;

fn finish_run(fixture: &Fixture, receipt: &HistoryReceipt) {
    committed(fixture.history.commit_step(
        receipt.run,
        &WorkDelta::default(),
        &[(at(12), HistoryFact::RunFinished(RunOutcome::Completed))],
    ));
}

#[test]
fn replay_preserves_ids_sequence_and_occurrence_times_without_hooks() {
    let fixture = Fixture::new();
    let script = fixture.path("replay-hook");
    write_hook(&script);
    let capture = fixture.path("replay-capture");
    let report = command(&script, &capture, &fixture.log_path(), "ok", &[]);
    let event = Event {
        kind: EventKind::BriefWritten,
        state: EventState::Done,
        occasion: None,
        recording: None,
        detail: EventDetail::None,
        counts: EventCounts::default(),
        paths: vec![EventPath {
            key: EventPathKey::Brief,
            path: fixture.path("brief\nname"),
        }],
        occurred_at: at(11),
    };
    let receipt = fixture.save_event(event);
    finish_run(&fixture, &receipt);
    let before = fixture.history.recovery_view().unwrap();

    assert_eq!(
        sink(&fixture, &report, &[]).recover(&before),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    let after = fixture.history.recovery_view().unwrap();
    assert_eq!(after.records.len(), before.records.len());
    for (old, new) in before.records.iter().zip(&after.records) {
        assert_eq!(new.id, old.id);
        assert_eq!(new.run, old.run);
        assert_eq!(new.occurred_at, old.occurred_at);
        assert_eq!(new.fact, old.fact);
        assert!(new.export_position.is_some());
    }
    assert_eq!(after.records[1].id, receipt.records[0].id);
    assert_eq!(fixture.history.pending_exports().unwrap(), []);
    assert!(!capture.with_extension("calls").exists());
    let bytes = fs::read(fixture.log_path()).unwrap();
    assert_eq!(bytes.iter().filter(|byte| **byte == b'\n').count(), 3);
    let documents = fixture.documents();
    assert_eq!(documents[1]["event_id"], receipt.records[0].id.render());
    assert_eq!(documents[1]["occurred_at"], "1970-01-01T00:00:11Z");
    assert_eq!(
        documents[1]["fact"]["event"]["paths"]["brief"],
        fixture.path("brief\nname").to_str().unwrap()
    );
}

#[test]
fn partial_tail_and_append_before_ack_resume_with_recognizable_duplicates() {
    let fixture = Fixture::new();
    let script = fixture.path("unconfirmed-hook");
    write_hook(&script);
    let capture = fixture.path("unconfirmed-capture");
    let report = command(&script, &capture, &fixture.log_path(), "ok", &[]);
    let receipt = fixture.saved_event(EventKind::BriefWritten, EventState::Done);
    finish_run(&fixture, &receipt);
    let history = FaultHistory {
        inner: &fixture.history,
        state_path: fixture.state.path().into(),
        fault: AckFault::Unconfirmed,
        invalidate_state: false,
        ack_attempts: Cell::new(0),
    };
    let mut first = LoggedEvents::new(&fixture.state, &history, &report, &[], || at(20));
    assert_eq!(
        first.after_commit(&receipt),
        SinkOutcome::Continue {
            warnings: BTreeSet::from([SinkWarning::ExportProgressUnconfirmed])
        }
    );
    assert_eq!(history.ack_attempts.get(), 1);
    assert_eq!(fs::read(capture.with_extension("calls")).unwrap(), b"x");
    let original = fixture.documents();
    assert_eq!(original.len(), 1);
    let mut file = OpenOptions::new()
        .append(true)
        .open(fixture.log_path())
        .unwrap();
    use std::io::Write;
    file.write_all(b"{unfinished").unwrap();
    file.sync_all().unwrap();
    let incomplete_length = fs::metadata(fixture.log_path()).unwrap().len();

    let view = fixture.history.recovery_view().unwrap();
    assert_eq!(
        sink(&fixture, &report, &[]).recover(&view),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    assert_eq!(fs::read(capture.with_extension("calls")).unwrap(), b"x");
    assert!(
        fs::read(fixture.log_path()).unwrap().ends_with(b"\n"),
        "recovery did not complete the line after the partial tail"
    );
    let documents = fixture.documents();
    assert_eq!(documents.len(), 4);
    assert_eq!(documents[0]["event_id"], documents[1]["event_id"]);
    assert_eq!(documents[1]["event_id"], view.records[0].id.render());
    assert_eq!(documents[2]["event_id"], receipt.records[0].id.render());
    assert_eq!(documents[3]["fact"]["tag"], "run_finished");
    assert_eq!(documents[0]["occurred_at"], documents[1]["occurred_at"]);
    assert!(
        !fs::read(fixture.log_path())
            .unwrap()
            .windows(11)
            .any(|part| part == b"{unfinished")
    );
    assert!(incomplete_length > original[0].to_string().len() as u64);
    assert!(fixture.history.pending_exports().unwrap().is_empty());
}

#[test]
fn validated_replacement_or_truncation_restarts_export_generation() {
    for replace in [true, false] {
        let fixture = Fixture::new();
        let receipt = fixture.saved_event(EventKind::BriefWritten, EventState::Done);
        finish_run(&fixture, &receipt);
        assert_eq!(
            sink(&fixture, &[], &[]).after_commit(&receipt),
            SinkOutcome::Continue {
                warnings: BTreeSet::new()
            }
        );
        let before = fixture.history.recovery_view().unwrap();
        let old_cursor = before.active_cursor.unwrap();
        assert!(fixture.history.pending_exports().unwrap().is_empty());
        if replace {
            let complete_prefix = fs::read(fixture.log_path()).unwrap();
            assert_eq!(complete_prefix.len() as u64, old_cursor.position.end_offset);
            fs::rename(fixture.log_path(), fixture.path("retained-old-log")).unwrap();
            fs::write(fixture.log_path(), complete_prefix).unwrap();
            fs::set_permissions(fixture.log_path(), fs::Permissions::from_mode(0o600)).unwrap();
        } else {
            OpenOptions::new()
                .write(true)
                .open(fixture.log_path())
                .unwrap()
                .set_len(0)
                .unwrap();
        }

        assert_eq!(
            sink(&fixture, &[], &[]).recover(&before),
            SinkOutcome::Continue {
                warnings: BTreeSet::new()
            }
        );
        let after = fixture.history.recovery_view().unwrap();
        assert_eq!(after.restarts.len(), 1, "replace={replace}");
        assert_eq!(after.restarts[0].old, old_cursor);
        assert_eq!(after.restarts[0].new.generation, 1);
        assert_eq!(
            after.restarts[0].new.position.end_offset,
            if replace {
                old_cursor.position.end_offset
            } else {
                0
            }
        );
        assert_eq!(after.restarts[0].observed_at, at(20));
        assert_eq!(after.active_cursor.unwrap().generation, 1);
        assert!(
            after
                .records
                .iter()
                .all(|record| record.export_position.is_some())
        );
        assert_eq!(
            after
                .records
                .iter()
                .map(|record| record.id)
                .collect::<Vec<_>>(),
            before
                .records
                .iter()
                .map(|record| record.id)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            after
                .records
                .iter()
                .map(|record| record.occurred_at)
                .collect::<Vec<_>>(),
            before
                .records
                .iter()
                .map(|record| record.occurred_at)
                .collect::<Vec<_>>()
        );
        let documents = fixture.documents();
        assert_eq!(
            documents.len(),
            before.records.len() * if replace { 2 } else { 1 }
        );
        if replace {
            for (old, replayed) in documents[..before.records.len()]
                .iter()
                .zip(&documents[before.records.len()..])
            {
                assert_eq!(old["event_id"], replayed["event_id"]);
                assert_eq!(old["occurred_at"], replayed["occurred_at"]);
            }
        }
        assert!(fixture.history.pending_exports().unwrap().is_empty());
    }
}

#[test]
fn unsafe_log_never_resets_acknowledgements() {
    let fixture = Fixture::new();
    let receipt = fixture.saved_event(EventKind::BriefWritten, EventState::Done);
    finish_run(&fixture, &receipt);
    assert_eq!(
        sink(&fixture, &[], &[]).after_commit(&receipt),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    let before = fixture.history.recovery_view().unwrap();
    fs::set_permissions(fixture.log_path(), fs::Permissions::from_mode(0o644)).unwrap();

    assert_eq!(
        sink(&fixture, &[], &[]).recover(&before),
        SinkOutcome::Continue {
            warnings: BTreeSet::from([SinkWarning::ExportUnavailable])
        }
    );
    let after = fixture.history.recovery_view().unwrap();
    assert_eq!(after.active_cursor, before.active_cursor);
    assert_eq!(after.records, before.records);
    assert!(after.restarts.is_empty());
    assert!(fixture.history.pending_exports().unwrap().is_empty());
    assert!(after.failures.is_empty());
}
