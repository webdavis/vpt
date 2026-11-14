use super::{ContainedError, Kind, RootDir, io};
use std::os::fd::AsRawFd;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

impl RootDir {
    pub(crate) fn create_owned_directory(&self, name: &str) -> Result<Self, ContainedError> {
        self.revalidate()?;
        let path = self.leaf(Path::new(name))?;
        let component = self.c_name(&path)?;
        // SAFETY: the retained directory and validated NUL-terminated leaf outlive mkdirat.
        if unsafe { libc::mkdirat(self.file.as_raw_fd(), component.as_ptr(), 0o700) } != 0 {
            return Err(io(&path, std::io::Error::last_os_error()));
        }
        let directory = self.open_subdirectory(name)?;
        directory
            .file
            .set_permissions(std::fs::Permissions::from_mode(0o700))
            .map_err(|error| io(&path, error))?;
        directory.revalidate()?;
        Ok(directory)
    }

    pub(crate) fn unlink_unchecked(&self, path: &Path) -> Result<(), ContainedError> {
        remove_entry(self, &self.leaf(path)?, false)
    }

    pub(crate) fn remove_owned_directory(&self, directory: &Self) -> Result<(), ContainedError> {
        self.revalidate()?;
        self.leaf(directory.path())?;
        directory.revalidate()?;
        clear(directory, &mut 4096, 0)?;
        directory.revalidate()?;
        remove_entry(self, directory.path(), true)
    }
}

fn clear(root: &RootDir, remaining: &mut usize, depth: usize) -> Result<(), ContainedError> {
    if depth > 8 {
        return Err(io(root.path(), std::io::ErrorKind::FileTooLarge.into()));
    }
    for name in root.names_bounded(*remaining)? {
        if *remaining == 0 {
            return Err(io(root.path(), std::io::ErrorKind::FileTooLarge.into()));
        }
        *remaining -= 1;
        let path = Path::new(&name);
        let directory = root.stat(path)?.kind == Kind::Directory;
        if directory {
            let child = root.open_subdirectory(&name)?;
            clear(&child, remaining, depth + 1)?;
            child.revalidate()?;
        }
        remove_entry(root, path, directory)?;
    }
    Ok(())
}

fn remove_entry(root: &RootDir, path: &Path, directory: bool) -> Result<(), ContainedError> {
    let name = root.c_name(path)?;
    let flags = if directory { libc::AT_REMOVEDIR } else { 0 };
    // SAFETY: root retains its descriptor and name is a live NUL-terminated leaf. unlinkat
    // removes that entry only; it never follows a leaf symlink or uses an unvalidated path.
    if unsafe { libc::unlinkat(root.file.as_raw_fd(), name.as_ptr(), flags) } != 0 {
        return Err(io(&root.path.join(path), std::io::Error::last_os_error()));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
