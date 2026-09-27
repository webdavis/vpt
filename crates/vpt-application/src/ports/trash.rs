use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrashError {
    HelperAbsent,
    HelperVersion { found: u32 },
    Failed(String),
    Unknown(String),
}

pub trait Trash {
    fn trash(&self, path: &Path) -> Result<PathBuf, TrashError>;
    fn take_diagnostics(&self) -> Vec<String>;
}
