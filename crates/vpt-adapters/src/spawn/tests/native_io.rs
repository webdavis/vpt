use super::super::{
    Outcome, SpawnError, Status,
    lease::SignalLease,
    runtime::{Controls, PipeSlots, execute},
};
use super::{
    lifecycle_fixture::Fixture,
    native_io_fixture::{ERR_MARKER, OUT_MARKER, PRESSURE_BYTES},
    watchdog::Registration,
};
use std::fs::OpenOptions;
use std::io;
use std::process::Command;
use std::time::{Duration, Instant};

#[test]
fn native_nonempty_input_reaches_owned_child_exactly() {
    let fixture = Fixture::new();
    let input = b"private binary\0input\xff\n";
    let outcome = execute_owned(
        &fixture,
        command(&fixture, "exact", input.len()),
        input,
        1024,
        |_| Ok(()),
    )
    .expect("native input exchange executed");
    assert_eq!(outcome.status, Status::Exited(0));
    assert_eq!(after_marker(&outcome.stdout, OUT_MARKER), b"native-out");
    assert_eq!(after_marker(&outcome.stderr, ERR_MARKER), b"native-error");
    assert_eq!(
        outcome.stderr.len(),
        ERR_MARKER.len() + b"native-error".len()
    );
    let received = std::fs::read(fixture.root.path().join("input")).expect("owned input record");
    assert!(received == input, "exact child input differs");
    assert_eq!(
        std::fs::read_to_string(fixture.root.path().join("counts")).expect("owned counts"),
        format!(
            "{} {} {} false false true",
            input.len(),
            OUT_MARKER.len() + 10,
            ERR_MARKER.len() + 12
        )
    );
    fixture.assert_finished();
}

#[test]
fn native_dual_stream_pressure_drains_after_capture_limits() {
    let fixture = Fixture::new();
    let input = vec![b'i'; PRESSURE_BYTES];
    let outcome = execute_owned(
        &fixture,
        command(&fixture, "pressure", input.len()),
        &input,
        1024,
        |_| Ok(()),
    )
    .expect("native pressure exchange executed");
    assert_eq!(outcome.status, Status::Exited(0));
    assert_eq!(outcome.stdout.len(), 1024);
    assert_eq!(outcome.stderr.len(), 1024);
    let output = after_marker(&outcome.stdout, OUT_MARKER);
    assert!(!output.is_empty() && output.iter().all(|byte| *byte == b'o'));
    let error = after_marker(&outcome.stderr, ERR_MARKER);
    assert_eq!(error.len(), 1024 - ERR_MARKER.len());
    assert!(error.iter().all(|byte| *byte == b'e'));
    let received = std::fs::read(fixture.root.path().join("input")).expect("owned input record");
    assert!(received == input, "pressure input differs");
    assert_eq!(
        std::fs::read_to_string(fixture.root.path().join("counts")).expect("owned counts"),
        format!(
            "{PRESSURE_BYTES} {} {} true true true",
            OUT_MARKER.len() + PRESSURE_BYTES,
            ERR_MARKER.len() + PRESSURE_BYTES
        )
    );
    fixture.assert_finished();
}

#[test]
fn native_early_input_close_is_safe_error_and_reaps() {
    let fixture = Fixture::new();
    let input = b"private unwritten input";
    let result = execute_owned(
        &fixture,
        command(&fixture, "closed-input", input.len()),
        input,
        1024,
        |_| Ok(()),
    );
    assert_io(result, io::ErrorKind::BrokenPipe);
    fixture.assert_finished();
}

#[test]
fn injected_output_read_error_is_safe_and_reaps() {
    let fixture = Fixture::new();
    let result = execute_owned(&fixture, fixture.pending(), b"", 1024, |pipes| {
        let output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(fixture.root.path().join("private-read-sentinel"))?;
        pipes.stdout = Some(output.into());
        Ok(())
    });
    assert_io(result, io::Error::from_raw_os_error(libc::EBADF).kind());
    fixture.assert_finished();
}

#[test]
fn missing_output_pipe_is_setup_error_and_reaps() {
    let fixture = Fixture::new();
    let result = execute_owned(&fixture, fixture.pending(), b"", 1024, |pipes| {
        drop(pipes.stdout.take());
        Ok(())
    });
    assert_io(result, io::ErrorKind::InvalidInput);
    fixture.assert_finished();
}

fn command(fixture: &Fixture, mode: &str, bytes: usize) -> Command {
    let executable = std::env::current_exe().expect("test executable");
    let mut command = fixture.command(
        executable.to_str().expect("fixture executable path"),
        &[
            "--exact",
            "spawn::tests::native_io_fixture::child_exchange",
            "--nocapture",
        ],
    );
    command
        .env("VPT_PROCESS_FIXTURE", fixture.root.path())
        .env("VPT_PROCESS_IO_MODE", mode)
        .env("VPT_PROCESS_INPUT_BYTES", bytes.to_string());
    command
}

fn execute_owned(
    fixture: &Fixture,
    command: Command,
    input: &[u8],
    limit: usize,
    after_ready: impl FnOnce(&mut PipeSlots) -> io::Result<()>,
) -> Result<Outcome, SpawnError> {
    execute(
        command,
        input,
        Duration::from_millis(500),
        limit,
        Controls {
            interrupted: || false,
            now: Instant::now,
            wait: |_| std::thread::yield_now(),
            grace: Duration::ZERO,
            reaped: |status| fixture.record_reaped(status),
            observed: |lease: SignalLease,
                       pipes: &mut PipeSlots,
                       registration: &mut Option<Registration>| {
                fixture.register(lease, pipes, registration)?;
                while !fixture.ready() && !fixture.watchdog_fired() {
                    if fixture.leader_terminal() {
                        return Err(io::ErrorKind::UnexpectedEof.into());
                    }
                    std::thread::yield_now();
                }
                if !fixture.ready() {
                    return Err(io::ErrorKind::TimedOut.into());
                }
                after_ready(pipes)
            },
        },
    )
}

fn assert_io(result: Result<Outcome, SpawnError>, kind: io::ErrorKind) {
    let error = result.expect_err("native I/O failure returned");
    let expected = format!("{kind:?}");
    assert!(
        matches!(&error, SpawnError::Io(message) if message.contains(&expected)),
        "expected safe {kind:?} failure"
    );
    assert!(!format!("{error:?}").contains("private"));
}

fn after_marker<'a>(bytes: &'a [u8], marker: &[u8]) -> &'a [u8] {
    let start = bytes
        .windows(marker.len())
        .position(|window| window == marker)
        .expect("owned stream marker retained");
    &bytes[start + marker.len()..]
}
