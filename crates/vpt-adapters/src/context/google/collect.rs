use super::*;
use std::collections::BTreeSet;
use std::time::{Duration, Instant};
use vpt_domain::brief::ContextStatus;
mod session;
mod validate;
pub(super) use validate::validate;
pub(super) fn run(
    credentials: &GoogleCredentials,
    request: &ContextRequest<'_>,
    send: impl FnMut(Request<'_>, Duration) -> Result<Vec<u8>, Failure>,
    mut now: impl FnMut() -> Instant,
    interrupted: impl FnMut() -> bool,
) -> Result<ContextSnapshot, ContextError> {
    let mut session = session::Session {
        deadline: now() + Duration::from_secs(30),
        send,
        now,
        interrupted,
    };
    validate(credentials, request)?;
    let mut snapshot = ContextSnapshot::default();
    snapshot.status.tasks = if request.tasks {
        ContextStatus::Unsupported
    } else {
        ContextStatus::Disabled
    };
    let Some(window) = request.window else {
        return Ok(snapshot);
    };
    let result = (|| {
        use vpt_domain::occasion::time::OccasionTime;
        let start = OccasionTime::parse(&window.start.rfc3339()).map_err(|_| Failure::Invalid)?;
        let end = OccasionTime::parse(&window.end.rfc3339()).map_err(|_| Failure::Invalid)?;
        if window.end <= window.start {
            return Err(Failure::Invalid);
        }
        let token = token::decode(&session.fetch(Request::Refresh(credentials))?)?;
        let mut calendars = BTreeSet::new();
        snapshot.status.calendar = ContextStatus::Present;
        for calendar in request.calendars {
            if !calendars.insert(calendar) {
                continue;
            }
            let mut cursor = None;
            let mut cursors = BTreeSet::new();
            for number in 1..=10 {
                let bytes = session.fetch(Request::Events {
                    token: &token.0,
                    calendar,
                    window,
                    page: cursor.as_deref(),
                })?;
                let page = page::decode(&bytes, calendar)?;
                for row in page.occasions {
                    if row.details.at.cmp_instant(&start).is_lt()
                        || row.details.at.cmp_instant(&end).is_ge()
                    {
                        continue;
                    }
                    match snapshot
                        .occasions
                        .iter()
                        .find(|other| other.id == row.id && other.calendar == row.calendar)
                    {
                        Some(other) if other != &row => return Err(Failure::Invalid),
                        Some(_) => (),
                        None => snapshot.occasions.push(row),
                    }
                }
                vpt_protocol::context::encode(&snapshot).map_err(|_| Failure::Capacity)?;
                session.check()?;
                let Some(next) = page.next else {
                    break;
                };
                if number == 10 {
                    return Err(Failure::PageLimit);
                }
                if !cursors.insert(next.clone()) {
                    return Err(Failure::Invalid);
                }
                cursor = Some(next);
            }
        }
        let encoded = vpt_protocol::context::encode(&snapshot).map_err(|_| Failure::Capacity)?;
        snapshot = vpt_protocol::context::decode(encoded.as_bytes())
            .map_err(|_| Failure::Invalid)?
            .snapshot;
        session.check()
    })();
    match result {
        Ok(()) => Ok(snapshot),
        Err(Failure::Interrupted) => Err(ContextError::Interrupted),
        Err(error) => {
            snapshot.occasions.clear();
            snapshot.status.calendar = ContextStatus::Failed {
                detail: error.detail().into(),
            };
            let text = vpt_protocol::context::encode(&snapshot)
                .map_err(|_| ContextError::InvalidDocument)?;
            Ok(vpt_protocol::context::decode(text.as_bytes())
                .map_err(|_| ContextError::InvalidDocument)?
                .snapshot)
        }
    }
}
#[cfg(test)]
mod boundaries;
#[cfg(test)]
mod tests;
