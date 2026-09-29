use super::*;

fn retention(completed: u32, failed: u32) -> Event {
    let mut value = event(EventKind::Retention, EventState::Done);
    value.detail = EventDetail::Retention { completed, failed };
    value.counts.numeric = 91;
    value.counts.diagnostics = 92;
    value
}

#[test]
fn retention_hook_audio_count_is_the_actual_move_count() {
    let value: Value = serde_json::from_slice(&encode_hook(&retention(3, 7)).unwrap()).unwrap();
    assert_eq!(value["counts"]["audio"], 3);
    assert_eq!(value["counts"]["numeric"], 91);
    assert_eq!(value["counts"]["diagnostics"], 92);
    assert_eq!(value["channel"], "report");
}

#[test]
fn retention_log_audio_count_matches_hook_at_zero_and_upper_boundary() {
    for moved in [0, u32::MAX] {
        let event = retention(moved, 1);
        let hook: Value = serde_json::from_slice(&encode_hook(&event).unwrap()).unwrap();
        let logged: Value = serde_json::from_str(
            &encode_history(&record(HistoryFact::Event(event)), UtcInstant { secs: 20 }).unwrap(),
        )
        .unwrap();
        assert_eq!(logged["fact"]["event"]["counts"]["audio"], moved);
        assert_eq!(logged["fact"]["event"]["counts"], hook["counts"]);
    }
}

#[test]
fn non_retention_event_does_not_claim_audio_moves() {
    let mut value = event(EventKind::BriefWritten, EventState::Done);
    value.detail = EventDetail::Retention {
        completed: 7,
        failed: 2,
    };
    let hook: Value = serde_json::from_slice(&encode_hook(&value).unwrap()).unwrap();
    assert!(hook["counts"].get("audio").is_none());
}

#[test]
fn retention_without_a_move_detail_does_not_invent_an_audio_count() {
    let hook: Value = serde_json::from_slice(
        &encode_hook(&event(EventKind::Retention, EventState::Done)).unwrap(),
    )
    .unwrap();
    assert!(hook["counts"].get("audio").is_none());
}
