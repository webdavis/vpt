use super::{ContainedError, RootDir};
use std::os::unix::fs::MetadataExt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrivateStateError {
    Contained(ContainedError),
    Owner,
    Mode,
}

impl RootDir {
    pub fn validate_private_state(&self) -> Result<(), PrivateStateError> {
        validate_as(self, effective_user())
    }
}

pub(crate) fn effective_user() -> u32 {
    // SAFETY: geteuid has no arguments and only observes this process's effective identity.
    unsafe { libc::geteuid() }
}

fn validate_as(root: &RootDir, owner: u32) -> Result<(), PrivateStateError> {
    root.revalidate().map_err(PrivateStateError::Contained)?;
    let metadata = root
        .file
        .metadata()
        .map_err(|error| PrivateStateError::Contained(super::io(root.path(), error)))?;
    if metadata.uid() != owner {
        return Err(PrivateStateError::Owner);
    }
    if metadata.mode() & 0o7777 != 0o700 {
        return Err(PrivateStateError::Mode);
    }
    root.revalidate().map_err(PrivateStateError::Contained)
}

#[cfg(test)]
mod tests;
