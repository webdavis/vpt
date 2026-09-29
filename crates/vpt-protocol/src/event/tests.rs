mod bounds;
mod history;
mod hook;
mod retention;

use super::*;
use vpt_domain::history::{
    EventCounts, EventDetail, EventId, EventKind, EventPath, EventState, LedgerNamespace, RunId,
};
use vpt_domain::identity::RecordingId;
use vpt_domain::run::HistoryFact;

fn event(kind: EventKind, state: EventState) -> Event {
    Event {
        kind,
        state,
        recording: None,
        detail: EventDetail::None,
        counts: EventCounts::default(),
        paths: Vec::<EventPath>::new(),
        occurred_at: UtcInstant { secs: 0 },
    }
}

fn recording() -> RecordingId {
    RecordingId::parse("2026-08-24T144736-4f3ab19c02de").unwrap()
}

fn record(fact: HistoryFact) -> HistoryRecord {
    let namespace = LedgerNamespace::new([0x42; 16]);
    HistoryRecord {
        id: EventId::new(namespace, 2),
        run: RunId::new(namespace, 1),
        occurred_at: UtcInstant { secs: 11 },
        fact,
        appended_at: None,
        export_position: None,
    }
}
