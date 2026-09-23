use super::super::files::DatabaseFiles;
use super::super::{AccessPolicy, DatabaseError};
use super::support::{ROLES, root, snapshot};
use crate::{ContainedError, RootDir};
use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::symlink;
use std::path::Path;

mod names;
mod policies;
mod refusal;

#[test]
fn lazy_open_stays_with_the_acquired_directory() {
    let (temp, root) = root();
    let original = root.path().to_path_buf();
    let saved = temp.path().join("saved");
    let files =
        DatabaseFiles::new(root, Path::new("vpt.db"), AccessPolicy::WritableLedger).expect("files");
    files.revalidate().expect("valid before replacement");
    fs::rename(&original, &saved).expect("retain original inode");
    fs::create_dir(&original).expect("replacement");
    for (_, name) in ROLES {
        fs::write(original.join(name), b"replacement").unwrap();
    }
    let before = snapshot(&original);
    for (role, name) in ROLES {
        files
            .open(role, true)
            .expect("held root open")
            .write_all(b"owned")
            .unwrap();
        assert_eq!(fs::read(saved.join(name)).unwrap(), b"owned");
        assert!(files.revalidate().unwrap_err().is_containment());
        files.remove(role, true).expect("contained cleanup");
        assert!(!saved.join(name).exists());
    }
    assert_eq!(snapshot(&original), before);
}

#[test]
fn existing_files_and_cleanup_stay_with_a_replaced_ancestor() {
    let (temp, parent) = root();
    let root = parent.subdirectory("private").unwrap();
    for (_, name) in ROLES {
        fs::write(root.path().join(name), b"held").unwrap();
    }
    let files =
        DatabaseFiles::new(root, Path::new("vpt.db"), AccessPolicy::WritableLedger).unwrap();
    let original = parent.path().to_path_buf();
    fs::rename(&original, temp.path().join("saved")).unwrap();
    let outside = temp.path().join("outside");
    fs::create_dir(&outside).unwrap();
    fs::create_dir(outside.join("private")).unwrap();
    for (_, name) in ROLES {
        fs::write(outside.join("private").join(name), b"outside").unwrap();
    }
    symlink(&outside, &original).unwrap();
    let before = snapshot(&outside);
    assert!(files.revalidate().unwrap_err().is_containment());
    for (role, name) in ROLES {
        let mut bytes = Vec::new();
        files
            .open(role, false)
            .unwrap()
            .read_to_end(&mut bytes)
            .unwrap();
        assert_eq!(bytes, b"held");
        files.remove(role, true).unwrap();
        assert!(!temp.path().join("saved/private").join(name).exists());
        files
            .open(role, true)
            .unwrap()
            .write_all(b"new held")
            .unwrap();
        assert_eq!(
            fs::read(temp.path().join("saved/private").join(name)).unwrap(),
            b"new held"
        );
    }
    assert_eq!(snapshot(&outside), before);
}

#[test]
fn revalidation_classifies_missing_link_and_wrong_type_as_escape() {
    for replacement in ["missing", "link", "file", "directory"] {
        let (temp, root) = root();
        let original = root.path().to_path_buf();
        let files =
            DatabaseFiles::new(root, Path::new("vpt.db"), AccessPolicy::WritableLedger).unwrap();
        fs::rename(&original, temp.path().join("saved")).unwrap();
        match replacement {
            "link" => symlink(temp.path().join("saved"), &original).unwrap(),
            "file" => fs::write(&original, b"replacement").unwrap(),
            "directory" => fs::create_dir(&original).unwrap(),
            _ => {}
        }
        assert!(matches!(
            files.revalidate(),
            Err(DatabaseError::Contained(ContainedError::Escape { .. }))
        ));
    }
    let (_temp, root) = root();
    assert!(matches!(
        RootDir::open(&root.path().join("absent")),
        Err(ContainedError::Io {
            kind: std::io::ErrorKind::NotFound,
            ..
        })
    ));
}
