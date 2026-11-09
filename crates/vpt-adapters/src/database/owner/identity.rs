use super::{AccessPolicy, Database, DatabaseError, Registration, RootDir, invalid};
use std::path::Path;
use std::time::Duration;

impl Database {
    pub(crate) fn open_pinned(
        root: RootDir,
        leaf: &Path,
        policy: AccessPolicy,
        busy: Duration,
        owner: &Self,
    ) -> Result<Self, DatabaseError> {
        let retained = owner
            .registration
            .as_ref()
            .ok_or_else(|| invalid("closed registration"))?;
        let registration = Registration::new(root, leaf, policy)?;
        registration.state.files.pin_to(&retained.state.files)?;
        Self::open_registered(registration, busy)
    }
}
