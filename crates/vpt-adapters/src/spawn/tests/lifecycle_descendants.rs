use super::super::{
    lease::SignalLease,
    runtime::{Controls, PipeSlots, execute},
    *,
};
use super::{lifecycle_fixture::Fixture, watchdog::Registration};
use std::cell::Cell;
use std::time::Duration;

#[test]
fn exited_leader_with_injected_participant_pipe_waits_for_deadline_and_reaps_both_owners() {
    descendant(false, false);
}

#[test]
fn closed_participant_pipes_still_require_group_cleanup_and_exact_reaping() {
    descendant(true, false);
}

#[test]
fn interruption_overrides_an_exited_leader_with_held_participant_pipe() {
    descendant(false, true);
}

fn descendant(closed: bool, interrupt: bool) {
    let fixture = Fixture::new();
    let elapsed = Cell::new(Duration::ZERO);
    let cancelled = Cell::new(false);
    let deadline = Duration::from_secs(2);
    let command = fixture.command("/bin/sh", &["-c", "exit 3"]);
    let outcome = execute(
        command,
        b"",
        deadline,
        OUTPUT_LIMIT,
        Controls {
            interrupted: || cancelled.get(),
            now: || fixture.started + elapsed.get(),
            wait: |_| {
                if fixture.watchdog_fired() || fixture.leader_terminal() {
                    if interrupt {
                        cancelled.set(true);
                    }
                    elapsed.set(elapsed.get() + Duration::from_millis(100));
                }
                std::thread::yield_now();
            },
            grace: GRACE,
            reaped: |status| fixture.record_reaped(status),
            observed: |lease: SignalLease,
                       pipes: &mut PipeSlots,
                       registration: &mut Option<Registration>| {
                fixture.register_peer(lease, pipes, registration, closed)
            },
        },
    )
    .expect("owned participant scenario executed");
    let expected = if interrupt {
        Status::Interrupted
    } else if closed {
        Status::Exited(3)
    } else {
        Status::DeadlineExceeded
    };
    assert_eq!(outcome.status, expected);
    assert!(elapsed.get() >= GRACE, "virtual grace observed");
    fixture.assert_participant_reaped();
    fixture.assert_finished();
}
