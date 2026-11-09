use super::*;
use serde::Serialize;
use vpt_domain::brief::ContextStatus;

#[derive(Serialize)]
pub(super) struct Document<'a> {
    schema: &'static str,
    context: Summary<'a>,
    occasions: Vec<Occasion<'a>>,
    tasks: Vec<Task<'a>>,
}
#[derive(Serialize)]
struct Summary<'a> {
    calendar: Status<'a>,
    tasks: Status<'a>,
}
#[derive(Serialize)]
struct Status<'a> {
    status: &'static str,
    reason: Option<&'static str>,
    detail: Option<&'a str>,
}
impl<'a> From<&'a ContextStatus> for Status<'a> {
    fn from(value: &'a ContextStatus) -> Self {
        Self {
            status: value.status(),
            reason: value.reason(),
            detail: value.detail(),
        }
    }
}
#[derive(Serialize)]
struct Occasion<'a> {
    source: &'static str,
    id: &'a str,
    calendar: Option<&'a str>,
    title: &'a str,
    at: &'a str,
    duration_secs: Option<u64>,
    participants: Vec<Participant<'a>>,
    tags: &'a [String],
}
#[derive(Serialize)]
struct Participant<'a> {
    name: &'a str,
    email: Option<&'a str>,
}
#[derive(Serialize)]
struct Task<'a> {
    source: &'static str,
    project: &'a str,
    id: &'a str,
    content: &'a str,
    due: Option<&'a str>,
    completed: bool,
    priority: Option<u32>,
}
impl<'a> Document<'a> {
    pub(super) fn new(snapshot: &'a ContextSnapshot) -> Self {
        Self {
            schema: SCHEMA,
            context: Summary {
                calendar: (&snapshot.status.calendar).into(),
                tasks: (&snapshot.status.tasks).into(),
            },
            occasions: snapshot
                .occasions
                .iter()
                .map(|row| Occasion {
                    source: row.source.as_str(),
                    id: &row.id,
                    calendar: row.calendar.as_deref(),
                    title: &row.details.title,
                    at: row.details.at.as_str(),
                    duration_secs: row.details.duration_secs,
                    tags: &row.details.tags,
                    participants: row
                        .details
                        .participants
                        .iter()
                        .map(|p| Participant {
                            name: &p.name,
                            email: p.email.as_deref(),
                        })
                        .collect(),
                })
                .collect(),
            tasks: snapshot
                .tasks
                .iter()
                .map(|row| Task {
                    source: row.source.as_str(),
                    project: &row.project,
                    id: &row.id,
                    content: &row.content,
                    due: row.due.as_deref(),
                    completed: row.completed,
                    priority: row.priority,
                })
                .collect(),
        }
    }
}
