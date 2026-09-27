use super::super::{OpenError, SqliteLedger};
use super::{database, map, seed, supervised};
use crate::{ContainedError, RootDir, database::snapshot};
use std::os::unix::fs::symlink;
use vpt_application::ports::LedgerError;

#[test]
fn initial_database_link_is_refused_by_both_open_modes() {
    supervised("ledger_boundary_initial_link");
}

#[test]
fn reconnect_refuses_a_replaced_root_without_touching_its_contents() {
    supervised("ledger_boundary_replaced_root");
}

#[test]
fn reconnect_refuses_a_missing_root() {
    supervised("ledger_boundary_missing_root");
}

#[test]
fn reconnect_refuses_a_database_link_without_touching_its_target() {
    supervised("ledger_boundary_database_link");
}

#[test]
fn transaction_revalidates_before_commit_and_rolls_back_through_the_held_root() {
    supervised("ledger_boundary_precommit");
}

#[test]
fn a_mapped_query_failure_retains_its_containment_error() {
    supervised("ledger_boundary_latched");
}

pub(super) fn run(scenario: &str, root: &RootDir) {
    if scenario == "ledger_boundary_initial_link" {
        let target = root.path().with_file_name("target");
        std::fs::write(&target, b"private target").unwrap();
        symlink(&target, root.path().join("vpt.db")).unwrap();
        let before = snapshot(root.path());
        let target_before = snapshot(&target);
        let expected = OpenError::Contained(ContainedError::NotRegular(root.path().join("vpt.db")));
        assert_eq!(SqliteLedger::open(root).unwrap_err(), expected);
        assert_eq!(
            SqliteLedger::open_read_only(root.path()).unwrap_err(),
            expected
        );
        assert_eq!(snapshot(root.path()), before);
        assert_eq!(snapshot(&target), target_before);
        return;
    }
    let ledger = seed(root);
    let held = root.path().with_file_name("held");
    match scenario {
        "ledger_boundary_missing_root" | "ledger_boundary_replaced_root" => {
            std::fs::rename(root.path(), &held).unwrap();
            if scenario == "ledger_boundary_replaced_root" {
                std::fs::create_dir(root.path()).unwrap();
                std::fs::write(root.path().join("vpt.db"), b"replacement").unwrap();
            }
            let before = snapshot(root.path().parent().unwrap());
            assert_eq!(
                ledger.schema_version(),
                Err(LedgerError::PathEscape(root.path().into()))
            );
            assert_eq!(snapshot(root.path().parent().unwrap()), before);
        }
        "ledger_boundary_database_link" => {
            let path = root.path().join("vpt.db");
            let target = root.path().with_file_name("target");
            std::fs::rename(&path, &target).unwrap();
            symlink(&target, &path).unwrap();
            let before = snapshot(root.path());
            let target_before = snapshot(&target);
            assert_eq!(ledger.schema_version(), Err(LedgerError::PathEscape(path)));
            assert_eq!(snapshot(root.path()), before);
            assert_eq!(snapshot(&target), target_before);
        }
        "ledger_boundary_precommit" => {
            let result = ledger.transaction(|t| {
                t.execute(
                    "INSERT INTO dirty_publications VALUES('uncommitted', NULL, 'value', 1)",
                    [],
                )
                .map_err(map)?;
                std::fs::rename(root.path(), &held).unwrap();
                std::fs::create_dir(root.path()).unwrap();
                std::fs::write(root.path().join("sentinel"), b"untouched").unwrap();
                Ok(())
            });
            assert_eq!(result, Err(LedgerError::PathEscape(root.path().into())));
            assert_eq!(
                std::fs::read(root.path().join("sentinel")).unwrap(),
                b"untouched"
            );
            assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
            database(&RootDir::open(&held).unwrap(), |c| {
                assert_eq!(
                    c.query_row("SELECT count(*) FROM dirty_publications", [], |row| row
                        .get::<_, i64>(0))
                        .unwrap(),
                    0
                );
                Ok(())
            })
            .unwrap();
        }
        "ledger_boundary_latched" => {
            let observed = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
            let target = root.path().with_file_name("target");
            std::fs::write(&target, b"untouched").unwrap();
            let target_before = snapshot(&target);
            let wal = root.path().join("vpt.db-wal");
            let result = observed.read(|c| {
                std::fs::rename(&wal, root.path().with_file_name("held-wal")).unwrap();
                symlink(&target, &wal).unwrap();
                let mapped = c
                    .query_row("SELECT count(*) FROM dirty_publications", [], |row| {
                        row.get::<_, i64>(0)
                    })
                    .map_err(map);
                assert!(
                    matches!(mapped, Err(LedgerError::Io(_))),
                    "repository closure maps the SQL error: {mapped:?}"
                );
                mapped
            });
            assert_eq!(result, Err(LedgerError::PathEscape(wal)));
            assert_eq!(snapshot(&target), target_before);
        }
        _ => panic!("unknown ledger boundary scenario"),
    }
}
