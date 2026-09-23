use super::super::super::AccessPolicy;
use super::super::super::files::DatabaseFiles;
use super::super::support::{ROLES, root, snapshot};
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::Path;

#[test]
fn every_role_refuses_symbolic_links_directories_and_multiple_links() {
    for policy in [
        AccessPolicy::WritableLedger,
        AccessPolicy::ObservationalLedger,
        AccessPolicy::RefreshedTitleCopy,
        AccessPolicy::ExistingTitleCopy,
    ] {
        for (role, name) in ROLES {
            for kind in ["link", "directory", "hardlink"] {
                let (temp, root) = root();
                let path = root.path().join(name);
                let outside = temp.path().join("outside");
                fs::write(&outside, b"outside target").unwrap();
                fs::set_permissions(&outside, fs::Permissions::from_mode(0o640)).unwrap();
                match kind {
                    "link" => symlink(&outside, &path).unwrap(),
                    "directory" => fs::create_dir(&path).unwrap(),
                    _ => fs::hard_link(&outside, &path).unwrap(),
                }
                let before = snapshot(temp.path());
                let files = DatabaseFiles::new(root, Path::new("vpt.db"), policy).unwrap();
                for create in [false, true] {
                    let error = files.open(role, create).expect_err("unsafe role");
                    assert!(
                        error.is_containment(),
                        "{policy:?} {role:?} {kind}: {error:?}"
                    );
                }
                assert!(files.remove(role, true).is_err());
                assert_eq!(snapshot(temp.path()), before);
            }
        }
    }
}

#[test]
fn writable_removal_classifies_unsafe_entries_as_containment() {
    for (role, name) in ROLES {
        for kind in ["link", "directory", "hardlink"] {
            let (temp, root) = root();
            let outside = temp.path().join("outside");
            fs::write(&outside, b"outside").unwrap();
            let path = root.path().join(name);
            match kind {
                "link" => symlink(&outside, &path).unwrap(),
                "directory" => fs::create_dir(&path).unwrap(),
                _ => fs::hard_link(&outside, &path).unwrap(),
            }
            let before = snapshot(temp.path());
            let files = DatabaseFiles::new(root, Path::new("vpt.db"), AccessPolicy::WritableLedger)
                .unwrap();
            assert!(files.remove(role, true).unwrap_err().is_containment());
            assert_eq!(snapshot(temp.path()), before);
        }
    }
}
