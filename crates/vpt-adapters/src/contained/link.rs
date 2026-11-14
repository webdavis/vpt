use super::{ContainedError, RootDir, io};
use std::os::fd::AsRawFd;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    Placed,
    Occupied,
}

impl RootDir {
    pub(crate) fn link_exclusive(
        &self,
        from: &Path,
        to: &Path,
    ) -> Result<Placement, ContainedError> {
        let leaf = self.leaf(to)?;
        let (from, to) = (self.c_name(from)?, self.c_name(&leaf)?);
        let fd = self.file.as_raw_fd();
        // SAFETY: both validated NUL-terminated names and the retained descriptor outlive the call;
        // flag 0 links a leaf symlink itself and never follows it.
        if unsafe { libc::linkat(fd, from.as_ptr(), fd, to.as_ptr(), 0) } == 0 {
            return Ok(Placement::Placed);
        }
        match std::io::Error::last_os_error() {
            error if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(Placement::Occupied),
            error => Err(io(&leaf, error)),
        }
    }
}
