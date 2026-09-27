use super::{committed, cursor};
use vpt_application::ports::{HistoryWrite, RunHistory};
use vpt_domain::run::{ExportCursor, ExportPosition, RunKind};
use vpt_domain::time::UtcInstant;

fn old_position() -> ExportPosition {
    ExportPosition {
        device: 4,
        inode: 5,
        end_offset: 80,
    }
}

fn old_cursor() -> ExportCursor {
    ExportCursor {
        generation: 0,
        position: old_position(),
    }
}

fn recreated_position() -> ExportPosition {
    ExportPosition {
        device: 4,
        inode: 6,
        end_offset: 0,
    }
}

pub(crate) fn recreated_safe_log_replays_stable_event_ids<L: RunHistory>(ledger: &L) {
    let record = committed(ledger.begin_run(RunKind::Ingest, UtcInstant { secs: 10 }))
        .records
        .remove(0);
    committed(ledger.acknowledge_export(record.id, None, old_position(), UtcInstant { secs: 11 }));
    let observed = recreated_position();
    committed(ledger.restart_export(old_cursor(), observed, UtcInstant { secs: 12 }));
    let replay_pending = ledger.pending_exports().unwrap();
    assert_eq!(replay_pending[0].id, record.id);
    assert_eq!(replay_pending[0].occurred_at, record.occurred_at);
    let new_position = ExportPosition {
        end_offset: 90,
        ..observed
    };
    committed(ledger.acknowledge_export(
        record.id,
        Some(ExportCursor {
            generation: 1,
            position: observed,
        }),
        new_position,
        UtcInstant { secs: 13 },
    ));
    assert!(ledger.pending_exports().unwrap().is_empty());
    let replay_view = ledger.recovery_view().unwrap();
    assert_eq!(replay_view.records.len(), 1);
    assert_eq!(replay_view.records[0].id, record.id);
    assert_eq!(replay_view.records[0].occurred_at, record.occurred_at);
    assert_eq!(replay_view.records[0].export_position, Some(new_position));
    assert_eq!(
        replay_view.records[0].appended_at,
        Some(UtcInstant { secs: 13 })
    );
    assert_eq!(replay_view.restarts.len(), 1);
    assert_eq!(replay_view.restarts[0].old, old_cursor());
    assert_eq!(replay_view.restarts[0].new.position, observed);
    assert_eq!(replay_view.restarts[0].observed_at, UtcInstant { secs: 12 });
    assert_eq!(
        replay_view.active_cursor,
        Some(ExportCursor {
            generation: 1,
            position: new_position,
        })
    );
}

pub(crate) fn stale_acknowledgement_token_is_refused_after_recreated_log<L: RunHistory>(
    ledger: &L,
) {
    let record = committed(ledger.begin_run(RunKind::Ingest, UtcInstant { secs: 10 }))
        .records
        .remove(0);
    committed(ledger.acknowledge_export(record.id, None, old_position(), UtcInstant { secs: 11 }));
    let observed = recreated_position();
    committed(ledger.restart_export(old_cursor(), observed, UtcInstant { secs: 12 }));
    assert!(matches!(
        ledger.acknowledge_export(
            record.id,
            Some(old_cursor()),
            ExportPosition {
                end_offset: 90,
                ..observed
            },
            UtcInstant { secs: 13 }
        ),
        HistoryWrite::Unconfirmed(_)
    ));
    let replay_pending = ledger.pending_exports().unwrap();
    assert_eq!(replay_pending.len(), 1);
    assert_eq!(replay_pending[0].id, record.id);
    assert_eq!(replay_pending[0].occurred_at, record.occurred_at);
    let view = ledger.recovery_view().unwrap();
    assert_eq!(view.records.len(), 1);
    assert_eq!(view.records[0].id, record.id);
    assert_eq!(view.records[0].export_position, None);
    assert_eq!(view.restarts.len(), 1);
    assert_eq!(view.restarts[0].old, old_cursor());
    assert_eq!(view.restarts[0].new.position, observed);
    assert_eq!(view.restarts[0].observed_at, UtcInstant { secs: 12 });
    assert_eq!(
        view.active_cursor,
        Some(ExportCursor {
            generation: 1,
            position: observed,
        })
    );
}

