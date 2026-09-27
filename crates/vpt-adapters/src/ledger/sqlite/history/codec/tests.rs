use super::*;
use std::path::PathBuf;
use vpt_domain::history::{
    Event, EventCounts, EventDetail, EventId, EventKind, EventPath, EventPathKey, EventState,
    LedgerNamespace, RunId, SafeFailureClass,
};
use vpt_domain::run::{
    EffectKind, EffectOutcome, EffectReference, FailureWindow, HookFailureClass, RecoveryEvidence,
    RunKind, RunOutcome, WorkOutcome,
};
use vpt_domain::time::UtcInstant;

#[test]
fn every_fact_variant_round_trips_with_typed_safe_event_fields() {
    let namespace = LedgerNamespace::new([0xab; 16]);
    let run = RunId::new(namespace, 4);
    let intent = EventId::new(namespace, 5);
    let evidence = RecoveryEvidence {
        affected_runs: vec![run],
        first_failure_observed_at: Some(UtcInstant { secs: 6 }),
        logging_restored_observed_at: Some(UtcInstant { secs: 7 }),
        failures: 2,
        recovered_records: 3,
        completed: 4,
        failed: 5,
        unresolved: 6,
        missing_history: 7,
    };
    let event = Event {
        kind: EventKind::ReviewNeeded,
        state: EventState::NeedsAttention,
        recording: None,
        detail: EventDetail::Review {
            spans: 6,
            agreed_unverified: 11,
        },
        counts: EventCounts {
            numeric: 1,
            proper_noun: 2,
            low_confidence: 3,
            formatting: 4,
            other: 5,
            unsupported: 6,
            agreed_unverified: 7,
            diagnostics: 8,
        },
        paths: vec![EventPath {
            key: EventPathKey::Note,
            path: PathBuf::from("/h/transcripts/note.md"),
        }],
        occurred_at: UtcInstant { secs: 8 },
    };
    let cases = [
        HistoryFact::RunStarted(RunKind::Ingest),
        HistoryFact::RunStarted(RunKind::Recover),
        HistoryFact::RunStarted(RunKind::Publish),
        HistoryFact::RunStarted(RunKind::Retain),
        HistoryFact::RunFinished(RunOutcome::Completed),
        HistoryFact::RunFinished(RunOutcome::Failed),
        HistoryFact::RunFinished(RunOutcome::Interrupted),
        HistoryFact::WorkStep {
            recording: None,
            outcome: WorkOutcome::Completed,
        },
        HistoryFact::WorkStep {
            recording: None,
            outcome: WorkOutcome::Failed,
        },
        HistoryFact::WorkStep {
            recording: None,
            outcome: WorkOutcome::Unknown,
        },
        HistoryFact::EffectIntent {
            kind: EffectKind::Archive,
            reference: EffectReference::ArchiveTarget(PathBuf::from("/h/audio/a.m4a")),
        },
        HistoryFact::EffectIntent {
            kind: EffectKind::Publication,
            reference: EffectReference::PublicationTarget(PathBuf::from("/h/transcripts/note.md")),
        },
        HistoryFact::EffectIntent {
            kind: EffectKind::AudioTrash,
            reference: EffectReference::AudioTrash {
                recording: crate::ledger::contract::record(2).id,
                path: PathBuf::from("/h/audio/a.m4a"),
            },
        },
        HistoryFact::EffectIntent {
            kind: EffectKind::RetentionTrash,
            reference: EffectReference::RetentionIntent {
                id: 9,
                path: PathBuf::from("/h/audio/a.m4a"),
            },
        },
        HistoryFact::EffectObserved {
            intent,
            outcome: EffectOutcome::Completed,
        },
        HistoryFact::EffectObserved {
            intent,
            outcome: EffectOutcome::Failed,
        },
        HistoryFact::EffectObserved {
            intent,
            outcome: EffectOutcome::Unknown,
        },
        HistoryFact::Event(event),
        HistoryFact::RecoveryProgress(evidence.clone()),
        HistoryFact::RecoverySummary {
            evidence,
            window: None,
        },
    ];
    for fact in cases {
        let stored = encode(&fact).unwrap();
        assert_eq!(decode(&stored).unwrap(), fact, "stored={stored}");
    }
}

