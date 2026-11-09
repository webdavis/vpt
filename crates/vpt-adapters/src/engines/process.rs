use crate::spawn::{self, Outcome, SpawnError, Status};
use std::time::Duration;
use vpt_application::ports::EngineFailure;

pub(super) fn run(argv: &[String], timeout: Duration) -> Result<Outcome, EngineFailure> {
    run_with_limit(argv, timeout, super::DOCUMENT_LIMIT + 1)
}

pub(super) fn run_with_limit(
    argv: &[String],
    timeout: Duration,
    limit: usize,
) -> Result<Outcome, EngineFailure> {
    spawn::run(argv, b"", timeout, limit).map_err(|error| match error {
        SpawnError::NotFound(_) => EngineFailure::Unavailable,
        SpawnError::Io(_) => EngineFailure::Io,
    })
}

pub(super) fn success(status: Status) -> Result<(), EngineFailure> {
    match status {
        Status::Exited(0) => Ok(()),
        Status::Exited(code) => Err(EngineFailure::Exited(code)),
        Status::Signaled(signal) => Err(EngineFailure::Signaled(signal)),
        Status::DeadlineExceeded => Err(EngineFailure::DeadlineExceeded),
        Status::Interrupted => Err(EngineFailure::Interrupted),
    }
}
