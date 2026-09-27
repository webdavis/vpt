use super::{publication, record};
use std::path::{Path, PathBuf};
use vpt_application::ports::{
    HistoryReceipt, HistoryWrite, LedgerCommit, PublicationJournal, RecordingLedger, RunHistory,
    WorkDelta,
};
use vpt_domain::run::{
    EffectKind, EffectOutcome, EffectReference, ExportCursor, HistoryFact, RunKind, WorkOutcome,
};
use vpt_domain::time::UtcInstant;

mod ack;
mod export;
mod hook;
mod recovery;
mod recovery_invalid_id;
mod replay;
pub(crate) use ack::{
    export_ack_rejects_changed_file_and_nonincreasing_offset,
    export_ack_rejects_out_of_order_event, export_ack_saves_confirmed_receipt_fields,
};
pub(crate) use export::{
    encoding_export_failure_stays_distinct_from_io,
    export_failure_retains_cursor_and_partial_repair,
    failure_after_first_replay_ack_uses_receipt_position,
    failure_immediately_after_reset_has_no_confirmed_position,
    recovery_progress_and_summary_are_saved_facts,
};
pub(crate) use hook::hook_failure_requires_event_in_same_run;
pub(crate) use recovery::{
    invalid_summary_window_rolls_back_work_delta,
    observed_log_file_round_trips_without_acknowledgement,
    recovery_progress_does_not_advance_failure_watermark,
    recovery_summaries_advance_saved_failure_window,
    saved_export_failures_have_unique_order_for_repeated_observation,
    summary_window_rejects_stale_missing_and_foreign_ids,
    work_only_recovery_summary_preserves_absent_failure_time,
};
pub(crate) use recovery_invalid_id::{
    zero_affected_run_progress_rolls_back_work,
    zero_affected_run_windowless_summary_rolls_back_work,
};
pub(crate) use replay::{
    changed_stale_restart_boundary_is_refused, growing_same_file_reset_preserves_prior_ack,
    identical_stale_restart_does_not_erase_replay_progress,
    later_stale_restart_preserves_original_observation,
    recreated_safe_log_replays_stable_event_ids, repeated_same_file_shortening_replays_again,
    stale_acknowledgement_token_is_refused_after_recreated_log,
};

fn committed<T: std::fmt::Debug>(result: HistoryWrite<T>) -> T {
    match result {
        HistoryWrite::Committed(value) => value,
        other => panic!("expected committed history: {other:?}"),
    }
}

fn step(at: i64, outcome: WorkOutcome) -> (UtcInstant, HistoryFact) {
    (
        UtcInstant { secs: at },
        HistoryFact::WorkStep {
            recording: None,
            outcome,
        },
    )
}

fn cursor<L: RunHistory>(ledger: &L) -> Option<ExportCursor> {
    ledger.recovery_view().unwrap().active_cursor
}

pub(crate) fn start_and_steps_have_stable_order_and_pending_export<L: RunHistory>(ledger: &L) {
    let started = committed(ledger.begin_run(RunKind::Ingest, UtcInstant { secs: 10 }));
    assert_eq!(started.records.len(), 1);
    assert_eq!(started.records[0].id.sequence(), 1);
    let second = committed(ledger.commit_step(
        started.run,
        &WorkDelta::default(),
        &[step(11, WorkOutcome::Completed)],
    ));
    assert_eq!(second.records.len(), 1);
    assert_eq!(second.records[0].id.sequence(), 2);
    assert_eq!(second.records[0].run, started.run);
    assert_eq!(
        ledger.pending_exports().unwrap(),
        [started.records, second.records].concat()
    );
}

pub(crate) fn source_update_and_history_commit_together<L: RunHistory + RecordingLedger>(
    ledger: &L,
) {
    let recording = record(70);
    ledger
        .commit(&LedgerCommit {
            recordings: vec![recording.clone()],
            ..Default::default()
        })
        .unwrap();
    let run = committed(ledger.begin_run(RunKind::Ingest, UtcInstant { secs: 10 })).run;
    let source = PathBuf::from("/vm/Recordings/renamed.m4a");
    let written = committed(ledger.commit_step(
        run,
        &WorkDelta {
            source_updates: vec![(recording.id.clone(), source.clone())],
            ..Default::default()
        },
        &[step(11, WorkOutcome::Completed)],
    ));
    assert_eq!(
        ledger.by_id(&recording.id).unwrap().unwrap().source_path,
        Some(source)
    );
    assert_eq!(written.records.len(), 1);

    let impossible = record(71);
    ledger
        .commit(&LedgerCommit {
            recordings: vec![impossible.clone()],
            ..Default::default()
        })
        .unwrap();
    let failure = ledger.commit_step(
        run,
        &WorkDelta {
            commit: LedgerCommit {
                recordings: vec![impossible],
                ..Default::default()
            },
            source_updates: vec![(
                recording.id.clone(),
                PathBuf::from("/vm/Recordings/should-not-appear.m4a"),
            )],
            ..Default::default()
        },
        &[step(12, WorkOutcome::Completed)],
    );
    assert!(
        matches!(failure, HistoryWrite::Unconfirmed(_)),
        "{failure:?}"
    );
    assert_eq!(
        ledger.by_id(&recording.id).unwrap().unwrap().source_path,
        Some(PathBuf::from("/vm/Recordings/renamed.m4a"))
    );
    assert_eq!(ledger.pending_exports().unwrap().len(), 2);
}

