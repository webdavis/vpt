use crate::{ContainedError, FilesystemStores, Kind, RootDir, note_document::metadata};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use vpt_application::ports::StoreError;
mod lookup;
mod mentions;
mod read;

#[derive(Clone)]
pub struct IndexedNote {
    pub path: PathBuf,
    pub name: String,
    pub metadata: metadata::Metadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticKind {
    Metadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub path: PathBuf,
    pub kind: DiagnosticKind,
}

pub struct Index {
    notes: Vec<IndexedNote>,
    diagnostics: Vec<Diagnostic>,
    names: HashMap<String, Vec<PathBuf>>,
}

impl Index {
    pub fn read(roots: &[PathBuf]) -> Result<Self, StoreError> {
        Self::read_with(roots, Limits::default())
    }

    fn read_with(roots: &[PathBuf], mut limits: Limits) -> Result<Self, StoreError> {
        if roots.len() > 256 {
            return Err(capacity());
        }
        for root in roots {
            limits.metadata_bytes = limits
                .metadata_bytes
                .checked_sub(root.as_os_str().len() + std::mem::size_of::<PathBuf>())
                .ok_or_else(capacity)?;
        }
        let mut roots = roots.to_vec();
        roots.sort();
        roots.dedup();
        let mut covered: Vec<PathBuf> = Vec::new();
        let mut index = Self {
            notes: vec![],
            diagnostics: vec![],
            names: HashMap::new(),
        };
        for path in roots {
            if covered.iter().any(|root| path.starts_with(root)) {
                continue;
            }
            match RootDir::open(&path) {
                Ok(root) => read::walk(&root, &mut index, &mut limits, 0)?,
                Err(ContainedError::Io {
                    kind: std::io::ErrorKind::NotFound,
                    ..
                }) => (),
                Err(error) => return Err(contained(error)),
            }
            covered.push(path);
        }
        index.notes.sort_by(|a, b| a.path.cmp(&b.path));
        index.diagnostics.sort_by(|a, b| a.path.cmp(&b.path));
        index.build_lookup(&mut limits.metadata_bytes)?;
        Ok(index)
    }

    pub fn notes(&self) -> &[IndexedNote] {
        &self.notes
    }
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

impl std::fmt::Debug for Index {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Index")
            .field("notes", &self.notes.len())
            .field("diagnostics", &self.diagnostics)
            .finish()
    }
}

struct Limits {
    entries: usize,
    read_bytes: usize,
    metadata_bytes: usize,
    depth: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            entries: 65_536,
            read_bytes: 256 * 1024 * 1024,
            metadata_bytes: 16 * 1024 * 1024,
            depth: 64,
        }
    }
}

fn capacity() -> StoreError {
    StoreError::Io("note index exceeds capacity".into())
}
fn contained(error: ContainedError) -> StoreError {
    match error {
        ContainedError::Io { path, kind } => {
            StoreError::Io(format!("{kind} at {}", path.display()))
        }
        ContainedError::Escape { path, .. }
        | ContainedError::NotADirectory(path)
        | ContainedError::NotRegular(path) => StoreError::Escape(path),
    }
}

#[cfg(test)]
mod tests;