pub(crate) fn identical_stale_restart_does_not_erase_replay_progress<L: RunHistory>(ledger: &L) {
    let record = committed(ledger.begin_run(RunKind::Ingest, UtcInstant { secs: 10 }))
        .records
        .remove(0);
    committed(ledger.acknowledge_export(record.id, None, old_position(), UtcInstant { secs: 11 }));
    committed(ledger.restart_export(old_cursor(), recreated_position(), UtcInstant { secs: 12 }));
    let replay_position = ExportPosition {
        end_offset: 90,
        ..recreated_position()
    };
    let replay_cursor = ExportCursor {
        generation: 1,
        position: recreated_position(),
    };
    committed(ledger.acknowledge_export(
        record.id,
        Some(replay_cursor),
        replay_position,
        UtcInstant { secs: 13 },
    ));
    committed(ledger.restart_export(old_cursor(), recreated_position(), UtcInstant { secs: 12 }));
    assert!(
        ledger.pending_exports().unwrap().is_empty(),
        "identical restart erased replay progress"
    );
    let view = ledger.recovery_view().unwrap();
    assert_eq!(view.records[0].id, record.id);
    assert_eq!(view.records[0].export_position, Some(replay_position));
    assert_eq!(view.records[0].appended_at, Some(UtcInstant { secs: 13 }));
    assert_eq!(
        view.active_cursor,
        Some(ExportCursor {
            generation: 1,
            position: replay_position
        })
    );
    assert_eq!(view.restarts.len(), 1);
    assert_eq!(view.restarts[0].observed_at, UtcInstant { secs: 12 });
}

pub(crate) fn later_stale_restart_preserves_original_observation<L: RunHistory>(ledger: &L) {
    let record = committed(ledger.begin_run(RunKind::Ingest, UtcInstant { secs: 10 }))
        .records
        .remove(0);
    committed(ledger.acknowledge_export(record.id, None, old_position(), UtcInstant { secs: 11 }));
    committed(ledger.restart_export(old_cursor(), recreated_position(), UtcInstant { secs: 12 }));
    let replay_position = ExportPosition {
        end_offset: 90,
        ..recreated_position()
    };
    let replay_cursor = ExportCursor {
        generation: 1,
        position: recreated_position(),
    };
    committed(ledger.acknowledge_export(
        record.id,
        Some(replay_cursor),
        replay_position,
        UtcInstant { secs: 13 },
    ));
    committed(ledger.restart_export(old_cursor(), recreated_position(), UtcInstant { secs: 14 }));
    assert!(
        ledger.pending_exports().unwrap().is_empty(),
        "later retry erased replay progress"
    );
    let view = ledger.recovery_view().unwrap();
    assert_eq!(view.records[0].id, record.id);
    assert_eq!(view.records[0].export_position, Some(replay_position));
    assert_eq!(view.records[0].appended_at, Some(UtcInstant { secs: 13 }));
    assert_eq!(
        view.active_cursor,
        Some(ExportCursor {
            generation: 1,
            position: replay_position
        })
    );
    assert_eq!(view.restarts.len(), 1);
    assert_eq!(view.restarts[0].observed_at, UtcInstant { secs: 12 });
}

