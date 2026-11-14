use crate::contained::{Access, ContainedError, RootDir};
use crate::stores::digest_open;
use std::fs::Metadata;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use vpt_domain::digest::Sha256Digest;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Version {
    inode: u64,
    size: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}

impl Version {
    fn of(metadata: &Metadata) -> Self {
        Self {
            inode: metadata.ino(),
            size: metadata.size(),
            modified: (metadata.mtime(), metadata.mtime_nsec()),
            changed: (metadata.ctime(), metadata.ctime_nsec()),
        }
    }
}

pub(super) struct Displaced {
    pub digest: Sha256Digest,
    version: Version,
}

impl Displaced {
    pub(super) fn read(root: &RootDir, path: &Path) -> Result<Self, ContainedError> {
        let failed = |error: std::io::Error| ContainedError::Io {
            path: path.to_path_buf(),
            kind: error.kind(),
        };
        let mut file = root.open_file(path, Access::Read)?;
        let metadata = file.metadata().map_err(failed)?;
        Ok(Self {
            version: Version::of(&metadata),
            digest: digest_open(&mut file).map_err(failed)?,
        })
    }

    pub(super) fn unchanged(&self, root: &RootDir, path: &Path) -> bool {
        root.open_file(path, Access::Read)
            .ok()
            .and_then(|file| file.metadata().ok())
            .is_some_and(|metadata| Version::of(&metadata) == self.version)
    }
}
