mod files;
use crate::{Access, ContainedError, RootDir};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use vpt_application::ports::EngineFailure;

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(super) struct Workspace {
    parent: RootDir,
    root: RootDir,
    closed: bool,
}

impl Workspace {
    pub fn new(parent: &Path, source: &Path) -> Result<Self, EngineFailure> {
        Self::new_limited(parent, source, u64::MAX)
    }

    pub fn new_limited(parent: &Path, source: &Path, maximum: u64) -> Result<Self, EngineFailure> {
        let source_root = RootDir::open(source.parent().ok_or(EngineFailure::InvalidArguments)?)
            .map_err(|_| EngineFailure::Io)?;
        let input = source_root
            .open_file(source, Access::Read)
            .map_err(|_| EngineFailure::Io)?;
        let parent = RootDir::open(parent).map_err(|_| EngineFailure::Io)?;
        let root = allocate(&parent)?;
        let workspace = Self {
            parent,
            root,
            closed: false,
        };
        if let Err(error) = files::copy(input, &source_root, source, &workspace.root, maximum) {
            workspace.close()?;
            return Err(error);
        }
        Ok(workspace)
    }

    pub fn audio(&self) -> PathBuf {
        self.root.path().join("audio.m4a")
    }
    pub fn path(&self) -> &Path {
        self.root.path()
    }
    pub fn revalidate(&self) -> Result<(), EngineFailure> {
        self.root.revalidate().map_err(|_| EngineFailure::Io)
    }

    pub fn read_document(&self) -> Result<Vec<u8>, EngineFailure> {
        self.revalidate()?;
        let output = self
            .root
            .open_subdirectory("audio")
            .map_err(|_| EngineFailure::Io)?;
        let bytes = files::read_document(&output)?;
        output.revalidate().map_err(|_| EngineFailure::Io)?;
        self.revalidate()?;
        Ok(bytes)
    }

    pub fn close(mut self) -> Result<(), EngineFailure> {
        self.closed = true;
        self.parent
            .remove_owned_directory(&self.root)
            .map_err(|_| EngineFailure::WorkspaceCleanup(self.root.path().into()))
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        if !self.closed {
            let _ = self.parent.remove_owned_directory(&self.root);
        }
    }
}

fn allocate(parent: &RootDir) -> Result<RootDir, EngineFailure> {
    for _ in 0..256 {
        let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let name = format!(".vpt-engine-{}-{sequence}", std::process::id());
        match parent.create_owned_directory(&name) {
            Ok(root) => return Ok(root),
            Err(ContainedError::Io {
                kind: std::io::ErrorKind::AlreadyExists,
                ..
            }) => continue,
            Err(_) => return Err(EngineFailure::Io),
        }
    }
    Err(EngineFailure::Io)
}
