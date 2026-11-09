pub(crate) mod publication;
pub(crate) mod releases;
pub(crate) mod run_outcome;
pub(crate) mod startup;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use vpt_adapters::config::{ConfigError, default_state_dir, from_table, load_file};
use vpt_application::Settings;
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(crate) mod errors;
pub(crate) mod events;
pub(crate) mod history;
mod ledger;
pub(crate) mod mutation;
mod observations;
pub(crate) use ledger::RuntimeLedger;
pub(crate) use observations::{ledger_from, storage_from};

pub(crate) struct Environment {
    vars: Vec<(OsString, OsString)>,
}

impl Environment {
    pub fn from_process() -> Self {
        Self {
            vars: std::env::vars_os().collect(),
        }
    }

    pub fn var(&self, name: &str) -> Result<Option<String>, ConfigError> {
        self.vars
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| {
                value
                    .to_str()
                    .map(str::to_owned)
                    .ok_or_else(|| ConfigError::WrongType {
                        key: name.into(),
                        expected: "UTF-8 string".into(),
                    })
            })
            .transpose()
    }

    pub fn config_path(&self) -> Result<PathBuf, ConfigError> {
        if let Some(path) = self.var("VPT_CONFIG")? {
            return Ok(path.into());
        }
        if let Some(path) = self.var("XDG_CONFIG_HOME")? {
            return Ok(PathBuf::from(path).join("vpt/config.toml"));
        }
        Ok(self.home_dir()?.join(".config/vpt/config.toml"))
    }

    pub fn home_dir(&self) -> Result<PathBuf, ConfigError> {
        Ok(self.var("HOME")?.unwrap_or_default().into())
    }

    pub fn state_dir_default(&self) -> Result<String, ConfigError> {
        let value = self.var("XDG_STATE_HOME")?;
        Ok(default_state_dir(|_| value.clone()))
    }
}

pub(crate) fn settings_from(
    environment: &Environment,
    config: Option<&Path>,
) -> Result<Settings, Box<ErrorDocument>> {
    let selected = config
        .map(Path::to_path_buf)
        .map_or_else(|| environment.config_path().map_err(config_error), Ok)?;
    let table = load_file(&selected).map_err(config_error)?;
    let home = environment.home_dir().map_err(config_error)?;
    let settings = from_table(&table, &home).map_err(config_error)?;
    Ok(settings)
}

pub(crate) fn config_error(error: ConfigError) -> Box<ErrorDocument> {
    let document = match error {
        ConfigError::Missing(path) => ErrorDocument::new(
            ErrorKind::Config,
            format!("configuration {path} does not exist"),
        ),
        ConfigError::PathEscape(path) => ErrorDocument::new(
            ErrorKind::Refused,
            format!("configuration {path} escapes its checked root"),
        )
        .rule("path_escape"),
        ConfigError::Unreadable { path, detail } => ErrorDocument::new(
            ErrorKind::Config,
            format!("cannot read configuration {path}: {detail}"),
        ),
        ConfigError::Unparseable(_) => {
            ErrorDocument::new(ErrorKind::Config, "configuration does not parse")
        }
        ConfigError::MissingVersion => {
            ErrorDocument::new(ErrorKind::Config, "configuration needs config_version")
        }
        ConfigError::UnsupportedVersion(version) => ErrorDocument::new(
            ErrorKind::Config,
            format!("unsupported configuration version {version}"),
        ),
        ConfigError::UnknownKey(key) => ErrorDocument::new(
            ErrorKind::Config,
            format!("unknown configuration key {key}"),
        ),
        ConfigError::WrongType { key, expected } => {
            ErrorDocument::new(ErrorKind::Config, format!("{key} must be {expected}"))
        }
        ConfigError::OutOfRange { key, rule } => {
            ErrorDocument::new(ErrorKind::Config, format!("{key}: {rule}"))
        }
    };
    Box::new(document)
}


#[cfg(test)]
mod history_tests;
