use super::{EventLog, LogError, verify_file};
use crate::contained::private_state::effective_user;
use std::io::ErrorKind;
use std::os::unix::fs::{FileExt, MetadataExt};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LogInspection {
    pub device: u64,
    pub inode: u64,
    pub complete_end_offset: u64,
    pub physical_end_offset: u64,
}

impl EventLog {
    pub fn inspect(&self) -> Result<LogInspection, LogError> {
        self.inspect_with(|| Ok(()))
    }

    pub(super) fn inspect_with(
        &self,
        before_confirmation: impl FnOnce() -> Result<(), LogError>,
    ) -> Result<LogInspection, LogError> {
        verify_file(&self.root, &self.file, effective_user())?;
        let metadata = self
            .file
            .metadata()
            .map_err(|error| LogError::Io(error.kind()))?;
        let complete_end_offset = complete_end(&self.file, metadata.len())?;
        before_confirmation()?;
        verify_file(&self.root, &self.file, effective_user())?;
        let current = self
            .file
            .metadata()
            .map_err(|error| LogError::Io(error.kind()))?;
        if current.len() != metadata.len() {
            return Err(LogError::LengthChanged);
        }
        Ok(LogInspection {
            device: metadata.dev(),
            inode: metadata.ino(),
            complete_end_offset,
            physical_end_offset: metadata.len(),
        })
    }
}

fn complete_end(file: &std::fs::File, mut end: u64) -> Result<u64, LogError> {
    let mut buffer = [0; 8192];
    while end != 0 {
        let start = end.saturating_sub(8192);
        let length =
            usize::try_from(end - start).map_err(|_| LogError::Io(ErrorKind::InvalidData))?;
        file.read_exact_at(&mut buffer[..length], start)
            .map_err(|error| LogError::Io(error.kind()))?;
        if let Some(index) = buffer[..length].iter().rposition(|byte| *byte == b'\n') {
            let count =
                u64::try_from(index + 1).map_err(|_| LogError::Io(ErrorKind::InvalidData))?;
            return start
                .checked_add(count)
                .ok_or(LogError::Io(ErrorKind::InvalidData));
        }
        end = start;
    }
    Ok(0)
}
