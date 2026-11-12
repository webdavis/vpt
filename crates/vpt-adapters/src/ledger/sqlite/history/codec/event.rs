mod detail;
mod retention_counts;
use super::{corrupt, count, field, recording, signed, string};
use serde_json::{Value, json};
use std::path::PathBuf;
use vpt_application::ports::LedgerError;
use vpt_domain::history::{
    Event, EventCounts, EventDetail, EventKind, EventPath, EventPathKey, EventState,
};
use vpt_domain::time::UtcInstant;

pub(super) fn encode(event: &Event) -> Result<Value, LedgerError> {
    let detail = detail::encode(event.detail);
    let counts = &event.counts;
    let paths: Vec<_> = event
        .paths
        .iter()
        .map(|entry| {
            Ok(json!({"key":entry.key.as_str(),"path":entry.path.to_str().ok_or_else(corrupt)?}))
        })
        .collect::<Result<_, LedgerError>>()?;
    Ok(json!({
        "kind":event.kind.as_str(),
        "state":event.state.as_str(),
        "recording":event.recording.as_ref().map(|id| id.as_str()),
        "occasion":event.occasion.as_ref().map(|id| id.as_str()),
        "detail":detail,
        "counts":{
            "retention":counts.retention.map(retention_counts::encode),
            "numeric":counts.numeric,
            "proper_noun":counts.proper_noun,
            "low_confidence":counts.low_confidence,
            "formatting":counts.formatting,
            "other":counts.other,
            "unsupported":counts.unsupported,
            "unsourced":counts.unsourced,
            "bad_reference":counts.bad_reference,
            "built_on_flagged_text":counts.built_on_flagged_text,
            "agreed_unverified":counts.agreed_unverified,
            "diagnostics":counts.diagnostics,
        },
        "paths":paths,
        "occurred_at":event.occurred_at.secs,
    }))
}

pub(super) fn decode(value: &Value) -> Result<Event, LedgerError> {
    let kind = match string(value, "kind")? {
        "review_needed" => EventKind::ReviewNeeded,
        "transcribe_failed" => EventKind::TranscribeFailed,
        "ingest_failed" => EventKind::IngestFailed,
        "deferred" => EventKind::Deferred,
        "brief_written" => EventKind::BriefWritten,
        "retention" => EventKind::Retention,
        "config_refused" => EventKind::ConfigRefused,
        "transcribed" => EventKind::Transcribed,
        "analyzed" => EventKind::Analyzed,
        _ => return Err(corrupt()),
    };
    let state = match string(value, "state")? {
        "needs_attention" => EventState::NeedsAttention,
        "failed" => EventState::Failed,
        "done" => EventState::Done,
        _ => return Err(corrupt()),
    };
    let detail = detail::decode(field(value, "detail")?)?;
    let stored_counts = field(value, "counts")?;
    let counts = EventCounts {
        retention: retention_counts::decode(stored_counts.get("retention"))?,
        numeric: count(stored_counts, "numeric")?,
        proper_noun: count(stored_counts, "proper_noun")?,
        low_confidence: count(stored_counts, "low_confidence")?,
        formatting: count(stored_counts, "formatting")?,
        other: count(stored_counts, "other")?,
        unsupported: count(stored_counts, "unsupported")?,
        unsourced: optional_count(stored_counts, "unsourced")?,
        bad_reference: optional_count(stored_counts, "bad_reference")?,
        built_on_flagged_text: optional_count(stored_counts, "built_on_flagged_text")?,
        agreed_unverified: count(stored_counts, "agreed_unverified")?,
        diagnostics: count(stored_counts, "diagnostics")?,
    };
    let paths = field(value, "paths")?
        .as_array()
        .ok_or_else(corrupt)?
        .iter()
        .map(|entry| {
            let key = match string(entry, "key")? {
                "note" => EventPathKey::Note,
                "audio" => EventPathKey::Audio,
                "brief" => EventPathKey::Brief,
                "archive" => EventPathKey::Archive,
                "source" => EventPathKey::Source,
                _ => return Err(corrupt()),
            };
            Ok(EventPath {
                key,
                path: PathBuf::from(string(entry, "path")?),
            })
        })
        .collect::<Result<Vec<_>, LedgerError>>()?;
    Ok(Event {
        kind,
        state,
        occasion: match value.get("occasion") {
            None | Some(Value::Null) => None,
            Some(value) => Some(
                vpt_domain::occasion::OccasionId::parse(value.as_str().ok_or_else(corrupt)?)
                    .map_err(|_| corrupt())?,
            ),
        },
        recording: recording(value, "recording")?,
        detail,
        counts,
        paths,
        occurred_at: UtcInstant {
            secs: signed(value, "occurred_at")?,
        },
    })
}

fn optional_count(value: &Value, key: &str) -> Result<u32, LedgerError> {
    if value.get(key).is_none() {
        Ok(0)
    } else {
        count(value, key)
    }
}
