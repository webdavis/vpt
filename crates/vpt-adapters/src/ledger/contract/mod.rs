pub(crate) mod journal;
pub(crate) mod recording;
pub(crate) mod retention;

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
    recording_cases!(run_recording);
    journal_cases!(run_journal);
    retention_cases!(run_retention);
    panic!("unknown ledger contract scenario");
}
