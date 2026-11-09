use super::{TermsError, missing};
use crate::RootDir;
use std::path::{Component, Path};

pub(super) struct Parent {
    anchor: RootDir,
    missing: Vec<String>,
}

impl Parent {
    pub(super) fn capture(path: &Path) -> Result<Self, TermsError> {
        if !path.is_absolute()
            || path
                .components()
                .any(|part| !matches!(part, Component::RootDir | Component::Normal(_)))
            || path.file_name().is_none()
        {
            return Err(TermsError::UnsafePath);
        }
        let mut cursor = path.parent().ok_or(TermsError::UnsafePath)?;
        let mut names = Vec::new();
        loop {
            match RootDir::open(cursor) {
                Ok(anchor) => {
                    names.reverse();
                    return Ok(Self {
                        anchor,
                        missing: names,
                    });
                }
                Err(error) if missing(&error) => {
                    names.push(
                        cursor
                            .file_name()
                            .and_then(|name| name.to_str())
                            .ok_or(TermsError::UnsafePath)?
                            .to_owned(),
                    );
                    cursor = cursor.parent().ok_or(TermsError::UnsafePath)?;
                }
                Err(_) => return Err(TermsError::UnsafePath),
            }
        }
    }

    pub(super) fn existing(&self) -> Option<&RootDir> {
        self.missing.is_empty().then_some(&self.anchor)
    }

    pub(super) fn create(&self) -> Result<RootDir, TermsError> {
        self.anchor.revalidate().map_err(|_| TermsError::Changed)?;
        let mut root = self
            .anchor
            .try_clone()
            .map_err(|_| TermsError::Unavailable)?;
        for name in &self.missing {
            root.revalidate().map_err(|_| TermsError::Changed)?;
            let child = root
                .subdirectory(name)
                .map_err(|_| TermsError::UnsafePath)?;
            root.sync().map_err(|_| TermsError::Unavailable)?;
            root = child;
        }
        root.revalidate().map_err(|_| TermsError::Changed)?;
        Ok(root)
    }
}
