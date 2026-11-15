use super::*;
use std::io::ErrorKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SingleFile {
    pub(crate) identity: (u64, u64),
    pub(crate) size: u64,
    pub(crate) mode: u32,
}

impl RootDir {
    /// Looks at a regular file with one link by name without opening it, so no descriptor close
    /// can release record locks this process holds on it. An absent file is `None`.
    pub(crate) fn single_file(&self, path: &Path) -> Result<Option<SingleFile>, ContainedError> {
        let leaf = self.leaf(path)?;
        let stat = match self.named_stat(&leaf) {
            Ok(stat) => stat,
            Err(ContainedError::Io {
                kind: ErrorKind::NotFound,
                ..
            }) => return Ok(None),
            Err(error) => return Err(error),
        };
        if stat.st_mode & libc::S_IFMT != libc::S_IFREG {
            return Err(ContainedError::NotRegular(leaf));
        }
        if stat.st_nlink != 1 {
            return Err(escape(&self.path, &leaf));
        }
        Ok(Some(SingleFile {
            identity: (stat.st_dev as u64, stat.st_ino),
            size: u64::try_from(stat.st_size).unwrap_or(0),
            mode: u32::from(stat.st_mode) & 0o7777,
        }))
    }
}
