use super::*;

#[test]
fn old_events_default_missing_finding_counts_and_corrupt_new_counts_refuse() {
    let fact = HistoryFact::Event(Event {
        kind: EventKind::ReviewNeeded,
        state: EventState::NeedsAttention,
        occasion: None,
        recording: None,
        detail: EventDetail::None,
        counts: EventCounts::default(),
        paths: vec![],
        occurred_at: UtcInstant { secs: 20 },
    });
    let mut stored: serde_json::Value = serde_json::from_str(&encode(&fact).unwrap()).unwrap();
    for key in ["unsourced", "bad_reference", "built_on_flagged_text"] {
        stored["event"]["counts"]
            .as_object_mut()
            .unwrap()
            .remove(key);
    }
    assert_eq!(decode(&stored.to_string()).unwrap(), fact);
    for key in ["unsourced", "bad_reference", "built_on_flagged_text"] {
        for invalid in [
            serde_json::Value::Null,
            serde_json::json!(-1),
            serde_json::json!(4294967296u64),
            serde_json::json!("private corruption"),
        ] {
            let mut changed = stored.clone();
            changed["event"]["counts"][key] = invalid;
            let error = decode(&changed.to_string()).unwrap_err();
            assert!(!format!("{error:?}").contains("private corruption"));
        }
    }
}
