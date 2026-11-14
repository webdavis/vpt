mod deploy;
mod inspect;
mod verify;

use super::{LinkState, SymlinkError};
use crate::database::snapshot;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

struct Fixture {
    _root: tempfile::TempDir,
    link: PathBuf,
    target: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = crate::test_temp::tempdir().expect("owned fixture");
        let canonical = root.path().canonicalize().expect("canonical fixture");
        std::fs::create_dir(canonical.join("notes")).expect("target parent");
        Self {
            _root: root,
            link: canonical.join(".vpt"),
            target: canonical.join("notes/vpt"),
        }
    }

    fn link_to(&self, target: impl AsRef<Path>) {
        std::os::unix::fs::symlink(target, &self.link).expect("fixture link");
    }

    fn verify(&self, link: &Path, target: &Path) -> Result<(), SymlinkError> {
        let before = snapshot(self._root.path());
        let result = super::verify(link, target);
        assert_eq!(snapshot(self._root.path()), before);
        result
    }
}

fn mode(path: &Path) -> u32 {
    std::fs::metadata(path)
        .expect("metadata")
        .permissions()
        .mode()
        & 0o777
}
