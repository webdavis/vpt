use super::{LogError, LogRefusal, RootDir};
use crate::contained::descriptor::open_at;
use crate::contained::private_state::effective_user;
use std::fs::File;
use std::io::ErrorKind;
use std::os::fd::{AsFd, AsRawFd};
use std::os::unix::fs::{MetadataExt, PermissionsExt};

pub(super) fn acquire(root: &RootDir, creation_mode: u32) -> Result<File, LogError> {
    let existing = match named(root) {
        Ok(stat) => {
            check(
                stat.st_mode.into(),
                stat.st_uid,
                stat.st_nlink.into(),
                effective_user(),
                0o600,
            )?;
            true
        }
        Err(LogError::Io(ErrorKind::NotFound)) => false,
        Err(error) => return Err(error),
    };
    let flags =
        libc::O_RDWR | libc::O_APPEND | libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    let flags = if existing {
        flags
    } else {
        flags | libc::O_CREAT | libc::O_EXCL
    };
    let file =
        open_at(root.as_fd(), c"events.jsonl", flags, creation_mode).map_err(
            |error| match error.raw_os_error() {
                Some(libc::ELOOP | libc::EISDIR | libc::ENXIO) => {
                    LogError::Unsafe(LogRefusal::NotRegular)
                }
                _ => LogError::Io(error.kind()),
            },
        )?;
    if !existing {
        let mode = file
            .metadata()
            .map_err(|error| LogError::Io(error.kind()))?
            .mode()
            & 0o7777;
        if mode & !0o600 != 0 {
            return Err(LogError::Unsafe(LogRefusal::Mode));
        }
        verify_with_mode(root, &file, effective_user(), mode)?;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))
            .map_err(|error| LogError::Io(error.kind()))?;
    }
    Ok(file)
}

fn named(root: &RootDir) -> Result<libc::stat, LogError> {
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: the retained directory and fixed name are live; stat receives initialized output.
    let status = unsafe {
        libc::fstatat(
            root.file.as_raw_fd(),
            c"events.jsonl".as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if status != 0 {
        return Err(LogError::Io(std::io::Error::last_os_error().kind()));
    }
    // SAFETY: successful fstatat initialized every field read below.
    Ok(unsafe { stat.assume_init() })
}

fn check(
    mode: u32,
    owner: u32,
    links: u64,
    expected_owner: u32,
    expected_mode: u32,
) -> Result<(), LogError> {
    let refusal = if mode & u32::from(libc::S_IFMT) != u32::from(libc::S_IFREG) {
        Some(LogRefusal::NotRegular)
    } else if owner != expected_owner {
        Some(LogRefusal::Owner)
    } else if mode & 0o7777 != expected_mode {
        Some(LogRefusal::Mode)
    } else if links != 1 {
        Some(LogRefusal::MultipleLinks)
    } else {
        None
    };
    refusal.map_or(Ok(()), |refusal| Err(LogError::Unsafe(refusal)))
}

pub(super) fn verify_file(root: &RootDir, file: &File, owner: u32) -> Result<(), LogError> {
    verify_with_mode(root, file, owner, 0o600)
}

fn verify_with_mode(root: &RootDir, file: &File, owner: u32, mode: u32) -> Result<(), LogError> {
    root.validate_private_state().map_err(LogError::State)?;
    let held = file
        .metadata()
        .map_err(|error| LogError::Io(error.kind()))?;
    check(held.mode(), held.uid(), held.nlink(), owner, mode)?;
    let current = named(root).map_err(|error| match error {
        LogError::Io(ErrorKind::NotFound) => LogError::Unsafe(LogRefusal::Replaced),
        error => error,
    })?;
    check(
        current.st_mode.into(),
        current.st_uid,
        current.st_nlink.into(),
        owner,
        mode,
    )?;
    let device = u64::try_from(current.st_dev).map_err(|_| LogError::Io(ErrorKind::InvalidData))?;
    if held.dev() != device || held.ino() != current.st_ino {
        return Err(LogError::Unsafe(LogRefusal::Replaced));
    }
    root.validate_private_state().map_err(LogError::State)
}
