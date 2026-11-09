use super::*;

#[test]
fn brief_event_identity_and_counts_round_trip_and_legacy_events_still_decode() {
    let mut event = Event {
        kind: EventKind::BriefWritten,
        state: EventState::Done,
        recording: None,
        occasion: Some(
            vpt_domain::occasion::OccasionId::parse("2026-09-30T230000-abcdefabcdef").unwrap(),
        ),
        detail: EventDetail::Brief {
            notes: 2,
            spans: 7,
            tasks: 3,
            unresolved: 4,
        },
        counts: EventCounts::default(),
        paths: vec![],
        occurred_at: UtcInstant { secs: 20 },
    };
    let fact = HistoryFact::Event(event.clone());
    let stored = encode(&fact).unwrap();
    assert_eq!(decode(&stored).unwrap(), fact);
    let mut value: serde_json::Value = serde_json::from_str(&stored).unwrap();
    for invalid in [serde_json::json!("Private invalid"), serde_json::json!(123)] {
        value["event"]["occasion"] = invalid;
        assert!(!format!("{:?}", decode(&value.to_string()).unwrap_err()).contains("Private"));
    }
    event.occasion = None;
    let fact = HistoryFact::Event(event);
    let mut old: serde_json::Value = serde_json::from_str(&encode(&fact).unwrap()).unwrap();
    old["event"].as_object_mut().unwrap().remove("occasion");
    assert_eq!(decode(&old.to_string()).unwrap(), fact);
    for field in ["notes", "spans", "tasks", "unresolved"] {
        let mut corrupt = old.clone();
        corrupt["event"]["detail"][field] = serde_json::json!(-1);
        assert!(decode(&corrupt.to_string()).is_err());
    }
}
