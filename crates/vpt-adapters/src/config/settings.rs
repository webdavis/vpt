use super::{ConfigError, get};
use std::path::{Path, PathBuf};
use toml::{Table, Value};
use vpt_application::{RetentionSettings, Settings, SourceSettings, StorePaths};
use vpt_domain::duration::parse_duration;
use vpt_domain::layout::StoreKey;
use vpt_domain::retention::Hold;

pub const DEFAULT_HOME: &str = "~/.vpt";

pub fn from_table(table: &Table, home_dir: &Path) -> Result<Settings, ConfigError> {
    let home_text = text(table, "home.path")?;
    let home = expand(home_text, home_dir, None);
    let symlink_text = text(table, "home.symlink_target")?;
    let symlink_target = if symlink_text.is_empty() {
        None
    } else {
        if home_text != DEFAULT_HOME {
            return Err(ConfigError::OutOfRange {
                key: "home.symlink_target".into(),
                rule: "home.path must stay at its default when a symlink target is set".into(),
            });
        }
        Some(expand(symlink_text, home_dir, None))
    };
    let store = |key| text(table, key).map(|raw| expand(raw, home_dir, Some(&home)));
    let stores = StorePaths {
        audio: store("stores.audio")?,
        transcripts: store("stores.transcripts")?,
        analysis: store("stores.analysis")?,
        briefs: store("stores.briefs")?,
        engine_outputs: store("stores.engine_outputs")?,
        drafts: store("stores.drafts")?,
        released: store("stores.released")?,
    };
    let holds = StoreKey::all()
        .into_iter()
        .map(|key| {
            hold(table, &format!("retention.hold.{}", key.key_name())).map(|hold| (key, hold))
        })
        .collect::<Result<_, _>>()?;
    Ok(Settings {
        config_version: small(table, "config_version")?,
        home,
        state_dir: expand(text(table, "home.state_dir")?, home_dir, None),
        symlink_target,
        helper_path: PathBuf::from(text(table, "helper.path")?),
        stores,
        source: SourceSettings {
            recordings_dir: expand(text(table, "source.recordings_dir")?, home_dir, None),
            read_titles: boolean(table, "source.read_titles")?,
            quiet_period_secs: u64::from(small(table, "source.quiet_period_secs")?),
            deferral_page_threshold: small(table, "source.deferral_page_threshold")?,
            max_audio_bytes: wide(table, "source.max_audio_bytes")?,
        },
        report_command: strings(table, "report.command")?,
        alert_command: strings(table, "alert.command")?,
        alert_aggregate_after: small(table, "alert.aggregate_after")?,
        retention: RetentionSettings {
            enabled: boolean(table, "retention.enabled")?,
            include_audio: boolean(table, "retention.include_audio")?,
            holds,
        },
    })
}

fn expand(raw: &str, home_dir: &Path, home: Option<&Path>) -> PathBuf {
    if let Some(rest) = raw.strip_prefix("~/") {
        return home_dir.join(rest);
    }
    if raw == "~" {
        return home_dir.to_path_buf();
    }
    if let (Some(rest), Some(home)) = (raw.strip_prefix("<home>/"), home) {
        return home.join(rest);
    }
    PathBuf::from(raw)
}

fn leaf<'a>(table: &'a Table, key: &str) -> Result<&'a Value, ConfigError> {
    get(table, key).ok_or_else(|| ConfigError::UnknownKey(key.into()))
}

fn text<'a>(table: &'a Table, key: &str) -> Result<&'a str, ConfigError> {
    leaf(table, key)?
        .as_str()
        .ok_or_else(|| wrong(key, "string"))
}

fn boolean(table: &Table, key: &str) -> Result<bool, ConfigError> {
    leaf(table, key)?
        .as_bool()
        .ok_or_else(|| wrong(key, "boolean"))
}

fn small(table: &Table, key: &str) -> Result<u32, ConfigError> {
    let value = leaf(table, key)?
        .as_integer()
        .ok_or_else(|| wrong(key, "integer"))?;
    u32::try_from(value).map_err(|_| wrong(key, "integer"))
}

fn wide(table: &Table, key: &str) -> Result<u64, ConfigError> {
    let value = leaf(table, key)?
        .as_integer()
        .ok_or_else(|| wrong(key, "integer"))?;
    u64::try_from(value).map_err(|_| wrong(key, "integer"))
}

fn strings(table: &Table, key: &str) -> Result<Vec<String>, ConfigError> {
    let items = leaf(table, key)?
        .as_array()
        .ok_or_else(|| wrong(key, "list of strings"))?;
    items
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| wrong(key, "list of strings"))
        })
        .collect()
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

fn wrong(key: &str, expected: &str) -> ConfigError {
    ConfigError::WrongType {
        key: key.into(),
        expected: expected.into(),
    }
}

#[cfg(test)]
mod tests;
