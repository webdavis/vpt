use super::errors::{contained_error, root_error};
use std::path::Path;
use std::time::Duration;
use vpt_adapters::{LockError, PrivateStateError, RootDir, Roots, WriteLock};
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(crate) fn state(roots: &Roots) -> Result<RootDir, Box<ErrorDocument>> {
    roots
        .create_state_dir()
        .map_err(|error| Box::new(root_error(&error)))?;
    let state =
        RootDir::open(&roots.state_dir).map_err(|error| Box::new(contained_error(error)))?;
    state
        .validate_private_state()
        .map_err(|error| Box::new(private_state_error(state.path(), error)))?;
    Ok(state)
}

pub(crate) fn private_state_error(path: &Path, error: PrivateStateError) -> ErrorDocument {
    let quoted = format!("'{}'", path.to_string_lossy().replace('\'', r"'\''"));
    let message = match error {
        PrivateStateError::Contained(error) => return contained_error(error),
        PrivateStateError::Owner => format!(
            "state directory {quoted} is owned by another user; it must be owned by the current user with mode 0700"
        ),
        PrivateStateError::Mode => {
            format!("state directory {quoted} must have mode 0700; run: chmod 700 {quoted}")
        }
    };
    ErrorDocument::new(ErrorKind::Refused, message).rule("unsafe_state")
}

/// Takes this machine's write lock, then refuses unless this machine's last write is in the ledger.
pub(crate) fn lock(state: &RootDir) -> Result<WriteLock, Box<ErrorDocument>> {
    let lock = lock_with_wait(state, Duration::from_secs(5))?;
    let markers = super::ledger_identity::markers(&super::Environment::from_process())?;
    super::ledger_identity::claim(state, &markers)?;
    Ok(lock)
}

pub(crate) fn lock_with_wait(
    state: &RootDir,
    wait: Duration,
) -> Result<WriteLock, Box<ErrorDocument>> {
    WriteLock::acquire(state, wait).map_err(|error| {
        Box::new(match error {
            LockError::Contained(error) => contained_error(error),
            LockError::Busy => {
                ErrorDocument::new(ErrorKind::Store, "another VPT writer is running")
            }
            LockError::Io(_) => {
                ErrorDocument::new(ErrorKind::Store, "cannot acquire the write lock")
            }
        })
    })
}

#[cfg(test)]
mod tests;
