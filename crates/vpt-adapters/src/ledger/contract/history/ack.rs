use super::{committed, cursor, step};
use vpt_application::ports::{HistoryWrite, RunHistory, WorkDelta};
use vpt_domain::run::{ExportPosition, RunKind, WorkOutcome};
use vpt_domain::time::UtcInstant;

fn position(end_offset: u64) -> ExportPosition {
    ExportPosition {
        device: 4,
        inode: 5,
        end_offset,
    }
}

pub(crate) fn export_ack_rejects_out_of_order_event<L: RunHistory>(ledger: &L) {
    let started = committed(ledger.begin_run(RunKind::Ingest, UtcInstant { secs: 10 }));
    let second = committed(ledger.commit_step(
        started.run,
        &WorkDelta::default(),
        &[step(11, WorkOutcome::Completed)],
    ))
    .records[0]
        .id;
    let wrong_file = ExportPosition {
        inode: 6,
        ..position(120)
    };
    assert!(matches!(
        ledger.acknowledge_export(second, None, wrong_file, UtcInstant { secs: 12 }),
        HistoryWrite::Unconfirmed(_)
    ));
    assert_eq!(ledger.pending_exports().unwrap().len(), 2);
}

pub(crate) fn export_ack_rejects_changed_file_and_nonincreasing_offset<L: RunHistory>(ledger: &L) {
    let started = committed(ledger.begin_run(RunKind::Ingest, UtcInstant { secs: 10 }));
    let second = committed(ledger.commit_step(
        started.run,
        &WorkDelta::default(),
        &[step(11, WorkOutcome::Completed)],
    ))
    .records[0]
        .id;
    committed(ledger.acknowledge_export(
        started.records[0].id,
        None,
        position(80),
        UtcInstant { secs: 12 },
    ));
    let expected = cursor(ledger);
    assert_eq!(expected.unwrap().generation, 0);
    let wrong_file = ExportPosition {
        inode: 6,
        ..position(120)
    };
    assert!(matches!(
        ledger.acknowledge_export(second, expected, wrong_file, UtcInstant { secs: 13 }),
        HistoryWrite::Unconfirmed(_)
    ));
    assert!(matches!(
        ledger.acknowledge_export(second, expected, position(79), UtcInstant { secs: 13 }),
        HistoryWrite::Unconfirmed(_)
    ));
    assert_eq!(ledger.pending_exports().unwrap().len(), 1);
}

pub(crate) fn export_ack_saves_confirmed_receipt_fields<L: RunHistory>(ledger: &L) {
    let started = committed(ledger.begin_run(RunKind::Ingest, UtcInstant { secs: 10 }));
    let second = committed(ledger.commit_step(
        started.run,
        &WorkDelta::default(),
        &[step(11, WorkOutcome::Completed)],
    ))
    .records[0]
        .id;
    let first_position = position(80);
    committed(ledger.acknowledge_export(
        started.records[0].id,
        None,
        first_position,
        UtcInstant { secs: 12 },
    ));
    let expected = cursor(ledger);
    committed(ledger.acknowledge_export(second, expected, position(120), UtcInstant { secs: 13 }));
    assert!(ledger.pending_exports().unwrap().is_empty());
    let records = ledger.recovery_view().unwrap().records;
    assert_eq!(records[0].export_position, Some(first_position));
    assert_eq!(records[0].appended_at, Some(UtcInstant { secs: 12 }));
    assert_eq!(records[1].export_position.unwrap().end_offset, 120);
}
