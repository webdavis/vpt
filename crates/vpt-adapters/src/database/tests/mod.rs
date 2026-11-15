mod failures;
mod files;
mod locks;
mod owner;
mod scenarios;
mod shared_memory;
mod wal;

use crate::test_support::children;
pub(crate) use scenarios::run_scenario;

pub(super) mod support {
    use super::super::FileRole;
    pub(crate) use crate::test_support::{root, snapshot};

    pub(crate) const ROLES: [(FileRole, &str); 4] = [
        (FileRole::Database, "vpt.db"),
        (FileRole::Journal, "vpt.db-journal"),
        (FileRole::Wal, "vpt.db-wal"),
        (FileRole::SharedMemory, "vpt.db-shm"),
    ];
}
