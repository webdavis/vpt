use super::*;

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
        EventDetail::Deferred {
            reason: DeferralReason::Dataless,
            count: 4,
        },
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
