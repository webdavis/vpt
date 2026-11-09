use super::{boolean, leaf, wrong};
use crate::config::ConfigError;
use toml::{Table, Value};
use vpt_application::RetentionSettings;
use vpt_domain::{duration::parse_duration, layout::StoreKey, retention::Hold};

pub(super) fn from_table(table: &Table) -> Result<RetentionSettings, ConfigError> {
    let holds = StoreKey::all()
        .into_iter()
        .map(|key| {
            hold(table, &format!("retention.hold.{}", key.key_name())).map(|hold| (key, hold))
        })
        .collect::<Result<_, _>>()?;
    Ok(RetentionSettings {
        enabled: boolean(table, "retention.enabled")?,
        include_audio: boolean(table, "retention.include_audio")?,
        holds,
    })
}

fn hold(table: &Table, key: &str) -> Result<Hold, ConfigError> {
    match leaf(table, key)? {
        Value::Integer(0) => Ok(Hold::never()),
        Value::String(duration) => {
            parse_duration(duration)
                .map(Hold::of_seconds)
                .map_err(|_| ConfigError::OutOfRange {
                    key: key.into(),
                    rule: "0 or <n>s|m|h|d".into(),
                })
        }
        _ => Err(wrong(key, "duration")),
    }
}
