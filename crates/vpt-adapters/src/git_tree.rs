use std::io;
use std::path::{Path, PathBuf};

pub fn enclosing_git_tree(path: &Path) -> io::Result<Option<PathBuf>> {
    locate(path, |entry| entry.symlink_metadata().map(|_| ()))
}

fn locate(
    path: &Path,
    mut inspect: impl FnMut(&Path) -> io::Result<()>,
) -> io::Result<Option<PathBuf>> {
    for ancestor in path.ancestors() {
        match inspect(&ancestor.join(".git")) {
            Ok(()) => return Ok(Some(ancestor.to_path_buf())),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_nested_git_entry_wins() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        let inner = dir.path().join("inner");
        std::fs::create_dir_all(inner.join(".git")).unwrap();
        assert_eq!(
            enclosing_git_tree(&inner.join("missing/leaf")).unwrap(),
            Some(inner)
        );
    }

    #[test]
    fn git_file_counts_as_repository() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".git"), b"gitdir: owned").unwrap();
        assert_eq!(
            enclosing_git_tree(dir.path()).unwrap(),
            Some(dir.path().into())
        );
    }

    #[test]
    fn git_symlink_counts_without_following() {
        let dir = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(dir.path().join("absent"), dir.path().join(".git")).unwrap();
        assert_eq!(
            enclosing_git_tree(dir.path()).unwrap(),
            Some(dir.path().into())
        );
    }

    #[test]
    fn not_found_continues_the_ancestor_walk() {
        let mut inspected = Vec::new();
        let result = locate(Path::new("/owned/nested"), |path| {
            inspected.push(path.to_path_buf());
            if path == Path::new("/owned/.git") {
                Ok(())
            } else {
                Err(io::ErrorKind::NotFound.into())
            }
        });
        assert_eq!(result.unwrap(), Some("/owned".into()));
        assert_eq!(
            inspected,
            [
                PathBuf::from("/owned/nested/.git"),
                PathBuf::from("/owned/.git")
            ]
        );
    }

    #[test]
    fn no_entries_returns_none() {
        let mut inspected = Vec::new();
        let result = locate(Path::new("/owned"), |path| {
            inspected.push(path.to_path_buf());
            Err(io::ErrorKind::NotFound.into())
        });
        assert_eq!(result.unwrap(), None);
        assert_eq!(
            inspected,
            [PathBuf::from("/owned/.git"), PathBuf::from("/.git")]
        );
    }

    fn assert_inspection_error(kind: io::ErrorKind) {
        let mut calls = 0;
        let result = locate(Path::new("/owned/nested"), |_| {
            calls += 1;
            Err(kind.into())
        });
        assert_eq!(result.unwrap_err().kind(), kind);
        assert_eq!(calls, 1);
    }

    #[test]
    fn permission_denied_stops_the_walk() {
        assert_inspection_error(io::ErrorKind::PermissionDenied);
    }

    #[test]
    fn not_a_directory_stops_the_walk() {
        assert_inspection_error(io::ErrorKind::NotADirectory);
    }
}
