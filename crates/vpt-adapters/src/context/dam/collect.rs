mod finish;
mod process;
use super::*;
use crate::spawn::{Outcome, SpawnError};
use std::time::{Duration, Instant};
use vpt_domain::brief::ContextStatus;

pub(super) fn run(
    command: &[String],
    request: &ContextRequest<'_>,
    mut runner: impl FnMut(&[String], Duration) -> Result<Outcome, SpawnError>,
    mut now: impl FnMut() -> Instant,
) -> Result<ContextSnapshot, ContextError> {
    let deadline = now() + Duration::from_secs(30);
    let plan = plan::make(command, request)?;
    let mut snapshot = ContextSnapshot::default();
    let mut half = |queries: Result<Vec<plan::Query>, Failure>, calendar| {
        let result = queries.and_then(|queries| {
            let mut collected = ContextSnapshot::default();
            if calendar {
                collected.status.calendar = ContextStatus::Present;
            } else {
                collected.status.tasks = ContextStatus::Present;
            }
            for query in queries {
                let remaining = deadline
                    .checked_duration_since(now())
                    .filter(|d| !d.is_zero())
                    .ok_or(Failure::Deadline)?;
                let mut argv = command.to_vec();
                argv.extend(["ls".into(), query.text, "--json".into(), "--no-pull".into()]);
                let bytes = process::output(runner(&argv, remaining));
                if matches!(&bytes, Err(Failure::Interrupted)) {
                    return Err(Failure::Interrupted);
                }
                if now() >= deadline {
                    return Err(Failure::Deadline);
                }
                let bytes = bytes?;
                let row = decode::read(&bytes, calendar, &query.scope)?;
                process::merge(&mut collected, row)?;
                if now() >= deadline {
                    return Err(Failure::Deadline);
                }
            }
            Ok(collected)
        });
        match result {
            Ok(collected) => Ok(collected),
            Err(Failure::Interrupted) => Err(ContextError::Interrupted),
            Err(error) => {
                let mut collected = ContextSnapshot::default();
                let failed = ContextStatus::Failed {
                    detail: error.detail().into(),
                };
                if calendar {
                    collected.status.calendar = failed;
                } else {
                    collected.status.tasks = failed;
                }
                Ok(collected)
            }
        }
    };
    if let Some(queries) = plan.calendar {
        let result = half(queries, true)?;
        snapshot.status.calendar = result.status.calendar;
        snapshot.occasions = result.occasions;
    }
    if let Some(queries) = plan.tasks {
        let result = half(Ok(queries), false)?;
        snapshot.status.tasks = result.status.tasks;
        snapshot.tasks = result.tasks;
    }
    snapshot = finish::normalize(snapshot, request)?;
    if now() >= deadline {
        finish::deadline(&mut snapshot, request);
        snapshot = finish::normalize(snapshot, request)?;
    }
    Ok(snapshot)
}
