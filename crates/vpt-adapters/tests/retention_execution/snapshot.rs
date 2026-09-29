use std::fs;
use std::os::darwin::fs::MetadataExt as DarwinMetadataExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq)]
pub struct Entry {
    path: PathBuf,
    metadata: [u64; 12],
    content: Vec<u8>,
}

pub fn tree(path: &Path) -> Vec<Entry> {
    let stat = fs::symlink_metadata(path).unwrap();
    let mut content = vec![];
    if stat.is_file() {
        content = fs::read(path).unwrap();
    }
    if stat.file_type().is_symlink() {
        use std::os::unix::ffi::OsStrExt;
        content = fs::read_link(path).unwrap().as_os_str().as_bytes().to_vec();
    }
    let mut result = vec![Entry {
        path: path.into(),
        metadata: [
            stat.dev(),
            stat.ino(),
            stat.mode().into(),
            stat.uid().into(),
            stat.gid().into(),
            stat.nlink(),
            stat.size(),
            stat.mtime() as u64,
            stat.mtime_nsec() as u64,
            stat.ctime() as u64,
            stat.ctime_nsec() as u64,
            stat.st_flags().into(),
        ],
        content,
    }];
    if stat.is_dir() {
        let mut children: Vec<_> = fs::read_dir(path)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        children.sort();
        for child in children {
            result.extend(tree(&child));
        }
    }
    result
}
