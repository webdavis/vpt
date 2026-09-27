use super::*;
use serde_json::{Value, json};
use std::path::PathBuf;
use vpt_domain::history::{EventPathKey, SafeFailureClass};

fn hook(event: &Event) -> Value {
    serde_json::from_slice(&encode_hook(event).unwrap()).unwrap()
}

#[test]
fn hook_document_preserves_typed_event_metadata() {
    let mut value = event(EventKind::ReviewNeeded, EventState::NeedsAttention);
    value.recording = Some(recording());
    value.detail = EventDetail::Review {
        spans: 6,
        agreed_unverified: 11,
    };
    value.counts = EventCounts {
        numeric: 3,
        proper_noun: 2,
        low_confidence: 4,
        formatting: 5,
        other: 6,
        unsupported: 1,
        agreed_unverified: 11,
        diagnostics: 7,
    };
    value.paths = vec![
        EventPath {
            key: EventPathKey::Note,
            path: PathBuf::from("/sandbox/note-雪.md"),
        },
        EventPath {
            key: EventPathKey::Audio,
            path: PathBuf::from("/sandbox/audio.m4a"),
        },
        EventPath {
            key: EventPathKey::Brief,
            path: PathBuf::from("/sandbox/brief.md"),
        },
        EventPath {
            key: EventPathKey::Archive,
            path: PathBuf::from("/sandbox/archive.md"),
        },
    ];
    value.occurred_at = UtcInstant {
        secs: 1_787_604_456,
    };

    let bytes = encode_hook(&value).unwrap();
    assert!(!bytes.contains(&b'\n'));
    assert_eq!(
        serde_json::from_slice::<Value>(&bytes).unwrap(),
        json!({
            "schema":"vpt.event/1",
            "channel":"alert",
            "event":"review_needed",
            "state":"needs_attention",
            "recording":"2026-08-24T144736-4f3ab19c02de",
            "detail":"6 spans to review, 11 agreed spans unverified",
            "counts":{
                "numeric":3,"proper_noun":2,"low_confidence":4,"formatting":5,
                "other":6,"unsupported":1,"agreed_unverified":11,"diagnostics":7
            },
            "paths":{
                "note":"/sandbox/note-雪.md","audio":"/sandbox/audio.m4a",
                "brief":"/sandbox/brief.md","archive":"/sandbox/archive.md"
            },
            "occurred_at":"2026-08-24T20:47:36Z"
        })
    );
}

#[test]
fn all_event_kinds_choose_the_approved_channel() {
    for (kind, state, channel) in [
        (EventKind::BriefWritten, EventState::Done, "report"),
        (EventKind::Retention, EventState::Done, "report"),
        (EventKind::ReviewNeeded, EventState::NeedsAttention, "alert"),
        (EventKind::TranscribeFailed, EventState::Failed, "alert"),
        (EventKind::IngestFailed, EventState::Failed, "alert"),
        (EventKind::Deferred, EventState::NeedsAttention, "alert"),
        (EventKind::ConfigRefused, EventState::Failed, "alert"),
    ] {
        let value = hook(&event(kind, state));
        assert_eq!(value["channel"], channel, "{:?}", kind);
        assert_eq!(value["event"], kind.as_str());
        assert_eq!(value["state"], state.as_str());
        assert_eq!(value["recording"], Value::Null);
    }

    for (detail, expected) in [
        (EventDetail::None, ""),
        (
            EventDetail::Review {
                spans: 2,
                agreed_unverified: 3,
            },
            "2 spans to review, 3 agreed spans unverified",
        ),
        (
            EventDetail::Failure(SafeFailureClass::SourceUnavailable),
            "source unavailable",
        ),
        (
            EventDetail::Failure(SafeFailureClass::ArchiveFailed),
            "archive failed",
        ),
        (
            EventDetail::Failure(SafeFailureClass::RecordingFailed),
            "recording failed",
        ),
        (
            EventDetail::Failure(SafeFailureClass::LedgerUnavailable),
            "ledger unavailable",
        ),
        (
            EventDetail::Failure(SafeFailureClass::ExportUnavailable),
            "event export unavailable",
        ),
        (
            EventDetail::Failure(SafeFailureClass::ConfigurationRefused),
            "configuration refused",
        ),
        (EventDetail::Deferred, "recording deferred"),
        (
            EventDetail::Retention {
                completed: 4,
                failed: 1,
            },
            "4 retention items completed, 1 failed",
        ),
    ] {
        let mut value = event(EventKind::ReviewNeeded, EventState::NeedsAttention);
        value.detail = detail;
        assert_eq!(hook(&value)["detail"], expected, "{:?}", detail);
    }
}
