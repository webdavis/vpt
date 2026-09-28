use super::*;

#[test]
fn deferred_work_outcome_has_exact_private_wire_round_trip() {
    let fact = HistoryFact::WorkStep {
        recording: None,
        outcome: WorkOutcome::Deferred,
    };
    let stored = encode(&fact).expect("deferred work encodes");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&stored).unwrap(),
        serde_json::json!({"tag":"work","recording":null,"outcome":"deferred"})
    );
    assert_eq!(decode(&stored).unwrap(), fact);
}

#[test]
fn deferred_event_reason_count_and_source_path_round_trip() {
    let event = Event {
        kind: EventKind::Deferred,
        state: EventState::NeedsAttention,
        recording: None,
        detail: EventDetail::Deferred {
            reason: DeferralReason::ChangedDuringRead,
            count: 17,
        },
        counts: EventCounts::default(),
        paths: vec![EventPath {
            key: EventPathKey::Source,
            path: PathBuf::from("/owned/recording.m4a"),
        }],
        occurred_at: UtcInstant { secs: 22 },
    };
    let fact = HistoryFact::Event(event);
    let stored = encode(&fact).expect("deferred event encodes");
    let json: serde_json::Value = serde_json::from_str(&stored).unwrap();
    assert_eq!(
        json["event"]["detail"],
        serde_json::json!({"tag":"deferred","reason":"changed_during_read","count":17})
    );
    assert_eq!(
        json["event"]["paths"],
        serde_json::json!([{"key":"source","path":"/owned/recording.m4a"}])
    );
    assert_eq!(decode(&stored).unwrap(), fact);
}

#[test]
fn malformed_deferred_metadata_is_content_free_corruption() {
    let base = serde_json::json!({
        "tag":"event",
        "event":{
            "kind":"deferred","state":"needs_attention","recording":null,
            "detail":{"tag":"deferred","reason":"dataless","count":4},
            "counts":{"numeric":0,"proper_noun":0,"low_confidence":0,"formatting":0,
                "other":0,"unsupported":0,"agreed_unverified":0,"diagnostics":0},
            "paths":[{"key":"note","path":"/owned/recording.m4a"}],
            "occurred_at":22
        }
    });
    assert_eq!(
        decode(&base.to_string()).unwrap(),
        HistoryFact::Event(Event {
            kind: EventKind::Deferred,
            state: EventState::NeedsAttention,
            recording: None,
            detail: EventDetail::Deferred {
                reason: DeferralReason::Dataless,
                count: 4,
            },
            counts: EventCounts::default(),
            paths: vec![EventPath {
                key: EventPathKey::Note,
                path: PathBuf::from("/owned/recording.m4a"),
            }],
            occurred_at: UtcInstant { secs: 22 },
        })
    );
    for (field, value) in [
        ("reason", serde_json::Value::Null),
        ("reason", serde_json::json!("private-unknown-canary")),
        ("reason", serde_json::json!("Dataless")),
        ("count", serde_json::Value::Null),
        ("count", serde_json::json!(-1)),
        ("count", serde_json::json!(4.0)),
        ("count", serde_json::json!(4_294_967_296_u64)),
        ("count", serde_json::json!("private-count-canary")),
    ] {
        let mut stored = base.clone();
        stored["event"]["detail"][field] = value;
        let error = decode(&stored.to_string()).unwrap_err();
        assert!(matches!(error, LedgerError::Corrupt(_)));
        assert!(!format!("{error:?}").contains("private"));
    }
    for field in ["reason", "count"] {
        let mut stored = base.clone();
        stored["event"]["detail"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(matches!(
            decode(&stored.to_string()),
            Err(LedgerError::Corrupt(_))
        ));
    }
    let mut unknown_path = base.clone();
    unknown_path["event"]["paths"][0]["key"] = serde_json::json!("private-path-canary");
    assert!(matches!(
        decode(&unknown_path.to_string()),
        Err(LedgerError::Corrupt(_))
    ));
    let mut unknown_detail = base;
    unknown_detail["event"]["detail"]["tag"] = serde_json::json!("private-tag-canary");
    assert!(matches!(
        decode(&unknown_detail.to_string()),
        Err(LedgerError::Corrupt(_))
    ));
}

#[test]
fn all_deferred_reasons_use_their_exact_private_wire_spellings() {
    for (reason, spelling) in [
        (DeferralReason::Dataless, "dataless"),
        (DeferralReason::AudioTooLarge, "audio_too_large"),
        (DeferralReason::InvalidContainer, "invalid_container"),
        (DeferralReason::NotAtRest, "not_at_rest"),
        (DeferralReason::ChangedDuringRead, "changed_during_read"),
    ] {
        let fact = HistoryFact::Event(Event {
            kind: EventKind::Deferred,
            state: EventState::NeedsAttention,
            recording: None,
            detail: EventDetail::Deferred { reason, count: 1 },
            counts: EventCounts::default(),
            paths: vec![],
            occurred_at: UtcInstant { secs: 1 },
        });
        let stored = encode(&fact).unwrap();
        let json: serde_json::Value = serde_json::from_str(&stored).unwrap();
        assert_eq!(json["event"]["detail"]["reason"], spelling);
        assert_eq!(decode(&stored).unwrap(), fact);
    }
}
