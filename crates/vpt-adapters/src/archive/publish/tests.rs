mod after_placement;
mod boundaries;
mod durability;
mod placement;
mod substitution;

use super::*;
use crate::archive::ClonefileArchive;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use vpt_application::ports::Archive;

fn audio() -> (tempfile::TempDir, PathBuf, RootDir) {
    let temp = crate::test_temp::tempdir().expect("temp");
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

fn digest(mut bytes: &[u8]) -> vpt_domain::digest::Sha256Digest {
    crate::stores::digest_reader(&mut bytes).unwrap()
}

fn prepared(root: &RootDir, path: &Path) -> Result<(Staged, ArchiveReader), ArchiveError> {
    let handle = ArchiveReader::open(root, path)?;
    let staged = Staged {
        path: path.into(),
        digest: handle.digest(root)?,
        size: handle.initial.len(),
        copy_on_write: false,
    };
    Ok((staged, handle))
}

fn exclusive(root: &RootDir, path: &Path, target: &str) -> Result<Published, ArchiveError> {
    let (staged, handle) = prepared(root, path)?;
    super::exclusive(root, &staged, &handle, target)
}

fn exclusive_with<F, D>(
    root: &RootDir,
    path: &Path,
    target: &str,
    mut sync_file: F,
    sync_directory: D,
) -> Result<Published, ArchiveError>
where
    F: FnMut(&RootDir, &File) -> Result<(), ArchiveError>,
    D: FnOnce(&RootDir) -> Result<(), ArchiveError>,
{
    let (staged, handle) = prepared(root, path)?;
    super::exclusive_with(
        root,
        &staged,
        &handle,
        target,
        |file| sync_file(root, file),
        sync_directory,
        crate::archive::placement::place,
    )
}

fn exclusive_placing<P>(
    root: &RootDir,
    path: &Path,
    target: &str,
    place: P,
) -> Result<Published, ArchiveError>
where
    P: FnOnce(&RootDir, &File, &Path, &Path) -> Result<Placement, ContainedError>,
{
    let (staged, handle) = prepared(root, path)?;
    super::exclusive_with(
        root,
        &staged,
        &handle,
        target,
        sync_open,
        sync_directory,
        place,
    )
}
