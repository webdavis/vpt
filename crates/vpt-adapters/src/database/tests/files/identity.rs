use super::*;
use crate::database::FileRole;

#[test]
fn replacement_after_identity_preflight_is_refused_before_file_permissions_change() {
    check_late_change(true);
}

#[test]
fn removal_after_identity_preflight_is_refused_without_recreating_the_file() {
    check_late_change(false);
}

fn check_late_change(replace: bool) {
    use std::os::unix::fs::PermissionsExt;

    for (role, name) in ROLES
        .into_iter()
        .filter(|(role, _)| *role != FileRole::Journal)
    {
        let (_temp, root) = root();
        let owner = DatabaseFiles::new(
            root.try_clone().unwrap(),
            Path::new("vpt.db"),
            AccessPolicy::WritableLedger,
        )
        .unwrap();
        let retained: Vec<_> = ROLES
            .into_iter()
            .filter(|(role, _)| *role != FileRole::Journal)
            .map(|(role, _)| owner.open(role, true).unwrap())
            .collect();
        let next = DatabaseFiles::new(
            root.try_clone().unwrap(),
            Path::new("vpt.db"),
            AccessPolicy::WritableLedger,
        )
        .unwrap();
        next.pin_to(&owner).unwrap();
        let path = root.path().join(name);
        fs::rename(&path, root.path().join("held-file")).unwrap();
        if replace {
            fs::write(&path, b"operator replacement").unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        }
        let before = snapshot(root.path());
        for result in [
            next.open(role, false),
            next.open(role, true),
            next.create(role),
        ] {
            assert!(
                matches!(result, Err(DatabaseError::Contained(ContainedError::Escape { path: refused, .. })) if refused == path)
            );
        }
        assert!(
            matches!(next.metadata(role, false), Err(DatabaseError::Contained(ContainedError::Escape { path: refused, .. })) if refused == path)
        );
        assert!(
            snapshot(root.path()) == before,
            "late refusal changed files for {role:?}"
        );
        drop(retained);
    }
}
