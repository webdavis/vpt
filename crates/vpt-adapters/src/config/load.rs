use super::render::render;
use super::schema::{spec_for, table_is_declared};
use std::path::Path;
use toml::{Table, Value};

pub const CONFIG_VERSION: i64 = 1;
const UNPARSEABLE: &str = "invalid TOML syntax in configuration";

#[derive(Debug, PartialEq, Eq)]
pub enum ConfigError {
    Missing(String),
    Unreadable { path: String, detail: String },
    Unparseable(String),
    MissingVersion,
    UnsupportedVersion(i64),
    UnknownKey(String),
    WrongType { key: String, expected: String },
    OutOfRange { key: String, rule: String },
}

pub fn load_file(path: &Path) -> Result<Table, ConfigError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(ConfigError::Missing(path.display().to_string()));
        }
        Err(error) => {
            return Err(ConfigError::Unreadable {
                path: path.display().to_string(),
                detail: error.kind().to_string(),
            });
        }
    };
    load_text(&text)
}

pub fn load_text(text: &str) -> Result<Table, ConfigError> {
    let operator: Table = text
        .parse()
        .map_err(|_: toml::de::Error| ConfigError::Unparseable(UNPARSEABLE.into()))?;
    match operator.get("config_version") {
        None => return Err(ConfigError::MissingVersion),
        Some(Value::Integer(CONFIG_VERSION)) => {}
        Some(Value::Integer(other)) => return Err(ConfigError::UnsupportedVersion(*other)),
        Some(_) => {
            return Err(ConfigError::WrongType {
                key: "config_version".into(),
                expected: "integer".into(),
            });
        }
    }
    refuse_unknown_keys(&operator, "")?;
    let mut merged: Table = render(None)
        .parse()
        .map_err(|_: toml::de::Error| ConfigError::Unparseable(UNPARSEABLE.into()))?;
    merge_into(&mut merged, operator);
    Ok(merged)
}

fn refuse_unknown_keys(table: &Table, prefix: &str) -> Result<(), ConfigError> {
    for (name, value) in table {
        let path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}.{name}")
        };
        if name.contains('.') {
            return Err(ConfigError::UnknownKey(path));
        }
        match value {
            Value::Table(inner) => {
                if let Some(kind) = spec_for(&path) {
                    return Err(ConfigError::WrongType {
                        key: path,
                        expected: kind.expected().into(),
                    });
                }
                if !table_is_declared(&path) {
                    return Err(ConfigError::UnknownKey(path));
                }
                refuse_unknown_keys(inner, &path)?;
            }
            _ => {
                if spec_for(&path).is_none() {
                    if table_is_declared(&path) {
                        return Err(ConfigError::WrongType {
                            key: path,
                            expected: "table".into(),
                        });
                    }
                    return Err(ConfigError::UnknownKey(path));
                }
            }
        }
    }
    Ok(())
}

fn merge_into(base: &mut Table, over: Table) {
    for (name, value) in over {
        match (base.get_mut(&name), value) {
            (Some(Value::Table(existing)), Value::Table(incoming)) => {
                merge_into(existing, incoming)
            }
            (_, incoming) => {
                base.insert(name, incoming);
            }
        }
    }
}

pub fn get<'a>(table: &'a Table, path: &str) -> Option<&'a Value> {
    let mut segments = path.split('.');
    let first = segments.next()?;
    let mut current = table.get(first)?;
    for segment in segments {
        current = current.as_table()?.get(segment)?;
    }
    Some(current)
}
#[cfg(test)]
mod tests;
