use crate::ContainedError;
use std::path::Path;

pub(crate) mod document;
mod parent;
mod snapshot;
mod update;
pub(crate) mod validation;
pub(crate) use update::prepare_with;
pub use update::{Update, prepare};

pub(crate) const MAX_BYTES: usize = 16 * 1024 * 1024;
pub(crate) const MAX_TERMS: usize = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermsError {
    UnsafePath,
    Unavailable,
    TooLarge,
    Changed,
    InvalidText,
}

pub fn read(path: &Path) -> Result<Vec<String>, TermsError> {
    let parent = parent::Parent::capture(path)?;
    let Some(root) = parent.existing() else {
        return Ok(vec![]);
    };
    let snapshot = snapshot::read(root, path)?;
    document::lines(
        snapshot
            .as_ref()
            .map_or(&[], |value| value.bytes.as_slice()),
    )
    .map(|lines| lines.into_iter().map(str::to_owned).collect())
}

fn missing(error: &ContainedError) -> bool {
    matches!(
        error,
        ContainedError::Io {
            kind: std::io::ErrorKind::NotFound,
            ..
        }
    )
}

#[cfg(test)]
mod tests;
