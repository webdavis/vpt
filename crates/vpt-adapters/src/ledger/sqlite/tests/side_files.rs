use super::super::{OpenError, SqliteLedger};
use super::seed;
use crate::test_support::{root, snapshot};
use std::os::unix::fs::{PermissionsExt, symlink};
use vpt_application::ports::LedgerError;

#[test]
fn a_linked_side_file_is_refused_by_both_open_modes_without_changes() {
    for leaf in ["vpt.db-journal", "vpt.db-wal", "vpt.db-shm"] {
        for hard in [false, true] {
            let (_temp, root) = root();
            drop(seed(&root));
            let target = root.path().with_file_name("target");
            std::fs::write(&target, b"private target").unwrap();
            let path = root.path().join(leaf);
            if hard {
                std::fs::hard_link(&target, &path).unwrap();
            } else {
                symlink(&target, &path).unwrap();
            }
            let before = snapshot(root.path());
            let target_before = snapshot(&target);
            let refused = |result: Result<Option<SqliteLedger>, OpenError>| matches!(result, Err(OpenError::Ledger(LedgerError::PathEscape(refused))) if refused == path);
            assert!(
                refused(SqliteLedger::open(&root).map(Some)),
                "{leaf} hard={hard}"
            );
            assert!(
                refused(SqliteLedger::open_read_only(root.path())),
                "{leaf} hard={hard}"
            );
            assert_eq!(snapshot(root.path()), before);
            assert_eq!(snapshot(&target), target_before);
        }
    }
}

#[test]
fn a_new_ledger_and_its_side_files_are_private() {
    let (_temp, root) = root();
    let ledger = seed(&root);
    for leaf in ["vpt.db", "vpt.db-wal", "vpt.db-shm"] {
        let mode = std::fs::metadata(root.path().join(leaf))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "{leaf}");
    }
    drop(ledger);
    let mode = std::fs::metadata(root.path().join("vpt.db"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
}
