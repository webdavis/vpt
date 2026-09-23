use super::*;
use std::os::fd::AsRawFd;
use std::os::unix::fs::PermissionsExt;

impl RootDir {
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "D3 wires database creation callbacks")
    )]
    pub(crate) fn create_read_write_file(
        &self,
        path: &Path,
        mode: u32,
    ) -> Result<File, ContainedError> {
        let leaf = self.leaf(path)?;
        let file = self.open_at(
            &leaf,
            libc::O_RDWR | libc::O_CREAT | libc::O_EXCL | NO_FOLLOW | libc::O_NONBLOCK,
            mode,
        )?;
        let metadata = file.metadata().map_err(|error| io(&leaf, error))?;
        if !metadata.is_file() {
            return Err(ContainedError::NotRegular(leaf));
        }
        if metadata.nlink() != 1 {
            return Err(escape(&self.path, &leaf));
        }
        file.set_permissions(std::fs::Permissions::from_mode(mode))
            .map_err(|error| io(&leaf, error))?;
        self.sync()?;
        Ok(file)
    }

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "D3 wires database deletion callbacks")
    )]
    pub(crate) fn unlink_file(&self, path: &Path) -> Result<(), ContainedError> {
        let leaf = self.leaf(path)?;
        let metadata = self.named_stat(&leaf)?;
        if metadata.st_mode & libc::S_IFMT != libc::S_IFREG {
            return Err(ContainedError::NotRegular(leaf));
        }
        if metadata.st_nlink != 1 {
            return Err(escape(&self.path, &leaf));
        }
        let name = self.c_name(&leaf)?;
        // SAFETY: the retained directory and validated NUL-terminated name outlive unlinkat.
        if unsafe { libc::unlinkat(self.file.as_raw_fd(), name.as_ptr(), 0) } != 0 {
            return Err(io(&leaf, std::io::Error::last_os_error()));
        }
        Ok(())
    }

    pub(super) fn open_at(
        &self,
        leaf: &Path,
        flags: libc::c_int,
        mode: u32,
    ) -> Result<File, ContainedError> {
        let name = self.c_name(leaf)?;
        // SAFETY: the retained descriptor and NUL-terminated name stay live; variadic mode is promoted.
        let fd = unsafe {
            libc::openat(
                self.file.as_raw_fd(),
                name.as_ptr(),
                flags,
                mode as libc::c_uint,
            )
        };
        opened(fd, leaf, flags & libc::O_DIRECTORY != 0)
    }

    fn named_stat(&self, path: &Path) -> Result<libc::stat, ContainedError> {
        let leaf = self.leaf(path)?;
        let name = self.c_name(&leaf)?;
        let mut raw = std::mem::MaybeUninit::<libc::stat>::uninit();
        // SAFETY: raw is writable stat storage; descriptor and NUL-terminated name stay live.
        let result = unsafe {
            libc::fstatat(
                self.file.as_raw_fd(),
                name.as_ptr(),
                raw.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        if result != 0 {
            return Err(io(&leaf, std::io::Error::last_os_error()));
        }
        // SAFETY: successful fstatat initialized raw completely.
        Ok(unsafe { raw.assume_init() })
    }

    pub fn stat(&self, path: &Path) -> Result<Stat, ContainedError> {
        let raw = self.named_stat(path)?;
        let kind = match raw.st_mode & libc::S_IFMT {
            libc::S_IFREG => Kind::File,
            libc::S_IFDIR => Kind::Directory,
            libc::S_IFLNK => Kind::Link,
            _ => Kind::Other,
        };
        Ok(Stat {
            kind,
            size: u64::try_from(raw.st_size).unwrap_or(0),
            mtime_secs: raw.st_mtime,
            mtime_nanos: u32::try_from(raw.st_mtime_nsec).unwrap_or(0),
            flags: raw.st_flags,
        })
    }

    pub fn regular(&self, path: &Path) -> Result<PathBuf, ContainedError> {
        let leaf = self.leaf(path)?;
        match self.stat(&leaf)?.kind {
            Kind::File => Ok(leaf),
            _ => Err(ContainedError::NotRegular(leaf)),
        }
    }

    pub fn open_file(&self, path: &Path, access: Access) -> Result<File, ContainedError> {
        let leaf = self.leaf(path)?;
        let access = match access {
            Access::Read => libc::O_RDONLY,
            Access::Write => libc::O_WRONLY,
            Access::ReadWrite => libc::O_RDWR,
        };
        let file = self.open_at(&leaf, access | NO_FOLLOW | libc::O_NONBLOCK, 0)?;
        if !file.metadata().map_err(|error| io(&leaf, error))?.is_file() {
            return Err(ContainedError::NotRegular(leaf));
        }
        Ok(file)
    }

    pub fn create_file(&self, path: &Path, mode: u32) -> Result<File, ContainedError> {
        let leaf = self.leaf(path)?;
        let file = self.open_at(
            &leaf,
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | NO_FOLLOW,
            mode,
        )?;
        file.set_permissions(std::fs::Permissions::from_mode(mode))
            .map_err(|error| io(&leaf, error))?;
        Ok(file)
    }
}
