use super::{
    Outcome, SpawnError, Status, group,
    io::{Capture, Input},
    lease::SignalLease,
    process,
};
use std::io;
use std::os::fd::OwnedFd;
use std::process::{Command, ExitStatus};
use std::time::{Duration, Instant};

pub(super) struct PipeSlots {
    pub stdin: Option<OwnedFd>,
    pub stdout: Option<OwnedFd>,
    pub stderr: Option<OwnedFd>,
}

pub(super) struct Controls<I, K, W, O, F> {
    pub interrupted: I,
    pub now: K,
    pub wait: W,
    pub grace: Duration,
    pub observed: O,
    pub reaped: F,
}

pub(super) fn execute<I, K, W, O, F, R>(
    command: Command,
    stdin: &[u8],
    deadline: Duration,
    output_limit: usize,
    controls: Controls<I, K, W, O, F>,
) -> Result<Outcome, SpawnError>
where
    I: Fn() -> bool,
    K: Fn() -> Instant,
    W: Fn(Duration),
    O: FnOnce(SignalLease, &mut PipeSlots, &mut Option<R>) -> io::Result<()>,
    F: Fn(ExitStatus),
{
    let Controls {
        interrupted,
        now,
        wait,
        grace,
        observed,
        reaped,
    } = controls;
    let (mut owned, slots, started) = group::launch(command, &now, &wait, grace, observed, reaped)?;
    let group = owned.group();
    let mut stdout = Capture::new(output_limit);
    let mut stderr = Capture::new(output_limit);
    let result = (|| {
        let mut input_pipe = Some(process::pipe(slots.stdin)?);
        let mut output_pipe = process::pipe(slots.stdout)?;
        let mut error_pipe = process::pipe(slots.stderr)?;
        let mut input = Input::new(stdin);
        let mut status = None;
        loop {
            if interrupted() {
                return Ok(Status::Interrupted);
            }
            if now().saturating_duration_since(started) >= deadline {
                return Ok(Status::DeadlineExceeded);
            }
            if let Some(status) = status
                && input_pipe.is_none()
                && stdout.finished
                && stderr.finished
            {
                return Ok(status);
            }
            if let Some(pipe) = input_pipe.as_mut()
                && input.write(pipe)?
            {
                drop(input_pipe.take());
            }
            stdout.drain(&mut output_pipe)?;
            stderr.drain(&mut error_pipe)?;
            if status.is_none() {
                match owned.observe() {
                    Ok(observed) => status = observed,
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                    Err(error) => return Err(error),
                }
            }
            wait(Duration::from_millis(1));
        }
    })();
    let cleanup = owned.finish(&now, &wait);
    let status = match (result, cleanup) {
        (_, Err(error)) if error.raw_os_error() == Some(libc::ECHILD) => {
            return Err(safe_error(error));
        }
        (Err(error), _) | (Ok(_), Err(error)) => return Err(safe_error(error)),
        (Ok(status), Ok(())) => status,
    };
    Ok(Outcome {
        status,
        stdout: stdout.bytes,
        stderr: stderr.bytes,
        group,
    })
}

pub(super) fn safe_error(error: io::Error) -> SpawnError {
    SpawnError::Io(format!("process I/O: {:?}", error.kind()))
}
