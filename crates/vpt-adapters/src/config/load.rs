use super::render::render;
use super::schema::{spec_for, table_is_declared};
use crate::{Access, ContainedError, RootDir};
use std::io::Read;
use std::path::Path;
use toml::{Table, Value};

pub const CONFIG_VERSION: i64 = 1;
const UNPARSEABLE: &str = "invalid TOML syntax in configuration";

#[derive(Debug, PartialEq, Eq)]
pub enum ConfigError {
    Missing(String),
    PathEscape(String),
    Unreadable { path: String, detail: String },
    Unparseable(String),
    MissingVersion,
    UnsupportedVersion(i64),
    UnknownKey(String),
    WrongType { key: String, expected: String },
    OutOfRange { key: String, rule: String },
}

impl ConfigError {
    pub fn key(&self) -> Option<&str> {
        match self {
            Self::UnknownKey(key) | Self::WrongType { key, .. } | Self::OutOfRange { key, .. } => {
                Some(key)
            }
            _ => None,
        }
    }
}

pub fn load_file(path: &Path) -> Result<Table, ConfigError> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = parent
        .canonicalize()
        .map_err(|error| config_io(path, error))?;
    let root = RootDir::open(&parent).map_err(|error| config_contained(path, error))?;
    let name = path
        .file_name()
        .ok_or_else(|| config_io(path, std::io::ErrorKind::InvalidInput.into()))?;
    let mut file = root
        .open_file(Path::new(name), Access::Read)
        .map_err(|error| config_contained(path, error))?;
    let mut text = String::new();
    file.read_to_string(&mut text)
        .map_err(|error| config_io(path, error))?;
    load_text(&text)
}

fn config_io(path: &Path, error: std::io::Error) -> ConfigError {
    if error.kind() == std::io::ErrorKind::NotFound {
        ConfigError::Missing(path.display().to_string())
    } else {
        ConfigError::Unreadable {
            path: path.display().to_string(),
            detail: error.kind().to_string(),
        }
    }
}

fn config_contained(path: &Path, error: ContainedError) -> ConfigError {
    match error {
        ContainedError::Io { kind, .. } => config_io(path, kind.into()),
        _ => ConfigError::PathEscape(path.display().to_string()),
    }
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
    super::validate::validate(&merged)?;
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

#[cfg(test)]
#[path = "load/tests/contained.rs"]
mod contained_tests;
