use super::*;
use std::os::fd::AsRawFd;

impl RootDir {
    fn rename(&self, from: &Path, to: &Path, flags: libc::c_uint) -> Result<(), ContainedError> {
        let from = self.c_name(from)?;
        let leaf = self.leaf(to)?;
        let to = self.c_name(&leaf)?;
        // SAFETY: both validated NUL-terminated names and the retained descriptor outlive the call.
        let result = unsafe {
            libc::renameatx_np(
                self.file.as_raw_fd(),
                from.as_ptr(),
                self.file.as_raw_fd(),
                to.as_ptr(),
                flags,
            )
        };
        if result == 0 {
            Ok(())
        } else {
            Err(io(&leaf, std::io::Error::last_os_error()))
        }
    }

    pub fn rename_over(&self, from: &Path, to: &Path) -> Result<(), ContainedError> {
        self.rename(from, to, 0)
    }

    pub fn rename_exclusive(&self, from: &Path, to: &Path) -> Result<bool, ContainedError> {
        match self.rename(from, to, libc::RENAME_EXCL) {
            Ok(()) => Ok(true),
            Err(ContainedError::Io {
                kind: std::io::ErrorKind::AlreadyExists,
                ..
            }) => Ok(false),
            Err(error) => Err(error),
        }
    }

    pub fn rename_swap(&self, first: &Path, second: &Path) -> Result<(), ContainedError> {
        self.rename(first, second, libc::RENAME_SWAP)
    }
}
