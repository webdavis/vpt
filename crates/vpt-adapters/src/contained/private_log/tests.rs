use super::*;
use crate::contained::private_state::effective_user;
use std::fs;
use std::os::macos::fs::MetadataExt as _;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::path::{Path, PathBuf};

mod append;
mod failures;
mod inspect;
mod privacy;
mod proof;

fn fixture() -> (tempfile::TempDir, RootDir) {
    let temp = crate::test_temp::tempdir().expect("owned log fixture");
    let path = temp.path().canonicalize().expect("canonical fixture");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).expect("private state");
    let root = RootDir::open(&path).expect("retained root");
    (temp, root)
}

fn plain(path: &Path, bytes: &[u8], mode: u32) {
    fs::write(path, bytes).expect("owned file");
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).expect("owned mode");
}

#[derive(Debug, PartialEq, Eq)]
struct Entry {
    path: PathBuf,
    device: u64,
    inode: u64,
    mode: u32,
    owner: u32,
    group: u32,
    links: u64,
    size: u64,
    modified: (i64, i64),
    changed: (i64, i64),
    flags: u32,
    bytes: Vec<u8>,
    target: Option<PathBuf>,
}

fn snapshot(path: &Path) -> Vec<Entry> {
    let metadata = fs::symlink_metadata(path).expect("snapshot metadata");
    let mut result = vec![Entry {
        path: path.into(),
        device: metadata.dev(),
        inode: metadata.ino(),
        mode: metadata.mode(),
        owner: metadata.uid(),
        group: metadata.gid(),
        links: metadata.nlink(),
        size: metadata.len(),
        modified: (metadata.mtime(), metadata.mtime_nsec()),
        changed: (metadata.ctime(), metadata.ctime_nsec()),
        flags: metadata.st_flags(),
        bytes: if metadata.is_file() {
            fs::read(path).expect("snapshot bytes")
        } else {
            vec![]
        },
        target: metadata
            .is_symlink()
            .then(|| fs::read_link(path).expect("link target")),
    }];
    if metadata.is_dir() {
        let mut children = fs::read_dir(path)
            .expect("snapshot directory")
            .map(|entry| entry.expect("entry").path())
            .collect::<Vec<_>>();
        children.sort();
        for child in children {
            result.extend(snapshot(&child));
        }
    }
    result
}