pub(crate) fn changed_stale_restart_boundary_is_refused<L: RunHistory>(ledger: &L) {
    let record = committed(ledger.begin_run(RunKind::Ingest, UtcInstant { secs: 10 }))
        .records
        .remove(0);
    committed(ledger.acknowledge_export(record.id, None, old_position(), UtcInstant { secs: 11 }));
    committed(ledger.restart_export(old_cursor(), recreated_position(), UtcInstant { secs: 12 }));
    let replay_position = ExportPosition {
        end_offset: 90,
        ..recreated_position()
    };
    committed(ledger.acknowledge_export(
        record.id,
        Some(ExportCursor {
            generation: 1,
            position: recreated_position(),
        }),
        replay_position,
        UtcInstant { secs: 13 },
    ));
    assert!(matches!(
        ledger.restart_export(
            old_cursor(),
            ExportPosition {
                end_offset: 1,
                ..recreated_position()
            },
            UtcInstant { secs: 14 }
        ),
        HistoryWrite::Unconfirmed(_)
    ));
    assert!(ledger.pending_exports().unwrap().is_empty());
    let view = ledger.recovery_view().unwrap();
    assert_eq!(view.records.len(), 1);
    assert_eq!(view.records[0].id, record.id);
    assert_eq!(view.records[0].export_position, Some(replay_position));
    assert_eq!(view.records[0].appended_at, Some(UtcInstant { secs: 13 }));
    assert_eq!(
        view.active_cursor,
        Some(ExportCursor {
            generation: 1,
            position: replay_position,
        })
    );
    assert_eq!(view.restarts.len(), 1);
    assert_eq!(view.restarts[0].old, old_cursor());
    assert_eq!(view.restarts[0].new.position, recreated_position());
    assert_eq!(view.restarts[0].observed_at, UtcInstant { secs: 12 });
}

pub(crate) fn growing_same_file_reset_preserves_prior_ack<L: RunHistory>(ledger: &L) {
    let record = committed(ledger.begin_run(RunKind::Ingest, UtcInstant { secs: 10 }))
        .records
        .remove(0);
    committed(ledger.acknowledge_export(record.id, None, old_position(), UtcInstant { secs: 11 }));
    let old = cursor(ledger).unwrap();
    assert!(matches!(
        ledger.restart_export(
            old,
            ExportPosition {
                end_offset: 100,
                ..old_position()
            },
            UtcInstant { secs: 12 }
        ),
        HistoryWrite::Unconfirmed(_)
    ));
    assert!(ledger.pending_exports().unwrap().is_empty());
    assert!(ledger.recovery_view().unwrap().restarts.is_empty());
}

pub(crate) fn repeated_same_file_shortening_replays_again<L: RunHistory>(ledger: &L) {
    let record = committed(ledger.begin_run(RunKind::Ingest, UtcInstant { secs: 10 }))
        .records
        .remove(0);
    committed(ledger.acknowledge_export(record.id, None, old_position(), UtcInstant { secs: 11 }));
    let shortened = ExportPosition {
        end_offset: 20,
        ..old_position()
    };
    committed(ledger.restart_export(old_cursor(), shortened, UtcInstant { secs: 13 }));
    let shortened_view = ledger.recovery_view().unwrap();
    assert_eq!(shortened_view.records[0].id, record.id);
    assert!(shortened_view.records[0].appended_at.is_none());
    assert_eq!(shortened_view.restarts[0].old, old_cursor());
    assert_eq!(shortened_view.restarts[0].new.position, shortened);
    committed(ledger.acknowledge_export(
        record.id,
        shortened_view.active_cursor,
        old_position(),
        UtcInstant { secs: 14 },
    ));
    let replayed_cursor = ExportCursor {
        generation: 1,
        position: old_position(),
    };
    committed(ledger.restart_export(replayed_cursor, shortened, UtcInstant { secs: 15 }));
    let repeated_view = ledger.recovery_view().unwrap();
    assert_eq!(repeated_view.active_cursor.unwrap().generation, 2);
    assert_eq!(repeated_view.restarts[1].old.generation, 1);
    assert_eq!(ledger.pending_exports().unwrap()[0].id, record.id);
    assert_eq!(repeated_view.restarts.len(), 2);
}
