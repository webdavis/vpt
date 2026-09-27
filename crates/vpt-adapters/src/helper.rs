mod reply;

use crate::spawn::{self, SpawnError, Status};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::time::Duration;
use vpt_application::ports::{Trash, TrashError};
use vpt_protocol::{HelperVersion, Limits, Trashed, read_bounded};

pub const CALL_DEADLINE: Duration = Duration::from_secs(5);
pub const BUILT_AGAINST_MAJOR: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HelperError {
    Absent,
    MajorMismatch { found: u32 },
    Failed(String),
    Unknown(String),
}

pub struct HelperClient {
    path: PathBuf,
    deadline: Duration,
    version: RefCell<Option<String>>,
    diagnostics: RefCell<Vec<String>>,
}

impl HelperClient {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            deadline: CALL_DEADLINE,
            version: RefCell::new(None),
            diagnostics: RefCell::new(Vec::new()),
        }
    }

    pub fn with_deadline(mut self, deadline: Duration) -> Self {
        self.deadline = deadline;
        self
    }

    pub fn version(&self) -> Result<String, HelperError> {
        self.version_with_env(&[])
    }

    pub fn version_with_env(&self, env: &[(&str, &str)]) -> Result<String, HelperError> {
        self.version_using(&|argv| self.call(argv, env))
    }

    pub fn trash_with_env(&self, path: &Path, env: &[(&str, &str)]) -> Result<PathBuf, TrashError> {
        self.trash_using(path, &|argv| self.call(argv, env))
    }

    pub fn take_diagnostics(&self) -> Vec<String> {
        std::mem::take(&mut *self.diagnostics.borrow_mut())
    }

    fn version_using(
        &self,
        call: &impl Fn(&[String]) -> Result<Value, HelperError>,
    ) -> Result<String, HelperError> {
        if let Some(version) = self.version.borrow().clone() {
            return Ok(version);
        }
        let value = call(&["--version".into()])?;
        reply::schema(&value)?;
        let document: HelperVersion = self.decode(
            value,
            &["schema", "version"],
            "invalid helper version document",
        )?;
        let major = document
            .major()
            .ok_or_else(|| HelperError::Unknown("invalid helper version document".into()))?;
        if major != BUILT_AGAINST_MAJOR {
            return Err(HelperError::MajorMismatch { found: major });
        }
        *self.version.borrow_mut() = Some(document.version.clone());
        Ok(document.version)
    }

    fn trash_using(
        &self,
        path: &Path,
        call: &impl Fn(&[String]) -> Result<Value, HelperError>,
    ) -> Result<PathBuf, TrashError> {
        self.version_using(call).map_err(reply::trash_error)?;
        let path_text = path
            .to_str()
            .ok_or_else(|| TrashError::Failed("the Trash path is not UTF-8".into()))?;
        let argv = ["trash".to_owned(), path_text.to_owned()];
        let reply: Trashed = self
            .decode(
                call(&argv).map_err(reply::trash_error)?,
                &["trashed"],
                "invalid helper trash document",
            )
            .map_err(reply::trash_error)?;
        if reply.trashed != path_text {
            return Err(TrashError::Unknown(
                "the helper trash reply names another path".into(),
            ));
        }
        Ok(path.to_path_buf())
    }

    fn decode<T: DeserializeOwned>(
        &self,
        value: Value,
        supported: &[&str],
        invalid: &str,
    ) -> Result<T, HelperError> {
        let fields = value
            .as_object()
            .ok_or_else(|| HelperError::Unknown(invalid.into()))?;
        let mut diagnostics = self.diagnostics.borrow_mut();
        for key in fields
            .keys()
            .filter(|key| !supported.contains(&key.as_str()))
        {
            if diagnostics.len() >= 16 {
                break;
            }
            let escaped: String = key
                .chars()
                .take(32)
                .flat_map(char::escape_default)
                .collect();
            diagnostics.push(format!("unknown additive helper field: {escaped}"));
        }
        serde_json::from_value(value).map_err(|_| HelperError::Unknown(invalid.into()))
    }

    fn call(&self, argv: &[String], env: &[(&str, &str)]) -> Result<Value, HelperError> {
        self.call_using(argv, env, spawn::run_with_env)
    }

    fn call_using(
        &self,
        argv: &[String],
        env: &[(&str, &str)],
        run: impl FnOnce(
            &[String],
            &[u8],
            Duration,
            usize,
            &[(&str, &str)],
        ) -> Result<spawn::Outcome, SpawnError>,
    ) -> Result<Value, HelperError> {
        let path = self
            .path
            .to_str()
            .ok_or_else(|| HelperError::Failed("the helper path is not UTF-8".into()))?;
        let mut full = vec![path.to_owned()];
        full.extend_from_slice(argv);
        let limits = Limits::incoming();
        let outcome = run(
            &full,
            b"",
            self.deadline,
            limits.bytes.saturating_add(1),
            env,
        )
        .map_err(|error| match error {
            SpawnError::NotFound(_) => HelperError::Absent,
            SpawnError::Io(_) => HelperError::Failed("the helper could not start".into()),
        })?;
        match outcome.status {
            Status::Exited(0) => read_bounded(&outcome.stdout, &limits)
                .map_err(|_| HelperError::Unknown("invalid or oversized helper document".into())),
            Status::Exited(code) => Err(HelperError::Failed(format!("the helper exited {code}"))),
            Status::Signaled(signal) => Err(HelperError::Failed(format!(
                "the helper died on signal {signal}"
            ))),
            Status::DeadlineExceeded => Err(HelperError::Unknown(
                "the helper exceeded its deadline".into(),
            )),
            Status::Interrupted => Err(HelperError::Unknown("interrupted".into())),
        }
    }
}

impl Trash for HelperClient {
    fn trash(&self, path: &Path) -> Result<PathBuf, TrashError> {
        self.trash_with_env(path, &[])
    }

    fn take_diagnostics(&self) -> Vec<String> {
        HelperClient::take_diagnostics(self)
    }
}

#[cfg(test)]
mod tests;
