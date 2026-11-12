pub(super) mod children;
mod failures;
mod files;
mod locks;
mod owner;
mod shared_memory;
pub(super) mod support;
mod wal;

pub(crate) use children::{ChildGuard, child_for_root, turn};

pub(crate) fn test_root() -> (tempfile::TempDir, crate::RootDir) {
    support::root()
}
