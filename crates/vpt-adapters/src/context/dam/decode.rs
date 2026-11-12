use super::*;
use serde::de::DeserializeOwned;
use vpt_domain::{brief::ContextStatus, occasion::OccasionSource, occasion::Participant};
mod event;
mod model;
use crate::context::time;
pub(super) use event::Event;

pub(super) fn read(bytes: &[u8], scope: &str) -> Result<ContextSnapshot, Failure> {
    let reply: model::Ls = parse(bytes)?;
    let mut snapshot = ContextSnapshot::default();
    snapshot.status.tasks = ContextStatus::Present;
    for row in reply.objects {
        if !within(&row.path, &[scope]) || row.kind != "task" {
            return Err(Failure::Invalid);
        }
        let task = row.task.ok_or(Failure::Invalid)?;
        if !task.done {
            snapshot.tasks.push(vpt_domain::brief::ContextTask {
                source: OccasionSource::Dam,
                project: if row.path.is_empty() {
                    "/".into()
                } else {
                    row.path
                },
                id: row.oid,
                content: row.subject,
                due: task.due,
                completed: false,
                priority: Some(task.priority),
            });
        }
    }
    validated(snapshot)
}

pub(super) fn agenda(bytes: &[u8], scopes: &[String]) -> Result<Vec<Event>, Failure> {
    let reply: model::Agenda = parse(bytes)?;
    let mut events = Vec::new();
    for row in reply.events {
        if !within(&row.path, scopes) {
            return Err(Failure::Invalid);
        }
        events.extend(event::convert(row)?);
    }
    Ok(events)
}

pub(super) fn attendees(bytes: &[u8], oid: &str) -> Result<Vec<Participant>, Failure> {
    let shown: model::Shown = parse(bytes)?;
    if shown.oid != oid || shown.kind != "event" {
        return Err(Failure::Invalid);
    }
    event::participants(shown.event.ok_or(Failure::Invalid)?)
}

pub(super) fn validated(snapshot: ContextSnapshot) -> Result<ContextSnapshot, Failure> {
    let text = vpt_protocol::context::encode(&snapshot).map_err(|_| Failure::Invalid)?;
    Ok(vpt_protocol::context::decode(text.as_bytes())
        .map_err(|_| Failure::Invalid)?
        .snapshot)
}

fn parse<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, Failure> {
    let value = vpt_protocol::read_bounded(bytes, &vpt_protocol::Limits::incoming())
        .map_err(|_| Failure::Invalid)?;
    serde_json::from_value(value).map_err(|_| Failure::Invalid)
}

fn within(path: &str, scopes: &[impl AsRef<str>]) -> bool {
    let path = path.trim_matches('/');
    let wellformed = path.is_empty()
        || !path
            .split('/')
            .any(|s| s.is_empty() || s == "." || s == "..");
    wellformed
        && scopes.iter().any(|scope| {
            let scope = scope.as_ref().trim_matches('/');
            scope.is_empty() || path == scope || path.starts_with(&format!("{scope}/"))
        })
}
