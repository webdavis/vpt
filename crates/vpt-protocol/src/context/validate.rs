use super::{DecodeError, model};
use std::collections::BTreeSet;
use vpt_domain::{
    brief::{ContextStatus, ContextSummary, ContextTask},
    context::{ContextOccasion, ContextSnapshot},
    occasion::{OccasionDetails, OccasionSource, Participant, time::OccasionTime},
};

pub(super) fn convert(document: model::Document) -> Result<ContextSnapshot, DecodeError> {
    let status = ContextSummary {
        calendar: status(document.context.calendar)?,
        tasks: status(document.context.tasks)?,
    };
    if (!document.occasions.is_empty() && status.calendar != ContextStatus::Present)
        || (!document.tasks.is_empty() && status.tasks != ContextStatus::Present)
    {
        return Err(DecodeError::Invalid("context"));
    }
    let mut occasions = Vec::new();
    let mut keys = BTreeSet::new();
    for row in document.occasions {
        let occasion = ContextOccasion {
            source: source(&row.source)?,
            id: row.id,
            calendar: row.calendar,
            details: OccasionDetails {
                at: OccasionTime::parse(&row.at).map_err(|_| DecodeError::Invalid("at"))?,
                title: row.title,
                duration_secs: row.duration_secs,
                tags: row.tags,
                participants: row
                    .participants
                    .into_iter()
                    .map(|p| Participant {
                        name: p.name,
                        email: p.email,
                    })
                    .collect(),
            },
        };
        let key = occasion
            .key()
            .map_err(|_| DecodeError::Invalid("occasion identity"))?;
        if !occasion.details.is_valid() || !keys.insert(key.encoded()) {
            return Err(DecodeError::Invalid("occasion"));
        }
        occasions.push(occasion);
    }
    keys.clear();
    let mut tasks = Vec::new();
    for row in document.tasks {
        let source = source(&row.source)?;
        if [&row.project, &row.id]
            .iter()
            .any(|s| s.trim().is_empty() || s.chars().any(char::is_control))
            || row.content.trim().is_empty()
            || row.content.contains('\0')
            || row
                .due
                .as_ref()
                .is_some_and(|s| s.trim().is_empty() || s.chars().any(char::is_control))
            || !keys.insert(format!("{}:{}", source.as_str(), row.id))
        {
            return Err(DecodeError::Invalid("task"));
        }
        tasks.push(ContextTask {
            source,
            project: row.project,
            id: row.id,
            content: row.content,
            due: row.due,
            completed: row.completed,
            priority: row.priority,
        });
    }
    Ok(ContextSnapshot {
        status,
        occasions,
        tasks,
    })
}

fn source(value: &str) -> Result<OccasionSource, DecodeError> {
    match value {
        "dam" => Ok(OccasionSource::Dam),
        "google" => Ok(OccasionSource::Google),
        _ => Err(DecodeError::Invalid("source")),
    }
}
fn status(value: model::Status) -> Result<ContextStatus, DecodeError> {
    match (value.status.as_str(), value.reason.as_deref(), value.detail) {
        ("present", None, None) => Ok(ContextStatus::Present),
        ("absent", Some("disabled"), None) => Ok(ContextStatus::Disabled),
        ("absent", Some("unsupported"), None) => Ok(ContextStatus::Unsupported),
        ("absent", Some("failed"), Some(detail))
            if !detail.trim().is_empty()
                && detail.len() <= 1024
                && !detail.chars().any(char::is_control) =>
        {
            Ok(ContextStatus::Failed { detail })
        }
        _ => Err(DecodeError::Invalid("context status")),
    }
}
