use super::*;
use vpt_domain::{history::RetentionCounts, layout::StoreKey};

fn event(counts: Option<RetentionCounts>) -> HistoryFact {
    HistoryFact::Event(Event {
        kind: EventKind::Retention,
        state: EventState::Done,
        recording: None,
        occasion: None,
        detail: EventDetail::Retention {
            completed: 8,
            failed: 0,
        },
        counts: EventCounts {
            retention: counts,
            ..Default::default()
        },
        paths: vec![],
        occurred_at: UtcInstant { secs: 20 },
    })
}

#[test]
fn per_store_retention_counts_round_trip_and_old_history_remains_readable() {
    let mut counts = RetentionCounts::default();
    for (index, store) in StoreKey::all().into_iter().enumerate() {
        counts.set(store, if index == 6 { u32::MAX } else { index as u32 });
    }
    let fact = event(Some(counts));
    assert_eq!(decode(&encode(&fact).unwrap()).unwrap(), fact);
    let old = event(None);
    let mut stored: serde_json::Value = serde_json::from_str(&encode(&old).unwrap()).unwrap();
    stored["event"]["counts"]
        .as_object_mut()
        .unwrap()
        .remove("retention");
    assert_eq!(decode(&stored.to_string()).unwrap(), old);
}

#[test]
fn malformed_store_counts_are_refused_without_private_values_in_the_error() {
    let stored: serde_json::Value =
        serde_json::from_str(&encode(&event(Some(RetentionCounts::default()))).unwrap()).unwrap();
    for store in StoreKey::all() {
        for invalid in [
            serde_json::Value::Null,
            serde_json::json!(-1),
            serde_json::json!(4294967296u64),
            serde_json::json!("PRIVATE corruption"),
        ] {
            let mut changed = stored.clone();
            changed["event"]["counts"]["retention"][store.key_name()] = invalid;
            let error = decode(&changed.to_string()).unwrap_err();
            assert!(!format!("{error:?}").contains("PRIVATE"));
        }
        let mut changed = stored.clone();
        changed["event"]["counts"]["retention"]
            .as_object_mut()
            .unwrap()
            .remove(store.key_name());
        assert!(decode(&changed.to_string()).is_err());
    }
}
