use super::*;

impl FilesystemStores {
    pub fn with_private_roots(mut self, roots: &[PathBuf]) -> Self {
        self.private = roots.to_vec();
        self
    }

    pub fn open(roots: &[PathBuf]) -> Result<Self, ContainedError> {
        Ok(Self {
            roots: roots
                .iter()
                .map(|path| RootDir::open(path))
                .collect::<Result<Vec<_>, _>>()?,
            configured: roots.to_vec(),
            private: Vec::new(),
            sequence: AtomicU64::new(0),
        })
    }

    pub fn open_read_only(paths: &[PathBuf]) -> Result<Self, ContainedError> {
        let mut roots = Vec::new();
        for path in paths {
            match RootDir::open(path) {
                Ok(root) => roots.push(root),
                Err(ContainedError::Io {
                    kind: std::io::ErrorKind::NotFound,
                    ..
                }) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(Self {
            roots,
            configured: paths.to_vec(),
            private: Vec::new(),
            sequence: AtomicU64::new(0),
        })
    }

    pub(super) fn owner(&self, path: &Path) -> Result<(&RootDir, PathBuf), StoreError> {
        if !self
            .configured
            .iter()
            .any(|configured| path.parent() == Some(configured.as_path()))
        {
            return Err(StoreError::Escape(path.to_path_buf()));
        }
        let root = self
            .roots
            .iter()
            .find(|root| path.parent() == Some(root.path()))
            .ok_or_else(|| StoreError::Escape(path.to_path_buf()))?;
        let leaf = root.leaf(path).map_err(|error| store_error(path, error))?;
        root.revalidate()
            .map_err(|error| store_error(path, error))?;
        Ok((root, leaf))
    }
}
