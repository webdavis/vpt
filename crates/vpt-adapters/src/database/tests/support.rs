use super::super::FileRole;
use crate::RootDir;
use std::fs;
use std::os::macos::fs::MetadataExt as MacMetadataExt;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{DirBuilderExt, MetadataExt};
use std::path::{Path, PathBuf};

pub(super) const ROLES: [(FileRole, &str); 4] = [
    (FileRole::Database, "vpt.db"),
    (FileRole::Journal, "vpt.db-journal"),
    (FileRole::Wal, "vpt.db-wal"),
    (FileRole::SharedMemory, "vpt.db-shm"),
];

pub(super) fn root() -> (tempfile::TempDir, RootDir) {
    let temp = tempfile::tempdir().expect("owned temp");
    let path = temp
        .path()
        .canonicalize()
        .expect("canonical temp")
        .join("state");
    fs::DirBuilder::new()
        .mode(0o700)
        .create(&path)
        .expect("state");
    let root = RootDir::open(&path).expect("checked root");
    (temp, root)
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Snapshot {
    path: PathBuf,
    metadata: [u64; 12],
    bytes: Vec<u8>,
}

pub(super) fn snapshot(path: &Path) -> Vec<Snapshot> {
    let meta = fs::symlink_metadata(path).expect("metadata");
    let bytes = if meta.is_file() {
        fs::read(path).expect("bytes")
    } else if meta.is_symlink() {
        fs::read_link(path)
            .expect("link")
            .as_os_str()
            .as_bytes()
            .to_vec()
    } else {
        Vec::new()
    };
    let mut result = vec![Snapshot {
        path: path.into(),
        metadata: [
            u64::from(meta.mode()),
            u64::from(meta.uid()),
            u64::from(meta.gid()),
            meta.dev(),
            meta.ino(),
            meta.nlink(),
            meta.size(),
            meta.mtime() as u64,
            meta.mtime_nsec() as u64,
            meta.ctime() as u64,
            meta.ctime_nsec() as u64,
            u64::from(meta.st_flags()),
        ],
        bytes,
    }];
    if meta.is_dir() {
        let mut paths: Vec<_> = fs::read_dir(path)
            .expect("entries")
            .map(|entry| entry.expect("entry").path())
            .collect();
        paths.sort();
        for child in paths {
            result.extend(snapshot(&child));
        }
    }
    result
}
