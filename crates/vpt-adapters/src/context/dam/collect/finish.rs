use super::*;
use vpt_domain::occasion::time::OccasionTime;

pub(super) fn normalize(
    mut snapshot: ContextSnapshot,
    request: &ContextRequest<'_>,
) -> Result<ContextSnapshot, ContextError> {
    if let Some(window) = request.window {
        match (
            OccasionTime::parse(&window.start.rfc3339()),
            OccasionTime::parse(&window.end.rfc3339()),
        ) {
            (Ok(start), Ok(end)) => snapshot.occasions.retain(|row| {
                row.details.at.cmp_instant(&start).is_ge()
                    && row.details.at.cmp_instant(&end).is_lt()
            }),
            _ => {
                snapshot.occasions.clear();
                snapshot.status.calendar = failed(Failure::Invalid);
            }
        }
    }
    if vpt_protocol::context::encode(&snapshot).is_err() && request.tasks {
        snapshot.tasks.clear();
        snapshot.status.tasks = failed(Failure::Capacity);
    }
    if vpt_protocol::context::encode(&snapshot).is_err() && request.window.is_some() {
        snapshot.occasions.clear();
        snapshot.status.calendar = failed(Failure::Capacity);
    }
    let text =
        vpt_protocol::context::encode(&snapshot).map_err(|_| ContextError::InvalidDocument)?;
    Ok(vpt_protocol::context::decode(text.as_bytes())
        .map_err(|_| ContextError::InvalidDocument)?
        .snapshot)
}
pub(super) fn deadline(snapshot: &mut ContextSnapshot, request: &ContextRequest<'_>) {
    if request.tasks && snapshot.status.tasks == ContextStatus::Present {
        snapshot.tasks.clear();
        snapshot.status.tasks = failed(Failure::Deadline);
    } else if !request.tasks
        && request.window.is_some()
        && snapshot.status.calendar == ContextStatus::Present
    {
        snapshot.occasions.clear();
        snapshot.status.calendar = failed(Failure::Deadline);
    }
}
fn failed(error: Failure) -> ContextStatus {
    ContextStatus::Failed {
        detail: error.detail().into(),
    }
}