#[test]
fn safe_failure_detail_round_trips_without_raw_error_text() {
    let event = Event {
        kind: EventKind::IngestFailed,
        state: EventState::Failed,
        recording: None,
        detail: EventDetail::Failure(SafeFailureClass::ArchiveFailed),
        counts: EventCounts::default(),
        paths: vec![],
        occurred_at: UtcInstant { secs: 1 },
    };
    let stored = encode(&HistoryFact::Event(event.clone())).unwrap();
    assert_eq!(decode(&stored).unwrap(), HistoryFact::Event(event));
    assert!(!stored.contains("private-error-canary"));
}

#[test]
fn hook_failure_classes_and_summary_window_round_trip_with_closed_tags() {
    let namespace = LedgerNamespace::new([0x31; 16]);
    let event = EventId::new(namespace, 7);
    for failure in [
        HookFailureClass::Spawn,
        HookFailureClass::Io,
        HookFailureClass::Encoding,
        HookFailureClass::Exited(-3),
        HookFailureClass::Signaled(9),
        HookFailureClass::DeadlineExceeded,
        HookFailureClass::Interrupted,
    ] {
        let fact = HistoryFact::HookFailed { event, failure };
        assert_eq!(decode(&encode(&fact).unwrap()).unwrap(), fact);
    }
    let evidence = RecoveryEvidence {
        affected_runs: vec![RunId::new(namespace, 1)],
        first_failure_observed_at: Some(UtcInstant { secs: 2 }),
        logging_restored_observed_at: None,
        failures: 1,
        recovered_records: 0,
        completed: 0,
        failed: 0,
        unresolved: 1,
        missing_history: 0,
    };
    let window = FailureWindow {
        after: Some(vpt_domain::history::ExportFailureId::new(namespace, 3)),
        through: vpt_domain::history::ExportFailureId::new(namespace, 4),
    };
    let fact = HistoryFact::RecoverySummary {
        evidence,
        window: Some(window),
    };
    assert_eq!(decode(&encode(&fact).unwrap()).unwrap(), fact);
    let malformed = r#"{"tag":"hook_failed","event":"31313131313131313131313131313131-7","failure":{"kind":"private-canary"}}"#;
    let error = decode(malformed).unwrap_err();
    assert!(matches!(error, LedgerError::Corrupt(_)));
    assert!(!format!("{error:?}").contains("private-canary"));
}

#[test]
fn work_only_recovery_evidence_round_trips_without_failure_observation() {
    let evidence = RecoveryEvidence {
        affected_runs: vec![RunId::new(LedgerNamespace::new([0x22; 16]), 1)],
        first_failure_observed_at: None,
        logging_restored_observed_at: None,
        failures: 0,
        recovered_records: 1,
        completed: 1,
        failed: 0,
        unresolved: 0,
        missing_history: 0,
    };
    let fact = HistoryFact::RecoverySummary {
        evidence,
        window: None,
    };
    let stored = encode(&fact).unwrap();
    assert_eq!(decode(&stored).unwrap(), fact);
    assert!(stored.contains("\"first_failure_observed_at\":null"));
}

#[test]
fn encoding_hook_failure_stays_distinct_from_io() {
    let event = EventId::new(LedgerNamespace::new([0x42; 16]), 1);
    let fact = HistoryFact::HookFailed {
        event,
        failure: HookFailureClass::Encoding,
    };
    let stored = encode(&fact).unwrap();
    assert_eq!(decode(&stored).unwrap(), fact);
    assert_ne!(
        stored,
        encode(&HistoryFact::HookFailed {
            event,
            failure: HookFailureClass::Io
        })
        .unwrap()
    );
}

#[test]
fn zero_affected_run_is_refused_before_recovery_fact_encoding() {
    let evidence = RecoveryEvidence {
        affected_runs: vec![RunId::new(LedgerNamespace::new([0x42; 16]), 0)],
        first_failure_observed_at: None,
        logging_restored_observed_at: None,
        failures: 0,
        recovered_records: 0,
        completed: 0,
        failed: 0,
        unresolved: 0,
        missing_history: 0,
    };
    for fact in [
        HistoryFact::RecoveryProgress(evidence.clone()),
        HistoryFact::RecoverySummary {
            evidence,
            window: None,
        },
    ] {
        assert!(matches!(encode(&fact), Err(LedgerError::Corrupt(_))));
    }
}

