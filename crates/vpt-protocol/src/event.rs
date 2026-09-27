use serde_json::{Map, Value, json};
use vpt_domain::history::Event;
use vpt_domain::run::HistoryRecord;
use vpt_domain::time::{UtcInstant, UtcOffset};

mod history;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventEncodeError {
    NonUtf8Path,
    TimestampOutOfRange,
}

pub fn encode_hook(event: &Event) -> Result<Vec<u8>, EventEncodeError> {
    let mut value = event_value(event)?;
    let fields = value.as_object_mut().expect("event object");
    fields.insert("schema".into(), json!("vpt.event/1"));
    let channel = match event.kind {
        vpt_domain::history::EventKind::BriefWritten
        | vpt_domain::history::EventKind::Retention => "report",
        _ => "alert",
    };
    fields.insert("channel".into(), json!(channel));
    Ok(value.to_string().into_bytes())
}

pub fn encode_history(
    record: &HistoryRecord,
    appended_at: UtcInstant,
) -> Result<String, EventEncodeError> {
    history::encode(record, appended_at)
}

fn event_value(event: &Event) -> Result<Value, EventEncodeError> {
    let mut paths = Map::new();
    for entry in &event.paths {
        paths.insert(entry.key.as_str().into(), json!(path(&entry.path)?));
    }
    Ok(json!({
        "event": event.kind.as_str(),
        "state": event.state.as_str(),
        "recording": event.recording.as_ref().map(|id| id.as_str()),
        "detail": event.detail_text(),
        "counts": {
            "numeric": event.counts.numeric,
            "proper_noun": event.counts.proper_noun,
            "low_confidence": event.counts.low_confidence,
            "formatting": event.counts.formatting,
            "other": event.counts.other,
            "unsupported": event.counts.unsupported,
            "agreed_unverified": event.counts.agreed_unverified,
            "diagnostics": event.counts.diagnostics,
        },
        "paths": paths,
        "occurred_at": timestamp(event.occurred_at)?,
    }))
}

fn path(value: &std::path::Path) -> Result<&str, EventEncodeError> {
    value.to_str().ok_or(EventEncodeError::NonUtf8Path)
}

fn timestamp(value: UtcInstant) -> Result<String, EventEncodeError> {
    if !(0..=9999).contains(&value.civil(UtcOffset { secs: 0 }).year) {
        return Err(EventEncodeError::TimestampOutOfRange);
    }
    Ok(value.rfc3339())
}

#[cfg(test)]
mod tests;
