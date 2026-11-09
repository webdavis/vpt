use super::errors::{contained_error, root_error};
use std::time::Duration;
use vpt_adapters::{LockError, PrivateStateError, RootDir, Roots, WriteLock};
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(crate) fn state(roots: &Roots) -> Result<RootDir, Box<ErrorDocument>> {
    roots
        .create_state_dir()
        .map_err(|error| Box::new(root_error(&error)))?;
    let state =
        RootDir::open(&roots.state_dir).map_err(|error| Box::new(contained_error(error)))?;
    state.validate_private_state().map_err(|error| {
        Box::new(match error {
            PrivateStateError::Contained(error) => contained_error(error),
            PrivateStateError::Owner | PrivateStateError::Mode => ErrorDocument::new(
                ErrorKind::Refused,
                "state directory must be owned by the current user with mode 0700",
            )
            .rule("unsafe_state"),
        })
    })?;
    Ok(state)
}

pub(crate) fn lock(state: &RootDir) -> Result<WriteLock, Box<ErrorDocument>> {
    lock_with_wait(state, Duration::from_secs(5))
}

fn lock_with_wait(state: &RootDir, wait: Duration) -> Result<WriteLock, Box<ErrorDocument>> {
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
