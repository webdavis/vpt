use crate::contained::{Access, ContainedError, Kind, RootDir};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use vpt_application::ports::{StoreError, Stores};
use vpt_domain::digest::Sha256Digest;

pub const BUFFER: usize = 64 * 1024;

pub struct FilesystemStores {
    roots: Vec<RootDir>,
    configured: Vec<PathBuf>,
    sequence: AtomicU64,
}

pub fn digest_open(file: &mut File) -> std::io::Result<Sha256Digest> {
    file.seek(SeekFrom::Start(0))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; BUFFER];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(Sha256Digest(hasher.finalize().into()))
}

fn store_error(path: &Path, error: ContainedError) -> StoreError {
    match error {
        ContainedError::Io { kind, .. } => StoreError::Io(format!("{kind} at {}", path.display())),
        ContainedError::Escape { .. }
        | ContainedError::NotRegular(_)
        | ContainedError::NotADirectory(_) => StoreError::Escape(path.to_path_buf()),
    }
}

fn io(path: &Path, error: &std::io::Error) -> StoreError {
    StoreError::Io(format!("{} at {}", error.kind(), path.display()))
}

impl FilesystemStores {
    pub fn open(roots: &[PathBuf]) -> Result<Self, ContainedError> {
        Ok(Self {
            roots: roots
                .iter()
                .map(|path| RootDir::open(path))
                .collect::<Result<Vec<_>, _>>()?,
            configured: roots.to_vec(),
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
            sequence: AtomicU64::new(0),
        })
    }

    fn owner(&self, path: &Path) -> Result<(&RootDir, PathBuf), StoreError> {
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

    fn create_temporary(&self, root: &RootDir, name: &str) -> Result<(File, PathBuf), StoreError> {
        loop {
            let sequence = self.sequence.fetch_add(1, Ordering::Relaxed);
            let temporary = PathBuf::from(format!(".{name}.vpt-{}-{sequence}", std::process::id()));
            match root.create_file(&temporary, 0o644) {
                Ok(file) => return Ok((file, temporary)),
                Err(ContainedError::Io {
                    kind: std::io::ErrorKind::AlreadyExists,
                    ..
                }) => continue,
                Err(error) => return Err(store_error(&root.path().join(temporary), error)),
            }
        }
    }
}

impl Stores for FilesystemStores {
    fn validate_path(&self, path: &Path) -> Result<(), StoreError> {
        let (root, leaf) = self.owner(path)?;
        match root.stat(&leaf) {
            Ok(stat) if stat.kind == Kind::File => Ok(()),
            Ok(_) => Err(StoreError::Escape(path.to_path_buf())),
            Err(ContainedError::Io {
                kind: std::io::ErrorKind::NotFound,
                ..
            }) => Ok(()),
            Err(error) => Err(store_error(path, error)),
        }
    }

    fn digest_of(&self, path: &Path) -> Result<Option<Sha256Digest>, StoreError> {
        let (root, leaf) = self.owner(path)?;
        match root.open_file(&leaf, Access::Read) {
            Ok(mut file) => digest_open(&mut file)
                .map(Some)
                .map_err(|error| io(path, &error)),
            Err(ContainedError::Io {
                kind: std::io::ErrorKind::NotFound,
                ..
            }) => Ok(None),
            Err(error) => Err(store_error(path, error)),
        }
    }

    fn digest_bytes(&self, bytes: &[u8]) -> Sha256Digest {
        Sha256Digest(Sha256::digest(bytes).into())
    }

    fn publish(&self, target: &Path, bytes: &[u8]) -> Result<(), StoreError> {
        let (root, leaf) = self.owner(target)?;
        match root.stat(&leaf) {
            Ok(stat) if stat.kind == Kind::Link => {
                return Err(StoreError::Escape(target.to_path_buf()));
            }
            Ok(_) => {}
            Err(ContainedError::Io {
                kind: std::io::ErrorKind::NotFound,
                ..
            }) => {}
            Err(error) => return Err(store_error(target, error)),
        }
        let name = leaf
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| StoreError::Escape(target.to_path_buf()))?;
        let (mut file, temporary) = self.create_temporary(root, name)?;
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|error| io(target, &error))?;
        drop(file);
        root.rename_over(&temporary, &leaf)
            .map_err(|error| store_error(target, error))?;
        root.sync().map_err(|error| store_error(target, error))
    }

    fn sync_directory_of(&self, path: &Path) -> Result<(), StoreError> {
        let (root, _leaf) = self.owner(path)?;
        root.sync().map_err(|error| store_error(path, error))
    }
}

#[cfg(test)]
mod tests;
