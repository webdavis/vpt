use crate::ContainedError;
use std::path::Path;
use vpt_application::ports::RecorderError;

pub(super) fn recorder_error(path: &Path, error: ContainedError) -> RecorderError {
    match error {
        ContainedError::Escape { .. } | ContainedError::NotADirectory(_) => {
            RecorderError::Escape(path.into())
        }
        ContainedError::NotRegular(leaf) => RecorderError::NotRegular(leaf),
        ContainedError::Io {
            kind: std::io::ErrorKind::NotFound,
            ..
        } => RecorderError::NotFound(path.into()),
        ContainedError::Io { kind, .. } => RecorderError::Io(kind.to_string()),
    }
}

pub(super) fn listing_error(path: &Path, error: ContainedError) -> RecorderError {
    match error {
        ContainedError::Io { kind, .. } => RecorderError::Unreadable(kind.to_string()),
        ContainedError::Escape { .. }
        | ContainedError::NotRegular(_)
        | ContainedError::NotADirectory(_) => RecorderError::Escape(path.into()),
    }
}
