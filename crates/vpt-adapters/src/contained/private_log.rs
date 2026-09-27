use super::{PrivateStateError, RootDir};
use std::fs::File;
use std::io::ErrorKind;

mod append;
mod files;
mod inspect;
pub use append::{AppendEffects, AppendFailure, AppendReceipt, AppendSync, TailRepair};
use files::verify_file;
pub use inspect::LogInspection;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogRefusal {
    NotRegular,
    Owner,
    Mode,
    MultipleLinks,
    Replaced,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogError {
    State(PrivateStateError),
    Unsafe(LogRefusal),
    Io(ErrorKind),
    InvalidLine,
    LengthChanged,
}

#[derive(Debug)]
pub struct EventLog {
    root: RootDir,
    file: File,
}

impl RootDir {
    pub fn open_event_log(&self) -> Result<EventLog, LogError> {
        self.open_event_log_with_mode(0o600)
    }

    fn open_event_log_with_mode(&self, creation_mode: u32) -> Result<EventLog, LogError> {
        self.validate_private_state().map_err(LogError::State)?;
        let file = files::acquire(self, creation_mode)?;
        verify_file(self, &file, super::private_state::effective_user())?;
        Ok(EventLog {
            root: self
                .try_clone()
                .map_err(|error| LogError::State(PrivateStateError::Contained(error)))?,
            file,
        })
    }
}

#[cfg(test)]
mod tests;
