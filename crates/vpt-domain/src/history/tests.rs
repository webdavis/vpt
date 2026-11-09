use super::*;
use crate::sweep::DeferralReason;

#[test]
fn detail_is_derived_from_typed_safe_counts() {
    let event = Event {
        kind: EventKind::ReviewNeeded,
        state: EventState::NeedsAttention,
        occasion: None,
        recording: None,
        detail: EventDetail::Review {
            spans: 6,
            agreed_unverified: 11,
        },
        counts: EventCounts::default(),
        paths: vec![],
        occurred_at: UtcInstant { secs: 0 },
    };
    assert_eq!(
        event.detail_text(),
        "6 spans to review, 11 agreed spans unverified"
    );
}

#[test]
fn deferred_detail_uses_typed_reason_and_count() {
    let event = Event {
        kind: EventKind::Deferred,
        state: EventState::NeedsAttention,
        occasion: None,
        recording: None,
        detail: EventDetail::Deferred {
            reason: DeferralReason::Dataless,
            count: 4,
        },
        counts: EventCounts::default(),
        paths: vec![],
        occurred_at: UtcInstant { secs: 0 },
    };
    assert_eq!(event.detail_text(), "deferred 4 times: dataless");
}

#[test]
fn source_event_path_key_has_exact_wire_name() {
    let name = std::panic::catch_unwind(|| EventPathKey::Source.as_str());
    assert_eq!(name.ok(), Some("source"));
}
