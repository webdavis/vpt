use super::super::{
    lease::SignalLease,
    runtime::{Controls, PipeSlots, execute},
    *,
};
use super::{lifecycle_fixture::Fixture, watchdog::Registration};
use std::cell::Cell;
use std::io;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Duration;

#[test]
fn finite_exit_returns_both_real_child_streams_and_reaps_the_leader() {
    let fixture = Fixture::new();
    let command = fixture.command("/bin/sh", &["-c", "printf out; printf err >&2; exit 3"]);
    let outcome = execute(
        command,
        b"",
        Duration::from_millis(500),
        OUTPUT_LIMIT,
        Controls {
            interrupted: || false,
            now: std::time::Instant::now,
            wait: |_| std::thread::yield_now(),
            grace: Duration::ZERO,
            reaped: |status| fixture.record_reaped(status),
            observed: |lease: SignalLease,
                       pipes: &mut PipeSlots,
                       registration: &mut Option<Registration>| {
                fixture.register(lease, pipes, registration)
            },
        },
    )
    .expect("finite process executed");
    assert_eq!(outcome.status, Status::Exited(3));
    assert_eq!(outcome.stdout, b"out");
    assert_eq!(outcome.stderr, b"err");
    fixture.assert_finished();
}

#[test]
fn finite_signal_returns_the_child_signal_and_reaps_the_leader() {
    let fixture = Fixture::new();
    let command = fixture.command("/bin/sh", &["-c", "kill -TERM $$"]);
    let outcome = execute(
        command,
        b"",
        Duration::from_millis(500),
        OUTPUT_LIMIT,
        Controls {
            interrupted: || false,
            now: std::time::Instant::now,
            wait: |_| std::thread::yield_now(),
            grace: Duration::ZERO,
            reaped: |status| fixture.record_reaped(status),
            observed: |lease: SignalLease,
                       pipes: &mut PipeSlots,
                       registration: &mut Option<Registration>| {
                fixture.register(lease, pipes, registration)
            },
        },
    )
    .expect("finite self-signal process executed");
    assert_eq!(outcome.status, Status::Signaled(libc::SIGTERM));
    assert!(outcome.stdout.is_empty());
    assert!(outcome.stderr.is_empty());
    fixture.assert_finished();
}

#[test]
fn a_deadline_after_readiness_terminates_and_reaps_the_owned_leader() {
    pending(false);
}

#[test]
fn interruption_after_readiness_terminates_and_reaps_the_owned_leader() {
    pending(true);
}

fn pending(interrupt: bool) {
    let fixture = Fixture::new();
    let elapsed = Cell::new(Duration::ZERO);
    let cancelled = Cell::new(false);
    let deadline = Duration::from_secs(1);
    let outcome = execute(
        fixture.pending(),
        b"",
        deadline,
        OUTPUT_LIMIT,
        Controls {
            interrupted: || cancelled.get(),
            now: || fixture.started + elapsed.get(),
            wait: |_| {
                if fixture.ready() || fixture.watchdog_fired() {
                    if interrupt {
                        cancelled.set(true);
                    } else {
                        elapsed.set(deadline);
                    }
                }
                std::thread::yield_now();
            },
            grace: Duration::ZERO,
            reaped: |status| fixture.record_reaped(status),
            observed: |lease: SignalLease,
                       pipes: &mut PipeSlots,
                       registration: &mut Option<Registration>| {
                fixture.register(lease, pipes, registration)
            },
        },
    )
    .expect("pending process executed");
    assert_eq!(
        outcome.status,
        if interrupt {
            Status::Interrupted
        } else {
            Status::DeadlineExceeded
        }
    );
    assert!(
        fixture.ready(),
        "child readiness preceded deadline or cancellation"
    );
    fixture.assert_finished();
}

#[test]
fn elapsed_spawn_and_registration_at_the_deadline_cannot_report_success() {
    let fixture = Fixture::new();
    let elapsed = Cell::new(Duration::ZERO);
    let deadline = Duration::from_secs(1);
    let command = fixture.command("/bin/sh", &["-c", "exit 0"]);
    let outcome = execute(
        command,
        b"",
        deadline,
        OUTPUT_LIMIT,
        Controls {
            interrupted: || false,
            now: || fixture.started + elapsed.get(),
            wait: |_| std::thread::yield_now(),
            grace: Duration::ZERO,
            reaped: |status| fixture.record_reaped(status),
            observed: |lease: SignalLease,
                       pipes: &mut PipeSlots,
                       registration: &mut Option<Registration>| {
                fixture.register(lease, pipes, registration)?;
                elapsed.set(deadline);
                Ok(())
            },
        },
    )
    .expect("late first observation executed");
    assert_eq!(outcome.status, Status::DeadlineExceeded);
    fixture.assert_finished();
}

#[test]
fn registered_observer_error_cleans_up_without_exposing_private_details() {
    let fixture = Fixture::new();
    let result = execute(
        fixture.pending(),
        b"",
        Duration::from_secs(1),
        OUTPUT_LIMIT,
        Controls {
            interrupted: || false,
            now: std::time::Instant::now,
            wait: |_| std::thread::yield_now(),
            grace: Duration::ZERO,
            reaped: |status| fixture.record_reaped(status),
            observed: |lease: SignalLease,
                       pipes: &mut PipeSlots,
                       registration: &mut Option<Registration>| {
                fixture.register(lease, pipes, registration)?;
                Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "private observer sentinel",
                ))
            },
        },
    );
    let error = result.expect_err("observer failure returned");
    assert!(matches!(&error, SpawnError::Io(message) if message.contains("PermissionDenied")));
    assert!(!format!("{error:?}").contains("private observer sentinel"));
    fixture.assert_finished();
}

#[test]
fn observer_unwinding_keeps_registration_until_owned_cleanup() {
    let fixture = Fixture::new();
    let result = catch_unwind(AssertUnwindSafe(|| {
        execute(
            fixture.pending(),
            b"",
            Duration::from_secs(1),
            OUTPUT_LIMIT,
            Controls {
                interrupted: || false,
                now: std::time::Instant::now,
                wait: |_| std::thread::yield_now(),
                grace: Duration::ZERO,
                reaped: |status| fixture.record_reaped(status),
                observed: |lease: SignalLease,
                           pipes: &mut PipeSlots,
                           registration: &mut Option<Registration>| {
                    fixture.register(lease, pipes, registration)?;
                    panic!("owned observer unwind");
                },
            },
        )
    }));
    assert!(result.is_err(), "observer must run and unwind");
    fixture.assert_finished();
}