pub(crate) fn publication_clear_commits_with_history<
    L: RunHistory + RecordingLedger + PublicationJournal,
>(
    ledger: &L,
) {
    let target = PathBuf::from("/h/transcripts/clear.md");
    let publication = publication(target.to_str().unwrap(), 80, 10);
    ledger
        .commit(&LedgerCommit {
            publications: vec![publication.clone()],
            ..Default::default()
        })
        .unwrap();
    let run = committed(ledger.begin_run(RunKind::Publish, UtcInstant { secs: 10 })).run;
    let fact = (
        UtcInstant { secs: 11 },
        HistoryFact::EffectIntent {
            kind: EffectKind::Publication,
            reference: EffectReference::PublicationTarget(target.clone()),
        },
    );
    let written = committed(ledger.commit_step(
        run,
        &WorkDelta {
            publication_clears: vec![target.clone()],
            ..Default::default()
        },
        &[fact],
    ));
    assert_eq!(written.records.len(), 1);
    assert!(ledger.pending_publications().unwrap().is_empty());
}

pub(crate) fn duplicate_recording_rolls_back_publication_clear<
    L: RunHistory + RecordingLedger + PublicationJournal,
>(
    ledger: &L,
) {
    let target = PathBuf::from("/h/transcripts/clear.md");
    let publication = publication(target.to_str().unwrap(), 80, 10);
    let duplicate = record(81);
    ledger
        .commit(&LedgerCommit {
            recordings: vec![duplicate.clone()],
            publications: vec![publication.clone()],
            ..Default::default()
        })
        .unwrap();
    let run = committed(ledger.begin_run(RunKind::Publish, UtcInstant { secs: 10 })).run;
    let failure = ledger.commit_step(
        run,
        &WorkDelta {
            commit: LedgerCommit {
                recordings: vec![duplicate],
                ..Default::default()
            },
            publication_clears: vec![target],
            ..Default::default()
        },
        &[step(12, WorkOutcome::Completed)],
    );
    assert!(
        matches!(failure, HistoryWrite::Unconfirmed(_)),
        "{failure:?}"
    );
    assert_eq!(ledger.pending_publications().unwrap(), vec![publication]);
    assert_eq!(ledger.pending_exports().unwrap().len(), 1);
}

pub(crate) fn unknown_intent_stays_unresolved<L: RunHistory>(ledger: &L) {
    let run = committed(ledger.begin_run(RunKind::Publish, UtcInstant { secs: 10 })).run;
    let intent = HistoryFact::EffectIntent {
        kind: EffectKind::Publication,
        reference: EffectReference::PublicationTarget(PathBuf::from("/h/transcripts/pending.md")),
    };
    committed(ledger.commit_step(
        run,
        &WorkDelta::default(),
        &[(UtcInstant { secs: 11 }, intent.clone())],
    ));
    let view = ledger.recovery_view().unwrap();
    assert_eq!(view.records.last().unwrap().fact, intent);
    assert!(
        view.records
            .iter()
            .all(|record| !matches!(record.fact, HistoryFact::EffectObserved { .. }))
    );
}

pub(crate) fn audio_trash_marker_shares_history_commit<L: RunHistory + RecordingLedger>(
    ledger: &L,
) {
    let recording = record(90);
    ledger
        .commit(&LedgerCommit {
            recordings: vec![recording.clone()],
            ..Default::default()
        })
        .unwrap();
    let run = committed(ledger.begin_run(RunKind::Retain, UtcInstant { secs: 10 })).run;
    let at = UtcInstant { secs: 20 };
    committed(ledger.commit_step(
        run,
        &WorkDelta {
            audio_trash_marks: vec![(recording.id.clone(), at)],
            ..Default::default()
        },
        &[step(20, WorkOutcome::Completed)],
    ));
    assert_eq!(
        ledger
            .by_id(&recording.id)
            .unwrap()
            .unwrap()
            .audio_trashed_at,
        Some(at)
    );
    assert_eq!(ledger.pending_exports().unwrap().len(), 2);
}

