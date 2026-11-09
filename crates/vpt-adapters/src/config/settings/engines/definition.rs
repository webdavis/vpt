use super::{nonempty, required};
use crate::config::{ConfigError, get};
use toml::Table;
use vpt_application::engines::{EngineConfig, EngineKind};

pub(super) fn read(table: &Table, name: &str) -> Result<EngineConfig, ConfigError> {
    let prefix = format!("engines.{name}");
    if name.is_empty()
        || name.len() > 64
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(required(&prefix));
    }
    let kind = match nonempty(table, &format!("{prefix}.kind"))? {
        "apple" => EngineKind::Apple,
        "whisply" => EngineKind::Whisply {
            command: if get(table, &format!("{prefix}.command")).is_some() {
                command(table, &format!("{prefix}.command"))?
            } else {
                vec!["whisply".into()]
            },
            device: text_or(table, &format!("{prefix}.device"), "mlx")?,
            model: text_or(table, &format!("{prefix}.model"), "large-v3-turbo")?,
        },
        "command" => EngineKind::Command {
            command: command(table, &format!("{prefix}.command"))?,
            family: nonempty(table, &format!("{prefix}.family"))?.into(),
            local: get(table, &format!("{prefix}.local"))
                .and_then(toml::Value::as_bool)
                .ok_or_else(|| required(&format!("{prefix}.local")))?,
        },
        _ => return Err(required(&format!("{prefix}.kind"))),
    };
    Ok(EngineConfig {
        name: name.into(),
        kind,
    })
}

fn command(table: &Table, key: &str) -> Result<Vec<String>, ConfigError> {
    if get(table, key).is_none() {
        return Err(required(key));
    }
    let argv = super::super::strings(table, key)?;
    if argv.first().is_none_or(|program| program.trim().is_empty())
        || argv.iter().any(|arg| arg.contains('\0'))
    {
        return Err(required(key));
    }
    Ok(argv)
}

fn text_or(table: &Table, key: &str, default: &str) -> Result<String, ConfigError> {
    if get(table, key).is_some() {
        nonempty(table, key).map(str::to_owned)
    } else {
        Ok(default.into())
    }
}
