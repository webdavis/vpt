use crate::support::{Sandbox, stderr};
use std::os::macos::fs::MetadataExt as _;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Output;

pub(super) fn configured() -> (Sandbox, PathBuf) {
    let sandbox = Sandbox::new("symlink");
    let parent = sandbox.path().join("home/notes");
    std::fs::create_dir(&parent).expect("owned target parent");
    let target = parent.join("vpt");
    write_config(
        &sandbox.config_path(),
        &format!("config_version = 1\n[home]\nsymlink_target = {target:?}\n"),
    );
    (sandbox, target)
}

pub(super) fn write_config(path: &Path, text: &str) {
    let parent = path.parent().expect("config parent");
    std::fs::create_dir_all(parent).expect("config directory");
    std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))
        .expect("private directory");
    std::fs::write(path, text).expect("owned configuration");
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).expect("private file");
}

pub(super) fn error(output: &Output, code: i32, kind: &str) -> serde_json::Value {
    assert_eq!(output.status.code(), Some(code), "{}", stderr(output));
    assert!(output.stdout.is_empty());
    let document: serde_json::Value = serde_json::from_str(&stderr(output)).expect("error JSON");
    assert_eq!(document["schema"], "vpt.error/1");
    assert_eq!(document["error"]["kind"], kind);
    document
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Entry {
    path: PathBuf,
    mode: u32,
    identity: (u64, u64),
    owner: (u32, u32),
    links: u64,
    size: u64,
    flags: u32,
    modified: (i64, i64),
    changed: (i64, i64),
    bytes: Vec<u8>,
    link: Option<PathBuf>,
}

pub(super) fn snapshot(root: &Path) -> Vec<Entry> {
    fn visit(root: &Path, path: &Path, entries: &mut Vec<Entry>) {
        let metadata = std::fs::symlink_metadata(path).expect("owned entry metadata");
        entries.push(Entry {
            path: path.strip_prefix(root).expect("owned path").into(),
            mode: metadata.mode(),
            identity: (metadata.dev(), metadata.ino()),
            owner: (metadata.uid(), metadata.gid()),
            links: metadata.nlink(),
            size: metadata.size(),
            flags: metadata.st_flags(),
            modified: (metadata.mtime(), metadata.mtime_nsec()),
            changed: (metadata.ctime(), metadata.ctime_nsec()),
            bytes: if metadata.is_file() {
                std::fs::read(path).expect("owned file")
            } else {
                vec![]
            },
            link: metadata
                .file_type()
                .is_symlink()
                .then(|| std::fs::read_link(path).expect("link")),
        });
        if metadata.is_dir() {
            for entry in std::fs::read_dir(path).expect("owned directory") {
                visit(root, &entry.expect("owned entry").path(), entries);
            }
        }
    }
    let mut entries = vec![];
    visit(root, root, &mut entries);
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    entries
}
