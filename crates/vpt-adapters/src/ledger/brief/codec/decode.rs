use super::*;
use vpt_domain::{
    brief::*,
    occasion::{OccasionDetails, OccasionId, OccasionSource, Participant, time::OccasionTime},
    time::UtcInstant,
};
mod evidence;

pub(super) fn snapshot(raw: model::Snapshot) -> Result<Snapshot, LedgerError> {
    if raw.schema != "vpt.saved-brief/1" {
        return Err(invalid());
    }
    let snapshot = Snapshot {
        id: OccasionId::parse(&raw.id).map_err(|_| invalid())?,
        source: provider(&raw.source)?,
        occasion: OccasionDetails {
            at: OccasionTime::parse(&raw.occasion.at).map_err(|_| invalid())?,
            title: raw.occasion.title,
            duration_secs: raw.occasion.duration_secs,
            tags: raw.occasion.tags,
            participants: raw
                .occasion
                .participants
                .into_iter()
                .map(|(name, email)| Participant { name, email })
                .collect(),
        },
        generated_at: UtcInstant {
            secs: raw.generated_at,
        },
        context: ContextSummary {
            calendar: half(raw.context.calendar)?,
            tasks: half(raw.context.tasks)?,
        },
        tasks: raw.tasks.into_iter().map(task).collect::<Result<_, _>>()?,
        pack: evidence::pack(raw.pack)?,
    };
    snapshot.document().validate().map_err(|_| invalid())?;
    Ok(snapshot)
}
fn provider(value: &str) -> Result<OccasionSource, LedgerError> {
    match value {
        "manual" => Ok(OccasionSource::Manual),
        "dam" => Ok(OccasionSource::Dam),
        "google" => Ok(OccasionSource::Google),
        _ => Err(invalid()),
    }
}
fn half(raw: model::Half) -> Result<ContextStatus, LedgerError> {
    match (raw.status.as_str(), raw.reason.as_deref(), raw.detail) {
        ("present", None, None) => Ok(ContextStatus::Present),
        ("absent", Some("disabled"), None) => Ok(ContextStatus::Disabled),
        ("absent", Some("unsupported"), None) => Ok(ContextStatus::Unsupported),
        ("absent", Some("failed"), Some(detail)) => Ok(ContextStatus::Failed { detail }),
        _ => Err(invalid()),
    }
}
fn task(raw: model::Task) -> Result<ContextTask, LedgerError> {
    Ok(ContextTask {
        source: provider(&raw.source)?,
        project: raw.project,
        id: raw.id,
        content: raw.content,
        due: raw.due,
        completed: raw.completed,
        priority: raw.priority,
    })
}
