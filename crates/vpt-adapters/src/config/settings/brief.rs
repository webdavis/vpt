use super::*;
use std::time::Duration;
use vpt_application::BriefSettings;
use vpt_domain::{brief::Options, duration::parse_duration};

pub(super) fn from_table(table: &Table) -> Result<BriefSettings, ConfigError> {
    let key = "brief.trigger.lead_time";
    let lead_time = match leaf(table, key)? {
        Value::Integer(0) => 0,
        Value::String(value) => parse_duration(value).map_err(|_| ConfigError::OutOfRange {
            key: key.into(),
            rule: "0 or <n>s|m|h|d".into(),
        })?,
        _ => return Err(wrong(key, "duration")),
    };
    Ok(BriefSettings {
        options: Options {
            max_notes: small(table, "brief.max_notes")? as usize,
            lookback_days: small(table, "brief.lookback_days")?,
            max_spans_per_note: small(table, "brief.max_spans_per_note")? as usize,
        },
        trigger_enabled: boolean(table, "brief.trigger.enabled")?,
        lead_time: Duration::from_secs(lead_time),
    })
}
