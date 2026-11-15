use super::checks::check;
use crate::compose::{errors, mutation::private_state_error};
use std::path::Path;
use vpt_adapters::config::planned_directory;
use vpt_adapters::{ContainedError, RootDir};
use vpt_protocol::error::Check;

pub(super) fn inspect(state: &Path) -> Check {
    let checked = planned_directory(state, "home.state_dir")
        .map_err(|error| errors::root_error(&error).message)
        .and_then(|path| match RootDir::open(&path) {
            Ok(root) => root
                .validate_private_state()
                .map(|()| path.display().to_string())
                .map_err(|error| private_state_error(&path, error).message),
            Err(ContainedError::Io {
                kind: std::io::ErrorKind::NotFound,
                ..
            }) => Ok(format!("{} does not exist yet", path.display())),
            Err(error) => Err(errors::contained_error(error).message),
        });
    match checked {
        Ok(detail) => check("state_dir", true, detail),
        Err(detail) => check("state_dir", false, detail),
    }
}
