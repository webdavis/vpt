mod calendar;
mod finish;
mod process;
mod usage;
use super::*;
use crate::spawn::{Outcome, SpawnError};
use std::time::{Duration, Instant};
use vpt_domain::brief::ContextStatus;

pub(super) fn run(
    command: &[String],
    max_age: &[String],
    request: &ContextRequest<'_>,
    mut runner: impl FnMut(&[String], Duration) -> Result<Outcome, SpawnError>,
    mut now: impl FnMut() -> Instant,
) -> Result<ContextSnapshot, ContextError> {
    let deadline = now() + Duration::from_secs(30);
    let plan = plan::make(command, max_age, request)?;
    let mut snapshot = ContextSnapshot::default();
    let mut call = |args: &[String]| {
        let remaining = deadline
            .checked_duration_since(now())
            .filter(|d| !d.is_zero())
            .ok_or(Failure::Deadline)?;
        let mut argv = command.to_vec();
        argv.extend_from_slice(args);
        let bytes = process::output(runner(&argv, remaining), args);
        if !matches!(&bytes, Err(Failure::Interrupted)) && now() >= deadline {
            return Err(Failure::Deadline);
        }
        bytes
    };
    if let (Some(agenda), Some(window)) = (plan.calendar, request.window) {
        let result = agenda.and_then(|agenda| calendar::collect(&agenda, window, &mut call));
        snapshot.status.calendar = settle(result.as_ref().map(|_| ()))?;
        snapshot.occasions = result.map(|s| s.occasions).unwrap_or_default();
    }
    if let Some(queries) = plan.tasks {
        let mut present = ContextSnapshot::default();
        present.status.tasks = ContextStatus::Present;
        let result = queries
            .into_iter()
            .try_fold(present, |mut collected, query| {
                let row = decode::read(&call(&query.args())?, &query.scope)?;
                process::merge(&mut collected, row)?;
                Ok(collected)
            });
        snapshot.status.tasks = settle(result.as_ref().map(|_| ()))?;
        snapshot.tasks = result.map(|s| s.tasks).unwrap_or_default();
    }
    snapshot = finish::normalize(snapshot, request)?;
    if now() >= deadline {
        finish::deadline(&mut snapshot, request);
        snapshot = finish::normalize(snapshot, request)?;
    }
    Ok(snapshot)
}

fn settle(result: Result<(), &Failure>) -> Result<ContextStatus, ContextError> {
    match result {
        Ok(()) => Ok(ContextStatus::Present),
        Err(Failure::Interrupted) => Err(ContextError::Interrupted),
        Err(error) => Ok(ContextStatus::Failed {
            detail: error.detail().into(),
        }),
    }
}
