use super::*;
use serde::Deserialize;
use vpt_domain::{brief::ContextStatus, occasion::OccasionSource};
mod event;
mod model;
use crate::context::time;

pub(super) fn read(bytes: &[u8], calendar: bool, scope: &str) -> Result<ContextSnapshot, Failure> {
    let value = vpt_protocol::read_bounded(bytes, &vpt_protocol::Limits::incoming())
        .map_err(|_| Failure::Invalid)?;
    #[derive(Deserialize)]
    struct Reply {
        objects: Vec<model::Object>,
    }
    let reply: Reply = serde_json::from_value(value).map_err(|_| Failure::Invalid)?;
    let mut snapshot = ContextSnapshot::default();
    if calendar {
        snapshot.status.calendar = ContextStatus::Present;
    } else {
        snapshot.status.tasks = ContextStatus::Present;
    }
    for row in reply.objects {
        let path = row.path.trim_matches('/');
        let scope = scope.trim_matches('/');
        if (!scope.is_empty() && path != scope && !path.starts_with(&format!("{scope}/")))
            || (!path.is_empty()
                && path
                    .split('/')
                    .any(|s| s.is_empty() || s == "." || s == ".."))
        {
            return Err(Failure::Invalid);
        }
        if calendar {
            if row.kind != "event" {
                return Err(Failure::Invalid);
            }
            if let Some(event) = event::convert(row)? {
                snapshot.occasions.push(event);
            }
        } else {
            if row.kind != "task" {
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
    }
    let text = vpt_protocol::context::encode(&snapshot).map_err(|_| Failure::Invalid)?;
    Ok(vpt_protocol::context::decode(text.as_bytes())
        .map_err(|_| Failure::Invalid)?
        .snapshot)
}