pub(crate) fn pending_export_does_not_read_unrelated_publication<
    L: RunHistory + RecordingLedger + PublicationJournal,
>(
    ledger: &L,
) {
    let target = Path::new("/h/transcripts/other.md");
    ledger
        .commit(&LedgerCommit {
            publications: vec![publication(target.to_str().unwrap(), 91, 10)],
            ..Default::default()
        })
        .unwrap();
    assert!(ledger.pending_exports().unwrap().is_empty());
    assert_eq!(ledger.pending_publications().unwrap().len(), 1);
}

pub(crate) fn receipt_lists_only_completed_recordings<L: RunHistory>(ledger: &L) {
    let run = committed(ledger.begin_run(RunKind::Ingest, UtcInstant { secs: 10 })).run;
    let recording = record(92);
    let result: HistoryReceipt = committed(ledger.commit_step(
        run,
        &WorkDelta {
            commit: LedgerCommit {
                recordings: vec![recording.clone()],
                ..Default::default()
            },
            ..Default::default()
        },
        &[step(11, WorkOutcome::Completed)],
    ));
    assert_eq!(result.completed, vec![recording.id]);
}

pub(crate) fn invalid_intent_fact_rolls_back_completed_work_delta<
    L: RunHistory + RecordingLedger + PublicationJournal,
>(
    ledger: &L,
) {
    let recording = record(93);
    let publication = publication("/h/transcripts/pending-93.md", 93, 10);
    ledger
        .commit(&LedgerCommit {
            recordings: vec![recording.clone()],
            publications: vec![publication.clone()],
            ..Default::default()
        })
        .unwrap();
    let before = ledger.by_id(&recording.id).unwrap().unwrap().source_path;
    let run = committed(ledger.begin_run(RunKind::Publish, UtcInstant { secs: 10 })).run;
    let unknown_intent = vpt_domain::history::EventId::new(run.namespace(), 999);
    let result = ledger.commit_step(
        run,
        &WorkDelta {
            source_updates: vec![(
                recording.id.clone(),
                PathBuf::from("/vm/Recordings/changed-93.m4a"),
            )],
            publication_clears: vec![publication.target.clone()],
            ..Default::default()
        },
        &[(
            UtcInstant { secs: 11 },
            HistoryFact::EffectObserved {
                intent: unknown_intent,
                outcome: EffectOutcome::Completed,
            },
        )],
    );
    assert!(matches!(result, HistoryWrite::Unconfirmed(_)), "{result:?}");
    assert_eq!(
        ledger.by_id(&recording.id).unwrap().unwrap().source_path,
        before
    );
    assert_eq!(ledger.pending_publications().unwrap(), vec![publication]);
    assert_eq!(ledger.pending_exports().unwrap().len(), 1);
    assert!(
        ledger
            .recovery_view()
            .unwrap()
            .records
            .iter()
            .all(|record| !matches!(record.fact, HistoryFact::EffectObserved { .. }))
    );
}

pub(crate) fn duplicate_run_started_fact_rolls_back_work_delta<L: RunHistory + RecordingLedger>(
    ledger: &L,
) {
    let started = committed(ledger.begin_run(RunKind::Ingest, UtcInstant { secs: 10 }));
    let recording = record(97);
    let result = ledger.commit_step(
        started.run,
        &WorkDelta {
            commit: LedgerCommit {
                recordings: vec![recording.clone()],
                ..Default::default()
            },
            ..Default::default()
        },
        &[(
            UtcInstant { secs: 11 },
            HistoryFact::RunStarted(RunKind::Recover),
        )],
    );
    assert!(matches!(result, HistoryWrite::Unconfirmed(_)), "{result:?}");
    assert_eq!(ledger.by_id(&recording.id).unwrap(), None);
    assert_eq!(ledger.recovery_view().unwrap().records, started.records);
}

pub(crate) fn mismatched_effect_kind_and_reference_are_refused<L: RunHistory>(ledger: &L) {
    let run = committed(ledger.begin_run(RunKind::Publish, UtcInstant { secs: 10 })).run;
    let fact = HistoryFact::EffectIntent {
        kind: EffectKind::Archive,
        reference: EffectReference::PublicationTarget(PathBuf::from(
            "/h/transcripts/not-an-archive.md",
        )),
    };
    let result = ledger.commit_step(
        run,
        &WorkDelta::default(),
        &[(UtcInstant { secs: 11 }, fact)],
    );
    assert!(matches!(result, HistoryWrite::Unconfirmed(_)), "{result:?}");
    assert_eq!(ledger.recovery_view().unwrap().records.len(), 1);
}
