use super::SIDE_FILES;
use crate::{Access, ContainedError, RootDir};
use std::fs::File;
use std::io::ErrorKind;
use std::os::fd::{AsFd, AsRawFd, FromRawFd};
use std::os::unix::fs::MetadataExt;
use std::path::Path;

pub(super) fn acquire(
    root: &RootDir,
    name: &str,
    access: Access,
    create: bool,
) -> Result<File, ContainedError> {
    root.revalidate()?;
    let path = root.path().join(name);
    let file = match root.open_file(Path::new(name), access) {
        Ok(file) => file,
        Err(ContainedError::Io {
            kind: ErrorKind::NotFound,
            ..
        }) if create => {
            let name_c = root.c_name(Path::new(name))?;
            // SAFETY: the retained root and checked name stay live; creation transfers one descriptor.
            let fd = unsafe {
                libc::openat(
                    root.as_fd().as_raw_fd(),
                    name_c.as_ptr(),
                    libc::O_RDWR
                        | libc::O_CREAT
                        | libc::O_EXCL
                        | libc::O_NOFOLLOW
                        | libc::O_CLOEXEC
                        | libc::O_NONBLOCK,
                    0o600 as libc::c_uint,
                )
            };
            if fd >= 0 {
                // SAFETY: successful exclusive creation returned this fresh owned descriptor.
                unsafe { File::from_raw_fd(fd) }
            } else {
                let error = std::io::Error::last_os_error();
                if error.kind() != ErrorKind::AlreadyExists {
                    return Err(ContainedError::Io {
                        path,
                        kind: error.kind(),
                    });
                }
                root.open_file(Path::new(name), access)?
            }
        }
        Err(error) => return Err(error),
    };
    verify(root, name, &file)?;
    Ok(file)
}

pub(super) fn verify(root: &RootDir, name: &str, file: &File) -> Result<(), ContainedError> {
    root.revalidate()?;
    let path = root.path().join(name);
    let escaped = || ContainedError::Escape {
        root: root.path().into(),
        path: path.clone(),
    };
    let owned = file.metadata().map_err(|error| ContainedError::Io {
        path: path.clone(),
        kind: error.kind(),
    })?;
    if !owned.is_file() {
        return Err(ContainedError::NotRegular(path));
    }
    if owned.nlink() != 1 {
        return Err(escaped());
    }
    let name = root.c_name(Path::new(name))?;
    let mut named = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: retained root anchors the no-follow lookup and named is writable output storage.
    if unsafe {
        libc::fstatat(
            root.as_fd().as_raw_fd(),
            name.as_ptr(),
            named.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    } != 0
    {
        let error = std::io::Error::last_os_error();
        return Err(if error.kind() == ErrorKind::NotFound {
            escaped()
        } else {
            ContainedError::Io {
                path,
                kind: error.kind(),
            }
        });
    }
    // SAFETY: successful fstatat initialized named.
    let named = unsafe { named.assume_init() };
    if named.st_mode & libc::S_IFMT != libc::S_IFREG
        || named.st_nlink != 1
        || owned.dev() != named.st_dev as u64
        || owned.ino() != named.st_ino
    {
        return Err(escaped());
    }
    root.revalidate()
}

pub(super) fn validate(root: &RootDir) -> Result<(), ContainedError> {
    for name in SIDE_FILES {
        match acquire(root, name, Access::Read, false) {
            Ok(_)
            | Err(ContainedError::Io {
                kind: ErrorKind::NotFound,
                ..
            }) => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}
