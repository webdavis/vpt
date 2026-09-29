pub(crate) mod history;
pub(crate) mod journal;
pub(crate) mod recording;
pub(crate) mod retention;
pub(crate) mod retention_history;

use std::path::{Path, PathBuf};
use vpt_application::ports::{
    DirtyPublication, LedgerCommit, RecordingRecord, SeenRow, StageStates, TitleOrigin,
};
use vpt_domain::digest::Sha256Digest;
use vpt_domain::identity::RecordingId;
use vpt_domain::time::{FileTime, UtcInstant, UtcOffset};

pub(crate) fn digest(seed: u8) -> Sha256Digest {
    Sha256Digest([seed; 32])
}

pub(crate) fn seen_row(path: &str) -> SeenRow {
    SeenRow {
        path: PathBuf::from(path),
        file_name: Path::new(path)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned(),
        size: 1_024,
        mtime: FileTime {
            secs: 1_787_604_456,
            nanos: 17,
        },
        flags: 0,
        first_seen: UtcInstant {
            secs: 1_787_700_000,
        },
        last_seen: UtcInstant {
            secs: 1_787_700_000,
        },
        deferral_count: 0,
        deferral_reason: None,
        deferred_size: None,
        source_gone_at: None,
        recording: None,
    }
}

pub(crate) fn record_at(seed: u8, captured_at: UtcInstant) -> RecordingRecord {
    let digest = digest(seed);
    let captured_offset = UtcOffset { secs: -21_600 };
    RecordingRecord {
        id: RecordingId::derive(captured_at, captured_offset, &digest).unwrap(),
        source_path: Some(PathBuf::from(format!("/vm/Recordings/{seed}.m4a"))),
        digest,
        captured_at,
        captured_offset,
        duration_secs: 612,
        title: Some("Invoice call".into()),
        title_source: TitleOrigin::VoiceMemos,
        ingested_at: UtcInstant {
            secs: 1_787_700_000,
        },
        audio_path: PathBuf::from(format!("/h/audio/{seed}.m4a")),
        stages: StageStates::fresh(),
        audio_trashed_at: None,
    }
}

pub(crate) fn record(seed: u8) -> RecordingRecord {
    record_at(
        seed,
        UtcInstant {
            secs: 1_787_604_456 + i64::from(seed),
        },
    )
}

pub(crate) fn publication(target: &str, seed: u8, at: i64) -> DirtyPublication {
    DirtyPublication {
        target: PathBuf::from(target),
        expected_previous: None,
        intended: digest(seed),
        recorded_at: UtcInstant { secs: at },
    }
}

pub(crate) fn ingest(recording: RecordingRecord, seen: SeenRow) -> LedgerCommit {
    LedgerCommit {
        recordings: vec![recording],
        seen: vec![seen],
        publications: vec![],
    }
}

macro_rules! recording_cases {
    ($case:ident) => {
        $case!(seen_is_absent_until_recorded_and_updated);
        $case!(second_record_preserves_first_seen);
        $case!(seen_all_is_ordered_by_path);
        $case!(commit_writes_recording_seen_and_publication);
        $case!(stored_conflict_rolls_back_earlier_rows);
        $case!(within_batch_conflict_rolls_back_earlier_rows);
        $case!(source_path_and_audio_trashed_are_updatable);
        $case!(unknown_identity_reads_as_absent);
        $case!(recording_without_source_or_seen_commits);
        $case!(recordings_order_by_capture_then_identity);
        $case!(different_stage_values_round_trip);
        $case!(repeated_seen_and_clear_are_idempotent);
        $case!(commit_seen_replacement_preserves_first_seen);
    };
}
pub(crate) use recording_cases;

macro_rules! journal_cases {
    ($case:ident) => {
        $case!(pending_publication_lists_until_cleared);
        $case!(same_target_keeps_latest_entry);
        $case!(publications_order_by_instant_then_target);
    };
}
pub(crate) use journal_cases;

macro_rules! retention_cases {
    ($case:ident) => {
        $case!(a_recorded_intent_is_pending_with_its_fields);
        $case!(completing_an_intent_removes_it_and_keeps_the_others_in_order);
        $case!(repeated_and_unknown_completion_change_no_pending_intents);
    };
}
pub(crate) use retention_cases;

