mod boundaries;
mod durability;
mod placement;

use super::*;
use crate::archive::ClonefileArchive;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use vpt_application::ports::Archive;

fn audio() -> (tempfile::TempDir, PathBuf, RootDir) {
    let temp = tempfile::tempdir().expect("temp");
    let path = temp.path().canonicalize().expect("canonical").join("audio");
    std::fs::create_dir(&path).expect("audio");
    let root = RootDir::open(&path).expect("root");
    (temp, path, root)
}

fn write(root: &RootDir, name: &str, bytes: &[u8]) -> PathBuf {
    let mut file = root.create_file(Path::new(name), 0o600).expect("create");
    file.write_all(bytes).expect("write");
    root.path().join(name)
}

fn mode(path: &Path) -> u32 {
    std::fs::metadata(path)
        .expect("metadata")
        .permissions()
        .mode()
        & 0o777
}
