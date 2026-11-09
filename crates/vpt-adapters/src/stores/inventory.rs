use super::{FilesystemStores, store_error};
use crate::contained::Kind;
use std::path::Path;
use vpt_application::ports::{RetentionEntries, StoreEntry, StoreError};
use vpt_domain::time::FileTime;

impl FilesystemStores {
    pub(super) fn scan(&self, path: &Path) -> Result<RetentionEntries, StoreError> {
        self.scan_bounded(path, usize::MAX)
    }

    pub(super) fn scan_bounded(
        &self,
        path: &Path,
        maximum: usize,
    ) -> Result<RetentionEntries, StoreError> {
        if !self.configured.iter().any(|configured| configured == path) {
            return Err(StoreError::Escape(path.to_path_buf()));
        }
        let Some(root) = self.roots.iter().find(|root| root.path() == path) else {
            return Ok(RetentionEntries::default());
        };
        let mut result = RetentionEntries::default();
        for name in root
            .names_bounded(maximum)
            .map_err(|error| store_error(path, error))?
        {
            let stat = root
                .stat(Path::new(&name))
                .map_err(|error| store_error(path, error))?;
            if stat.kind == Kind::Link {
                result.redirected.push(path.join(name));
                continue;
            }
            if stat.kind != Kind::File {
                continue;
            }
            result.files.push(StoreEntry {
                path: root
                    .leaf(Path::new(&name))
                    .map_err(|error| store_error(path, error))?,
                size: stat.size,
                mtime: FileTime {
                    secs: stat.mtime_secs,
                    nanos: stat.mtime_nanos,
                },
            });
        }
        root.revalidate()
            .map_err(|error| store_error(path, error))?;
        Ok(result)
    }
}
