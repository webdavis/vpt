use super::committed;
use vpt_application::ports::{HistoryWrite, RunHistory, WorkDelta};
use vpt_domain::history::{Event, EventCounts, EventDetail, EventId, EventKind, EventState};
use vpt_domain::run::{HistoryFact, HookFailureClass, RunKind};
use vpt_domain::time::UtcInstant;

pub(crate) fn hook_failure_requires_event_in_same_run<L: RunHistory>(ledger: &L) {
    let started = committed(ledger.begin_run(RunKind::Ingest, UtcInstant { secs: 10 }));
    let event = Event {
        kind: EventKind::IngestFailed,
        state: EventState::Failed,
        recording: None,
        detail: EventDetail::None,
        counts: EventCounts::default(),
        paths: vec![],
        occurred_at: UtcInstant { secs: 11 },
    };
    let event_id = committed(ledger.commit_step(
        started.run,
        &WorkDelta::default(),
        &[(UtcInstant { secs: 11 }, HistoryFact::Event(event))],
    ))
    .records[0]
        .id;
    let accepted = HistoryFact::HookFailed {
        event: event_id,
        failure: HookFailureClass::Exited(17),
    };
    committed(ledger.commit_step(
        started.run,
        &WorkDelta::default(),
        &[(UtcInstant { secs: 12 }, accepted.clone())],
    ));
    assert_eq!(
        ledger.recovery_view().unwrap().records.last().unwrap().fact,
        accepted
    );
    let other = committed(ledger.begin_run(RunKind::Ingest, UtcInstant { secs: 13 }));
    for (run, event) in [
        (other.run, event_id),
        (started.run, started.records[0].id),
        (started.run, EventId::new(started.run.namespace(), 999)),
    ] {
        let result = ledger.commit_step(
            run,
            &WorkDelta::default(),
            &[(
                UtcInstant { secs: 14 },
                HistoryFact::HookFailed {
                    event,
                    failure: HookFailureClass::Spawn,
                },
            )],
        );
        assert!(matches!(result, HistoryWrite::Unconfirmed(_)), "{result:?}");
    }
}
