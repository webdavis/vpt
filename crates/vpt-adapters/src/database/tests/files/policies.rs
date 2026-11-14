use super::super::super::files::DatabaseFiles;
use super::super::super::{AccessPolicy, DatabaseError, FileRole};
use super::super::support::{ROLES, root, snapshot};
use std::fs;
use std::io::{Read, Seek, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;

const POLICIES: [(AccessPolicy, [bool; 4]); 4] = [
    (AccessPolicy::WritableLedger, [true, true, true, true]),
    (
        AccessPolicy::ObservationalLedger,
        [false, false, false, false],
    ),
    (
        AccessPolicy::RefreshedTitleCopy,
        [false, false, false, true],
    ),
    (
        AccessPolicy::ExistingTitleCopy,
        [false, false, false, false],
    ),
];

#[test]
fn reopening_already_private_files_preserves_their_metadata_and_bytes() {
    for (policy, _) in POLICIES {
        let (_temp, root) = root();
        for (_, name) in ROLES {
            let path = root.path().join(name);
            fs::write(&path, b"private bytes").unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        }
        let before = snapshot(root.path());
        let files =
            DatabaseFiles::new(root.try_clone().unwrap(), Path::new("vpt.db"), policy).unwrap();
        for (role, _) in ROLES {
            for create in [false, true] {
                drop(files.open(role, create).unwrap());
            }
        }
        assert!(
            snapshot(root.path()) == before,
            "opening private files changed metadata for {policy:?}"
        );
    }
}

#[test]
fn observation_neither_creates_nor_repairs() {
    for policy in [
        AccessPolicy::ObservationalLedger,
        AccessPolicy::ExistingTitleCopy,
    ] {
        let (_temp, root) = root();
        let path = root.path().to_path_buf();
        for (_, name) in ROLES {
            fs::write(path.join(name), b"fixture").unwrap();
            fs::set_permissions(path.join(name), fs::Permissions::from_mode(0o640)).unwrap();
        }
        let before = snapshot(&path);
        let files = DatabaseFiles::new(root, Path::new("vpt.db"), policy).unwrap();
        for (role, _) in ROLES {
            for create in [false, true] {
                let mut file = files.open(role, create).expect("read-only writable probe");
                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes).unwrap();
                assert_eq!(bytes, b"fixture");
                assert!(file.write_all(b"changed").is_err());
            }
            assert!(files.remove(role, true).is_err());
        }
        assert_eq!(snapshot(&path), before);
    }
}

#[test]
fn each_policy_creates_and_removes_only_its_writable_roles() {
    for (policy, permissions) in POLICIES {
        for ((role, name), writable) in ROLES.into_iter().zip(permissions) {
            let (_temp, root) = root();
            let path = root.path().to_path_buf();
            let before = snapshot(&path);
            let files = DatabaseFiles::new(root, Path::new("vpt.db"), policy).unwrap();
            assert!(files.open(role, false).is_err());
            let result = files.open(role, true);
            if writable {
                let mut file = result.expect("create writable role");
                assert_eq!(file.metadata().unwrap().mode() & 0o777, 0o600);
                assert_eq!(file.metadata().unwrap().nlink(), 1);
                assert_ne!(
                    // SAFETY: file owns the live descriptor inspected by fcntl.
                    unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFD) } & libc::FD_CLOEXEC,
                    0
                );
                file.write_all(b"new").unwrap();
                file.rewind().unwrap();
                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes).unwrap();
                assert_eq!(bytes, b"new");
                files.remove(role, false).unwrap();
                assert!(!path.join(name).exists());
            } else {
                assert!(result.is_err());
                let error = files.remove(role, false).unwrap_err();
                assert!(!error.is_containment());
                assert!(matches!(
                    error,
                    DatabaseError::Io {
                        operation: "remove",
                        kind: std::io::ErrorKind::PermissionDenied
                    }
                ));
                assert_eq!(snapshot(&path), before);
            }
        }
    }
}

#[test]
fn permission_repair_and_descriptor_write_access_follow_each_policy() {
    for (policy, permissions) in POLICIES {
        for ((role, name), writable) in ROLES.into_iter().zip(permissions) {
            let (_temp, root) = root();
            let path = root.path().join(name);
            fs::write(&path, b"preserved").unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
            let before = snapshot(&path);
            let files = DatabaseFiles::new(root, Path::new("vpt.db"), policy).unwrap();
            let file = files.open(role, true).unwrap();
            // SAFETY: file owns the live descriptor inspected by fcntl.
            let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) };
            assert_eq!(
                flags & libc::O_ACCMODE,
                if writable {
                    libc::O_RDWR
                } else {
                    libc::O_RDONLY
                }
            );
            assert_eq!(
                file.metadata().unwrap().mode() & 0o777,
                if writable { 0o600 } else { 0o640 }
            );
            assert_eq!(fs::read(&path).unwrap(), b"preserved");
            if !writable {
                assert_eq!(snapshot(&path), before);
            }
        }
    }
}

#[test]
fn refreshed_titles_do_not_remove_copied_database_or_wal() {
    let (_temp, root) = root();
    for (_, name) in ROLES {
        fs::write(root.path().join(name), b"copied").unwrap();
    }
    let path = root.path().to_path_buf();
    let files =
        DatabaseFiles::new(root, Path::new("vpt.db"), AccessPolicy::RefreshedTitleCopy).unwrap();
    for role in [FileRole::Database, FileRole::Journal, FileRole::Wal] {
        let before = snapshot(&path);
        assert!(files.remove(role, true).is_err());
        assert_eq!(snapshot(&path), before);
    }
    files.remove(FileRole::SharedMemory, true).unwrap();
    assert!(!path.join("vpt.db-shm").exists());
}
