use super::TermsError;
use crate::{Access, ContainedError, RootDir};
use std::{
    fs::File,
    os::unix::fs::MetadataExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(super) struct Temporary<'a> {
    root: &'a RootDir,
    pub(super) path: PathBuf,
    pub(super) file: File,
    pub(super) published: bool,
}

impl<'a> Temporary<'a> {
    pub(super) fn validate(&self, bytes: &[u8]) -> Result<(), TermsError> {
        let current = super::snapshot::read(self.root, &self.root.path().join(&self.path))?;
        if !self.owns_name()
            || current
                .as_ref()
                .is_none_or(|value| !value.single_link || value.bytes != bytes)
        {
            return Err(TermsError::Changed);
        }
        Ok(())
    }

    fn owns_name(&self) -> bool {
        let Ok(named) = self.root.open_file(&self.path, Access::Read) else {
            return false;
        };
        let (Ok(named), Ok(owned)) = (named.metadata(), self.file.metadata()) else {
            return false;
        };
        named.nlink() == 1 && (named.dev(), named.ino()) == (owned.dev(), owned.ino())
    }

    pub(super) fn create(root: &'a RootDir) -> Result<Self, TermsError> {
        for _ in 0..64 {
            let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = PathBuf::from(format!(".vpt-terms-{}-{sequence}", std::process::id()));
            match root.create_file(&path, 0o600) {
                Ok(file) => {
                    return Ok(Self {
                        root,
                        path,
                        file,
                        published: false,
                    });
                }
                Err(ContainedError::Io {
                    kind: std::io::ErrorKind::AlreadyExists,
                    ..
                }) => (),
                Err(_) => return Err(TermsError::Unavailable),
            }
        }
        Err(TermsError::Unavailable)
    }
}

impl Drop for Temporary<'_> {
    fn drop(&mut self) {
        if self.published {
            return;
        }
        if self.owns_name() && self.root.unlink_file(&self.path).is_ok() {
            let _ = self.root.sync();
        }
    }
}
