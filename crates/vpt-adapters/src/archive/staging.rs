use super::*;
use std::os::unix::fs::PermissionsExt;

impl ClonefileArchive {
    pub(super) fn stage_with<C>(&self, clone: C) -> Result<Staged, StageFailure>
    where
        C: FnOnce(&Path, (u64, u64), &str) -> Result<CloneKind, CloneError>,
    {
        let root = self.root().map_err(|cause| StageFailure {
            cause,
            owned_staging: None,
        })?;
        let identity = root.identity().map_err(|error| StageFailure {
            cause: archive_error(root.path(), error),
            owned_staging: None,
        })?;
        let sequence = self.sequence.fetch_add(1, Ordering::Relaxed);
        let name = format!("{STAGING_PREFIX}{}-{sequence}.m4a", std::process::id());
        let result = match clone(root.path(), identity, &name) {
            Ok(kind) => self
                .finish(&name, kind)
                .map_err(|cause| self.owned_failure(&name, cause)),
            Err(CloneError::Exists) => Err(StageFailure {
                cause: ArchiveError::Io("staging name already exists".into()),
                owned_staging: None,
            }),
            Err(error) => Err(self.owned_failure(&name, clone_error(error))),
        };
        root.revalidate().map_err(|error| StageFailure {
            cause: archive_error(root.path(), error),
            owned_staging: None,
        })?;
        result
    }

    fn finish(&self, name: &str, kind: CloneKind) -> Result<Staged, ArchiveError> {
        let root = self.root()?;
        let path = root.path().join(name);
        let mut file = root
            .open_file(Path::new(name), Access::Read)
            .map_err(|error| archive_error(&path, error))?;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))
            .map_err(|error| io(&error))?;
        let digest = crate::digest_open(&mut file).map_err(|error| io(&error))?;
        let size = file.metadata().map_err(|error| io(&error))?.len();
        Ok(Staged {
            path,
            digest,
            size,
            copy_on_write: kind == CloneKind::CopyOnWrite,
        })
    }

    fn owned_failure(&self, name: &str, cause: ArchiveError) -> StageFailure {
        let owned = self.root.as_ref().and_then(|root| {
            root.revalidate().ok()?;
            let stat = root.stat(Path::new(name)).ok()?;
            (stat.kind == Kind::File).then_some(root.path().join(name))
        });
        if let Some(root) = &self.root
            && owned.is_some()
            && let Ok(file) = root.open_file(Path::new(name), Access::Read)
        {
            let _ = file.set_permissions(std::fs::Permissions::from_mode(0o600));
        }
        StageFailure {
            cause,
            owned_staging: owned,
        }
    }
}

fn clone_error(error: CloneError) -> ArchiveError {
    match error {
        CloneError::NoSpace => ArchiveError::NoSpace,
        CloneError::Exists => ArchiveError::Io("staging name already exists".into()),
        CloneError::Escape(path) => ArchiveError::Escape(path),
        CloneError::Io(detail) => ArchiveError::Io(detail),
    }
}
