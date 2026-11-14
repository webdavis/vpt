use super::*;
use std::ffi::CStr;
use std::os::fd::{AsRawFd, IntoRawFd};
use std::os::unix::fs::PermissionsExt;
use std::ptr::NonNull;

impl RootDir {
    pub fn open_subdirectory(&self, name: &str) -> Result<Self, ContainedError> {
        let path = self.leaf(Path::new(name))?;
        let file = self.open_at(&path, libc::O_RDONLY | libc::O_DIRECTORY | NO_FOLLOW, 0)?;
        Ok(Self { file, path })
    }

    pub fn subdirectory(&self, name: &str) -> Result<Self, ContainedError> {
        let path = self.leaf(Path::new(name))?;
        let name_c = self.c_name(&path)?;
        // SAFETY: the retained descriptor and validated NUL-terminated name outlive mkdirat.
        let made = unsafe { libc::mkdirat(self.file.as_raw_fd(), name_c.as_ptr(), 0o700) };
        if made != 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(io(&path, error));
            }
        }
        let directory = self.open_subdirectory(name)?;
        if made == 0 {
            directory
                .file
                .set_permissions(std::fs::Permissions::from_mode(0o700))
                .map_err(|error| io(&path, error))?;
        }
        Ok(directory)
    }

    pub fn names(&self) -> Result<Vec<String>, ContainedError> {
        self.revalidate()?;
        self.retained_names()
    }

    pub fn names_strict(&self) -> Result<Vec<String>, ContainedError> {
        self.revalidate()?;
        self.scan_names(true, usize::MAX)
    }

    pub(crate) fn names_bounded(&self, maximum: usize) -> Result<Vec<String>, ContainedError> {
        self.revalidate()?;
        self.scan_names(true, maximum)
    }

    pub(super) fn retained_names(&self) -> Result<Vec<String>, ContainedError> {
        self.scan_names(false, usize::MAX)
    }

    fn scan_names(&self, strict: bool, maximum: usize) -> Result<Vec<String>, ContainedError> {
        let flags = libc::O_RDONLY | libc::O_DIRECTORY | NO_FOLLOW;
        // Opening dot gives the stream its own offset instead of sharing the retained descriptor's.
        let file = open_relative(self.as_fd(), c".", flags, 0, &self.path)?;
        // SAFETY: file owns a fresh directory descriptor; ownership transfers only on success.
        let stream = unsafe { libc::fdopendir(file.as_raw_fd()) };
        let stream =
            NonNull::new(stream).ok_or_else(|| io(&self.path, std::io::Error::last_os_error()))?;
        let _ = file.into_raw_fd();
        let stream = DirectoryStream(stream);
        let mut names = Vec::new();
        loop {
            // SAFETY: __error yields this thread's writable errno; stream owns a live DIR.
            let entry = unsafe {
                *libc::__error() = 0;
                libc::readdir(stream.0.as_ptr())
            };
            if entry.is_null() {
                let error = std::io::Error::last_os_error();
                if error.raw_os_error() != Some(0) {
                    return Err(io(&self.path, error));
                }
                break;
            }
            // SAFETY: readdir's NUL-terminated name stays live until the next read or close.
            let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) };
            let decoded = if strict {
                checked_entry_name(name, &self.path)?
            } else {
                entry_name(name)
            };
            if let Some(name) = decoded {
                if names.len() >= maximum {
                    return Err(io(&self.path, std::io::ErrorKind::FileTooLarge.into()));
                }
                names.push(name.to_owned());
            }
        }
        stream.close().map_err(|error| io(&self.path, error))?;
        names.sort();
        Ok(names)
    }
}

pub(super) fn entry_name(name: &CStr) -> Option<&str> {
    name.to_str()
        .ok()
        .filter(|name| *name != "." && *name != "..")
}

pub(super) fn checked_entry_name<'a>(
    name: &'a CStr,
    root: &Path,
) -> Result<Option<&'a str>, ContainedError> {
    let name = name
        .to_str()
        .map_err(|_| io(root, std::io::ErrorKind::InvalidData.into()))?;
    Ok((name != "." && name != "..").then_some(name))
}

struct DirectoryStream(NonNull<libc::DIR>);

impl DirectoryStream {
    fn close(self) -> std::io::Result<()> {
        // SAFETY: this value owns the stream and descriptor; forgetting it prevents a second close.
        let result = unsafe { libc::closedir(self.0.as_ptr()) };
        std::mem::forget(self);
        if result == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }
}

impl Drop for DirectoryStream {
    fn drop(&mut self) {
        // SAFETY: this value exclusively owns the live stream and its descriptor.
        unsafe { libc::closedir(self.0.as_ptr()) };
    }
}
