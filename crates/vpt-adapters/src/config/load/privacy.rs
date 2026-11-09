use super::{
    ConfigError,
    file::{config_contained, config_io},
};
use crate::{PrivateStateError, RootDir, contained::effective_user};
use std::{fs::File, os::unix::fs::MetadataExt, path::Path};

pub(super) fn validate(root: &RootDir, file: &File, path: &Path) -> Result<(), ConfigError> {
    root.validate_private_state().map_err(|error| match error {
        PrivateStateError::Contained(error) => config_contained(path, error),
        PrivateStateError::Owner | PrivateStateError::Mode => ConfigError::Unreadable {
            path: path.display().to_string(),
            detail: "configuration directory must be private (0700) and owned by the current user"
                .into(),
        },
    })?;
    let metadata = file.metadata().map_err(|error| config_io(path, error))?;
    if metadata.uid() != effective_user() || metadata.mode() & 0o7777 != 0o600 {
        return Err(ConfigError::Unreadable {
            path: path.display().to_string(),
            detail: "configuration file must be private (0600) and owned by the current user"
                .into(),
        });
    }
    Ok(())
}
