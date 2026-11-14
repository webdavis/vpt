use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainedError {
    Escape {
        root: PathBuf,
        path: PathBuf,
    },
    NotRegular(PathBuf),
    NotADirectory(PathBuf),
    Io {
        path: PathBuf,
        kind: std::io::ErrorKind,
    },
}

pub(super) fn escape(root: &Path, path: &Path) -> ContainedError {
    ContainedError::Escape {
        root: root.into(),
        path: path.into(),
    }
}

pub(super) fn io(path: &Path, error: std::io::Error) -> ContainedError {
    ContainedError::Io {
        path: path.into(),
        kind: error.kind(),
    }
}
