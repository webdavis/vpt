use super::*;
use serde_json::{Value, json};

fn evidence() -> Value {
    json!({
        "affected_runs": [RunId::new(LedgerNamespace::new([0x55; 16]), 1).render()],
        "first_failure_observed_at": null,
        "logging_restored_observed_at": null,
        "failures": 2,
        "recovered_records": 3,
        "completed_work": 4,
        "failed_work": 5,
        "unknown_work": 6,
        "deferred_work": 0,
        "completed_effects": 7,
        "failed_effects": 8,
        "unresolved_effects": 9,
        "missing_history": 10,
    })
}

#[test]
fn distinct_recovery_work_and_effect_counts_round_trip() {
    let stored = json!({"tag":"recovery_summary", "evidence":evidence(), "window":null});
    let fact = decode(&stored.to_string()).expect("closed recovery evidence");
    let round_trip: Value = serde_json::from_str(&encode(&fact).unwrap()).unwrap();
    assert_eq!(round_trip, stored);
}

#[test]
fn legacy_ambiguous_recovery_counts_are_refused() {
    let mut old = evidence();
    let fields = old.as_object_mut().unwrap();
    for field in [
        "completed_work",
        "failed_work",
        "unknown_work",
        "completed_effects",
        "failed_effects",
        "unresolved_effects",
    ] {
        fields.remove(field);
    }
    fields.insert("completed".into(), json!(4));
    fields.insert("failed".into(), json!(5));
    fields.insert("unresolved".into(), json!(6));
    let stored = json!({"tag":"recovery_summary", "evidence":old, "window":null});
    assert!(matches!(
        decode(&stored.to_string()),
        Err(LedgerError::Corrupt(_))
    ));
}

#[test]
fn deferred_recovery_count_round_trips_without_merging_work_classes() {
    let mut value = evidence();
    value["deferred_work"] = json!(3);
    let stored = json!({"tag":"recovery_summary", "evidence":value, "window":null});
    let fact = decode(&stored.to_string()).expect("typed recovery evidence");
    let round_trip: Value = serde_json::from_str(&encode(&fact).unwrap()).unwrap();
    assert_eq!(round_trip, stored);
}

#[test]
fn missing_deferred_recovery_count_is_content_free_corruption() {
    let mut value = evidence();
    value.as_object_mut().unwrap().remove("deferred_work");
    let stored = json!({"tag":"recovery_summary", "evidence":value, "window":null});
    let error = decode(&stored.to_string()).expect_err("deferred count is required");
    assert!(matches!(error, LedgerError::Corrupt(_)));
}
