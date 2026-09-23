use super::super::super::files::DatabaseFiles;
use super::super::super::{AccessPolicy, DatabaseError};
use super::super::support::{ROLES, root, snapshot};
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

#[test]
fn database_names_accept_only_one_bare_leaf() {
    let (_temp, root) = root();
    let before = snapshot(root.path());
    for path in [
        Path::new("../vpt.db"),
        Path::new("/vpt.db"),
        &root.path().join("vpt.db"),
        Path::new("vpt.db/child"),
        Path::new(""),
        Path::new("vpt\0.db"),
        Path::new("."),
        Path::new(".."),
        Path::new("./vpt.db"),
        Path::new("vpt.db/"),
        Path::new("vpt.db/."),
    ] {
        let error = DatabaseFiles::new(
            root.try_clone().unwrap(),
            path,
            AccessPolicy::WritableLedger,
        )
        .err()
        .expect("invalid leaf");
        assert!(error.is_containment(), "{path:?}: {error:?}");
    }
    assert_eq!(snapshot(root.path()), before);
}

#[test]
fn companions_append_exact_suffixes_to_the_database_leaf() {
    let (_temp, root) = root();
    let path = root.path().to_path_buf();
    let leaf = Path::new("copie.é.db-wal");
    let files = DatabaseFiles::new(root, leaf, AccessPolicy::WritableLedger).unwrap();
    for (role, name) in ROLES {
        files.open(role, true).unwrap();
        let mut expected = leaf.as_os_str().as_bytes().to_vec();
        expected.extend_from_slice(&name.as_bytes()[6..]);
        assert!(path.join(std::ffi::OsStr::from_bytes(&expected)).is_file());
    }
    assert_eq!(fs::read_dir(path).unwrap().count(), 4);
}

#[test]
fn missing_files_remain_ordinary_io_errors() {
    let (_temp, root) = root();
    let path = root.path().to_path_buf();
    let before = snapshot(&path);
    let files =
        DatabaseFiles::new(root, Path::new("vpt.db"), AccessPolicy::WritableLedger).unwrap();
    for (role, _) in ROLES {
        for result in [
            files.open(role, false).map(|_| ()),
            files.remove(role, true),
        ] {
            assert!(matches!(
                result,
                Err(DatabaseError::Contained(crate::ContainedError::Io {
                    kind: std::io::ErrorKind::NotFound,
                    ..
                }))
            ));
        }
    }
    assert_eq!(snapshot(&path), before);
}

#[test]
fn storage_errors_keep_their_kind_without_becoming_containment() {
    for kind in [
        std::io::ErrorKind::PermissionDenied,
        std::io::ErrorKind::StorageFull,
        std::io::ErrorKind::Other,
    ] {
        let cause = crate::ContainedError::Io {
            path: "vpt.db".into(),
            kind,
        };
        let error = DatabaseError::from(cause.clone());
        assert!(!error.is_containment());
        assert!(matches!(error, DatabaseError::Contained(inner) if inner == cause));
    }
    let error = DatabaseError::from(rusqlite::Error::InvalidQuery);
    assert!(!error.is_containment());
    assert!(matches!(
        error,
        DatabaseError::Sqlite(rusqlite::Error::InvalidQuery)
    ));
}
