use super::*;
use std::os::fd::AsRawFd;

impl RootDir {
    pub(crate) fn clone_file(&self, source: &File, leaf: &Path) -> std::io::Result<()> {
        let name = self
            .c_name(leaf)
            .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
        // SAFETY: both owners retain their descriptors and c_name yields one NUL-terminated leaf.
        let result = unsafe {
            libc::fclonefileat(source.as_raw_fd(), self.file.as_raw_fd(), name.as_ptr(), 0)
        };
        if result == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }
}
