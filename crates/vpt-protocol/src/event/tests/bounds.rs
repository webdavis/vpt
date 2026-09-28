use super::*;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;
use vpt_domain::history::EventPathKey;
use vpt_domain::run::{EffectKind, EffectReference, RecoveryEvidence, RunKind};
use vpt_domain::time::{Civil, UtcOffset};

fn invalid_path() -> PathBuf {
    PathBuf::from(OsString::from_vec(vec![
        b'/', b's', b'a', b'f', b'e', b'/', 0xff,
    ]))
}

fn instant(year: i64, month: u32, day: u32, hour: u32, minute: u32, second: u32) -> UtcInstant {
    Civil {
        year,
        month,
        day,
        hour,
        minute,
        second,
    }
    .instant(UtcOffset { secs: 0 })
    .unwrap()
}

fn recovery() -> RecoveryEvidence {
    RecoveryEvidence {
        affected_runs: vec![],
        first_failure_observed_at: None,
        logging_restored_observed_at: None,
        failures: 0,
        recovered_records: 0,
        completed_work: 0,
        failed_work: 0,
        unknown_work: 0,
        deferred_work: 0,
        completed_effects: 0,
        failed_effects: 0,
        unresolved_effects: 0,
        missing_history: 0,
    }
}

#[test]
fn non_utf8_paths_return_typed_encoding_refusals_without_lossy_output() {
    for key in [
        EventPathKey::Note,
        EventPathKey::Audio,
        EventPathKey::Brief,
        EventPathKey::Archive,
    ] {
        let mut emitted = event(EventKind::ReviewNeeded, EventState::NeedsAttention);
        emitted.paths.push(EventPath {
            key,
            path: invalid_path(),
        });
        assert_eq!(
            encode_hook(&emitted),
            Err(EventEncodeError::NonUtf8Path),
            "{:?}",
            key
        );
        emitted.occurred_at = UtcInstant { secs: 11 };
        assert_eq!(
            encode_history(
                &record(HistoryFact::Event(emitted)),
                UtcInstant { secs: 12 }
            ),
            Err(EventEncodeError::NonUtf8Path),
            "{:?}",
            key
        );
    }

    for (kind, reference) in [
        (
            EffectKind::Archive,
            EffectReference::ArchiveTarget(invalid_path()),
        ),
        (
            EffectKind::Publication,
            EffectReference::PublicationTarget(invalid_path()),
        ),
        (
            EffectKind::AudioTrash,
            EffectReference::AudioTrash {
                recording: recording(),
                path: invalid_path(),
            },
        ),
        (
            EffectKind::RetentionTrash,
            EffectReference::RetentionIntent {
                id: 7,
                path: invalid_path(),
            },
        ),
    ] {
        let fact = HistoryFact::EffectIntent { kind, reference };
        assert_eq!(
            encode_history(&record(fact), UtcInstant { secs: 12 }),
            Err(EventEncodeError::NonUtf8Path),
            "{:?}",
            kind
        );
    }
}

#[test]
fn utc_wire_year_bounds_apply_to_occurrence_append_and_recovery_times() {
    for (at, expected) in [
        (instant(0, 1, 1, 0, 0, 0), "0000-01-01T00:00:00Z"),
        (instant(9999, 12, 31, 23, 59, 59), "9999-12-31T23:59:59Z"),
    ] {
        let mut emitted = event(EventKind::BriefWritten, EventState::Done);
        emitted.occurred_at = at;
        let hook: serde_json::Value =
            serde_json::from_slice(&encode_hook(&emitted).unwrap()).unwrap();
        assert_eq!(hook["occurred_at"], expected);

        let mut saved = record(HistoryFact::RunStarted(RunKind::Ingest));
        saved.occurred_at = at;
        let line: serde_json::Value =
            serde_json::from_str(&encode_history(&saved, at).unwrap()).unwrap();
        assert_eq!(line["occurred_at"], expected);
        assert_eq!(line["appended_at"], expected);
    }

    let below = UtcInstant {
        secs: instant(0, 1, 1, 0, 0, 0).secs - 1,
    };
    let above = UtcInstant {
        secs: instant(9999, 12, 31, 23, 59, 59).secs + 1,
    };
    for invalid in [
        below,
        above,
        UtcInstant { secs: i64::MIN },
        UtcInstant { secs: i64::MAX },
    ] {
        let mut emitted = event(EventKind::BriefWritten, EventState::Done);
        emitted.occurred_at = invalid;
        assert_eq!(
            encode_hook(&emitted),
            Err(EventEncodeError::TimestampOutOfRange)
        );

        let mut saved = record(HistoryFact::RunStarted(RunKind::Ingest));
        saved.occurred_at = invalid;
        assert_eq!(
            encode_history(&saved, UtcInstant { secs: 12 }),
            Err(EventEncodeError::TimestampOutOfRange)
        );
        saved.occurred_at = UtcInstant { secs: 11 };
        assert_eq!(
            encode_history(&saved, invalid),
            Err(EventEncodeError::TimestampOutOfRange)
        );

        emitted.occurred_at = invalid;
        assert_eq!(
            encode_history(
                &record(HistoryFact::Event(emitted)),
                UtcInstant { secs: 12 }
            ),
            Err(EventEncodeError::TimestampOutOfRange)
        );

        let mut evidence = recovery();
        evidence.first_failure_observed_at = Some(invalid);
        assert_eq!(
            encode_history(
                &record(HistoryFact::RecoveryProgress {
                    evidence: evidence.clone(),
                    baseline: vpt_domain::run::RecoveryBaseline::default(),
                }),
                UtcInstant { secs: 12 }
            ),
            Err(EventEncodeError::TimestampOutOfRange)
        );
        evidence.first_failure_observed_at = None;
        evidence.logging_restored_observed_at = Some(invalid);
        assert_eq!(
            encode_history(
                &record(HistoryFact::RecoverySummary {
                    evidence,
                    window: None
                }),
                UtcInstant { secs: 12 }
            ),
            Err(EventEncodeError::TimestampOutOfRange)
        );
    }

    let line: serde_json::Value = serde_json::from_str(
        &encode_history(
            &record(HistoryFact::RecoverySummary {
                evidence: recovery(),
                window: None,
            }),
            UtcInstant { secs: 12 },
        )
        .unwrap(),
    )
    .unwrap();
    assert!(line["fact"]["evidence"]["first_failure_observed_at"].is_null());
    assert!(line["fact"]["evidence"]["logging_restored_observed_at"].is_null());
}