#[cfg(unix)]
#[test]
fn non_utf8_effect_and_event_paths_are_refused_without_lossy_replacement() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let path = PathBuf::from(OsString::from_vec(vec![b'/', b'h', b'/', 0xff]));
    let recording = crate::ledger::contract::record(2).id;
    for (kind, reference) in [
        (
            EffectKind::Archive,
            EffectReference::ArchiveTarget(path.clone()),
        ),
        (
            EffectKind::Publication,
            EffectReference::PublicationTarget(path.clone()),
        ),
        (
            EffectKind::AudioTrash,
            EffectReference::AudioTrash {
                recording,
                path: path.clone(),
            },
        ),
        (
            EffectKind::RetentionTrash,
            EffectReference::RetentionIntent {
                id: 9,
                path: path.clone(),
            },
        ),
    ] {
        let result = encode(&HistoryFact::EffectIntent { kind, reference });
        assert!(
            matches!(&result, Err(LedgerError::Corrupt(_))),
            "{result:?}"
        );
        assert!(!format!("{result:?}").contains("�"));
    }
    let event = Event {
        kind: EventKind::ReviewNeeded,
        state: EventState::NeedsAttention,
        recording: None,
        detail: EventDetail::None,
        counts: EventCounts::default(),
        paths: vec![EventPath {
            key: EventPathKey::Archive,
            path,
        }],
        occurred_at: UtcInstant { secs: 1 },
    };
    let result = encode(&HistoryFact::Event(event));
    assert!(
        matches!(&result, Err(LedgerError::Corrupt(_))),
        "{result:?}"
    );
    assert!(!format!("{result:?}").contains("�"));
}

#[test]
fn event_wire_vocabulary_and_safe_detail_branches_round_trip() {
    let mut event = Event {
        kind: EventKind::ReviewNeeded,
        state: EventState::NeedsAttention,
        recording: None,
        detail: EventDetail::None,
        counts: EventCounts {
            numeric: u32::MAX,
            ..Default::default()
        },
        paths: vec![],
        occurred_at: UtcInstant { secs: -1 },
    };
    for kind in [
        EventKind::ReviewNeeded,
        EventKind::TranscribeFailed,
        EventKind::IngestFailed,
        EventKind::Deferred,
        EventKind::BriefWritten,
        EventKind::Retention,
        EventKind::ConfigRefused,
    ] {
        event.kind = kind;
        assert_eq!(
            decode(&encode(&HistoryFact::Event(event.clone())).unwrap()).unwrap(),
            HistoryFact::Event(event.clone())
        );
    }
    for state in [
        EventState::NeedsAttention,
        EventState::Failed,
        EventState::Done,
    ] {
        event.state = state;
        assert_eq!(
            decode(&encode(&HistoryFact::Event(event.clone())).unwrap()).unwrap(),
            HistoryFact::Event(event.clone())
        );
    }
    for detail in [
        EventDetail::None,
        EventDetail::Review {
            spans: 0,
            agreed_unverified: u32::MAX,
        },
        EventDetail::Deferred,
        EventDetail::Retention {
            completed: 1,
            failed: 2,
        },
    ] {
        event.detail = detail;
        assert_eq!(
            decode(&encode(&HistoryFact::Event(event.clone())).unwrap()).unwrap(),
            HistoryFact::Event(event.clone())
        );
    }
    for class in [
        SafeFailureClass::SourceUnavailable,
        SafeFailureClass::ArchiveFailed,
        SafeFailureClass::RecordingFailed,
        SafeFailureClass::LedgerUnavailable,
        SafeFailureClass::ExportUnavailable,
        SafeFailureClass::ConfigurationRefused,
    ] {
        event.detail = EventDetail::Failure(class);
        assert_eq!(
            decode(&encode(&HistoryFact::Event(event.clone())).unwrap()).unwrap(),
            HistoryFact::Event(event.clone())
        );
    }
    for key in [
        EventPathKey::Note,
        EventPathKey::Audio,
        EventPathKey::Brief,
        EventPathKey::Archive,
    ] {
        event.paths = vec![EventPath {
            key,
            path: PathBuf::from("/safe/path"),
        }];
        assert_eq!(
            decode(&encode(&HistoryFact::Event(event.clone())).unwrap()).unwrap(),
            HistoryFact::Event(event.clone())
        );
    }
}

#[test]
fn unknown_stored_discriminants_and_oversized_counts_are_content_free_corruption() {
    for stored in [
        r#"{"tag":"unknown-private-canary"}"#,
        r#"{"tag":"event","event":{"kind":"unknown-private-canary"}}"#,
        r#"{"tag":"work","outcome":"unknown-private-canary"}"#,
        r#"{"tag":"event","event":{"counts":{"numeric":4294967296}}}"#,
    ] {
        let error = decode(stored).unwrap_err();
        assert!(matches!(error, LedgerError::Corrupt(_)));
        assert!(!format!("{error:?}").contains("private-canary"));
    }
}
