use super::*;

#[test]
fn aggregate_review_has_recording_count_without_individual_identity_or_path() {
    let mut event = event(EventKind::ReviewNeeded, EventState::NeedsAttention);
    event.detail = EventDetail::ReviewAggregate { recordings: 4 };
    event.counts.low_confidence = 7;
    let value: Value = serde_json::from_slice(&encode_hook(&event).unwrap()).unwrap();
    assert_eq!(value["counts"]["recordings"], 4);
    assert_eq!(value["counts"]["low_confidence"], 7);
    assert_eq!(value["detail"], "4 recordings need review");
    assert!(value["recording"].is_null());
    assert_eq!(value["paths"], json!({}));
    assert_eq!(value["channel"], "alert");
}
