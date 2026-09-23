use super::root;
use crate::ContainedError;
use std::fs;
use std::io::{Read, Seek, Write};
use std::os::unix::fs::{MetadataExt, symlink};
use std::path::Path;

#[test]
fn read_write_creation_is_exclusive_and_retains_the_root() {
    let (_temp, root) = root();
    let directory = root.open_subdirectory("sub").unwrap();
    let saved = root.path().join("saved");
    fs::rename(directory.path(), &saved).unwrap();
    symlink(root.path(), directory.path()).unwrap();
    let mut file = directory
        .create_read_write_file(Path::new("new"), 0o600)
        .unwrap();
    file.write_all(b"retained").unwrap();
    file.rewind().unwrap();
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"retained");
    assert_eq!(file.metadata().unwrap().mode() & 0o777, 0o600);
    assert_eq!(file.metadata().unwrap().nlink(), 1);
    assert!(matches!(
        directory.create_read_write_file(Path::new("new"), 0o600),
        Err(ContainedError::Io {
            kind: std::io::ErrorKind::AlreadyExists,
            ..
        })
    ));
    assert_eq!(fs::read(saved.join("new")).unwrap(), b"retained");
    assert!(!root.path().join("new").exists());
    directory.unlink_file(Path::new("new")).unwrap();
    assert!(!saved.join("new").exists());
}

#[test]
fn read_write_create_and_unlink_refuse_traversal_and_unsafe_entries() {
    let (_temp, root) = root();
    for name in ["../outside", "sub/nested"] {
        assert!(matches!(
            root.create_read_write_file(Path::new(name), 0o600),
            Err(ContainedError::Escape { .. })
        ));
        assert!(matches!(
            root.unlink_file(Path::new(name)),
            Err(ContainedError::Escape { .. })
        ));
    }
    fs::hard_link(root.path().join("plain.m4a"), root.path().join("hardlink")).unwrap();
    for name in ["link.m4a", "sub", "hardlink"] {
        assert!(matches!(
            root.create_read_write_file(Path::new(name), 0o600),
            Err(ContainedError::Io {
                kind: std::io::ErrorKind::AlreadyExists,
                ..
            })
        ));
        assert!(matches!(
            root.unlink_file(Path::new(name)),
            Err(ContainedError::NotRegular(_) | ContainedError::Escape { .. })
        ));
        assert!(fs::symlink_metadata(root.path().join(name)).is_ok());
    }
    assert_eq!(fs::read(root.path().join("plain.m4a")).unwrap(), b"bytes");
}

#[test]
fn revalidation_reports_missing_or_linked_paths_as_escape() {
    let (_temp, root) = root();
    let sub = root.open_subdirectory("sub").unwrap();
    fs::rename(sub.path(), root.path().join("saved")).unwrap();
    assert!(matches!(
        sub.revalidate(),
        Err(ContainedError::Escape { .. })
    ));
    symlink(root.path().join("saved"), sub.path()).unwrap();
    assert!(matches!(
        sub.revalidate(),
        Err(ContainedError::Escape { .. })
    ));
}
