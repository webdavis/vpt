use std::ffi::OsString;
use std::path::PathBuf;
use vpt_adapters::config::{ConfigError, default_state_dir};

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
