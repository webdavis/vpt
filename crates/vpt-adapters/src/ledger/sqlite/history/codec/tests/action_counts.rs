use super::*;

fn event(withdrawn: Option<u32>) -> HistoryFact {
    HistoryFact::Event(Event {
        kind: EventKind::ActionsSent,
        state: EventState::Done,
        recording: None,
        occasion: None,
        detail: EventDetail::Actions {
            sent: 1,
            failed: 0,
            refused: 0,
            withdrawn,
        },
        counts: EventCounts::default(),
        paths: vec![],
        occurred_at: UtcInstant { secs: 20 },
    })
}

#[test]
fn action_counts_round_trip_with_and_without_a_withdrawn_count() {
    for withdrawn in [None, Some(0), Some(3)] {
        let fact = event(withdrawn);
        let stored = encode(&fact).unwrap();
        assert_eq!(stored.contains("withdrawn"), withdrawn.is_some());
        assert_eq!(decode(&stored).unwrap(), fact);
    }
}