macro_rules! retention_history_cases {
    ($case:ident) => {
        $case!(generated_intent_and_effect_are_one_confirmed_receipt);
        $case!(unknown_run_preparation_leaves_no_journal_or_history);
        $case!(non_utf8_preparation_leaves_no_journal_or_history);
        $case!(completion_couples_audio_mark_journal_and_confirmed_facts);
        $case!(repeated_completion_preserves_original_time_and_history);
        $case!(recovery_run_observes_original_effect_without_minting_an_intent);
        $case!(wrong_effect_refuses_without_audio_journal_or_history_change);
        $case!(unrelated_valid_effect_refuses_without_audio_journal_or_history_change);
        $case!(wrong_intent_path_refuses_without_audio_journal_or_history_change);
        $case!(wrong_intent_digest_refuses_without_audio_journal_or_history_change);
        $case!(missing_recording_refuses_completion_and_preserves_original_intent);
        $case!(two_pending_intents_preserve_the_first_audio_mark_and_repeat_history);
        $case!(two_pending_intents_preserve_first_audio_mark_when_clock_moves_backward);
    };
}
pub(crate) use retention_history_cases;

macro_rules! history_cases {
    ($case:ident) => {
        $case!(start_and_steps_have_stable_order_and_pending_export);
        $case!(source_update_and_history_commit_together);
        $case!(publication_clear_commits_with_history);
        $case!(duplicate_recording_rolls_back_publication_clear);
        $case!(unknown_intent_stays_unresolved);
        $case!(audio_trash_marker_shares_history_commit);
        $case!(pending_export_does_not_read_unrelated_publication);
        $case!(receipt_lists_only_completed_recordings);
        $case!(invalid_intent_fact_rolls_back_completed_work_delta);
        $case!(duplicate_run_started_fact_rolls_back_work_delta);
        $case!(mismatched_effect_kind_and_reference_are_refused);
        $case!(export_ack_rejects_out_of_order_event);
        $case!(export_ack_rejects_changed_file_and_nonincreasing_offset);
        $case!(export_ack_saves_confirmed_receipt_fields);
        $case!(export_failure_retains_cursor_and_partial_repair);
        $case!(encoding_export_failure_stays_distinct_from_io);
        $case!(failure_immediately_after_reset_has_no_confirmed_position);
        $case!(failure_after_first_replay_ack_uses_receipt_position);
        $case!(recovery_progress_and_summary_are_saved_facts);
        $case!(recreated_safe_log_replays_stable_event_ids);
        $case!(stale_acknowledgement_token_is_refused_after_recreated_log);
        $case!(identical_stale_restart_does_not_erase_replay_progress);
        $case!(later_stale_restart_preserves_original_observation);
        $case!(changed_stale_restart_boundary_is_refused);
        $case!(growing_same_file_reset_preserves_prior_ack);
        $case!(repeated_same_file_shortening_replays_again);
        $case!(saved_export_failures_have_unique_order_for_repeated_observation);
        $case!(observed_log_file_round_trips_without_acknowledgement);
        $case!(recovery_progress_does_not_advance_failure_watermark);
        $case!(recovery_summaries_advance_saved_failure_window);
        $case!(summary_window_rejects_stale_missing_and_foreign_ids);
        $case!(work_only_recovery_summary_preserves_absent_failure_time);
        $case!(zero_affected_run_progress_rolls_back_work);
        $case!(zero_affected_run_windowless_summary_rolls_back_work);
        $case!(positive_missing_retention_id_is_saved);
        $case!(zero_missing_retention_id_rolls_back_work);
        $case!(invalid_summary_window_rolls_back_work_delta);
        $case!(hook_failure_requires_event_in_same_run);
    };
}
pub(crate) use history_cases;

pub(crate) fn run_sqlite(scenario: &str, ledger: &crate::ledger::SqliteLedger) {
    macro_rules! run_recording {
        ($name:ident) => {
            if scenario == concat!("ledger_contract_recording_", stringify!($name)) {
                recording::$name(ledger);
                return;
            }
        };
    }
    macro_rules! run_journal {
        ($name:ident) => {
            if scenario == concat!("ledger_contract_journal_", stringify!($name)) {
                journal::$name(ledger);
                return;
            }
        };
    }
    macro_rules! run_retention {
        ($name:ident) => {
            if scenario == concat!("ledger_contract_retention_", stringify!($name)) {
                retention::$name(ledger);
                return;
            }
        };
    }
    macro_rules! run_history {
        ($name:ident) => {
            if scenario == concat!("ledger_contract_history_", stringify!($name)) {
                history::$name(ledger);
                return;
            }
        };
    }
    macro_rules! run_retention_history {
        ($name:ident) => {
            if scenario == concat!("ledger_contract_history_retention_", stringify!($name)) {
                retention_history::$name(ledger);
                return;
            }
        };
    }
    recording_cases!(run_recording);
    journal_cases!(run_journal);
    retention_cases!(run_retention);
    history_cases!(run_history);
    retention_history_cases!(run_retention_history);
    panic!("unknown ledger contract scenario");
}
