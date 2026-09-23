use super::support::root;
use crate::database::{AccessPolicy, Database, DatabaseError};
use std::path::Path;
use std::time::Duration;
mod callbacks;
mod controls;
mod durability;
mod lifetime;

pub(super) use controls::controls_case;
pub(super) use durability::transaction_failure;

#[test]
fn exclusive_creation_refuses_an_existing_file() {
    supervised("owner_exclusive_create");
}
#[test]
fn file_controls_report_lock_persistence_and_moved_identity() {
    supervised("owner_controls");
}
#[test]
fn readonly_policies_preserve_files_and_disable_checkpoint_on_close() {
    supervised("owner_readonly");
}
#[test]
fn pathname_and_access_callbacks_enforce_names_and_output_bounds() {
    supervised("owner_path_access");
}
pub(super) use lifetime::{error_latches, invalid_drop, retry_close};

#[test]
fn failed_file_sync_is_not_a_successful_transaction() {
    supervised("owner_transaction_sync");
}
#[test]
fn failed_directory_sync_is_not_a_successful_transaction() {
    supervised("owner_transaction_directory");
}
#[test]
fn failed_partial_write_is_not_a_successful_transaction() {
    supervised("owner_transaction_short");
}
#[test]
fn failed_write_is_not_a_successful_transaction() {
    supervised("owner_transaction_write");
}

#[test]
fn busy_close_retains_registration_until_statement_finalization() {
    supervised("owner_retry_close");
}
#[test]
fn mapped_query_errors_cannot_erase_connection_local_failures() {
    supervised("owner_latches");
}
#[test]
fn dropping_an_owner_with_a_live_statement_aborts_safely() {
    use std::os::unix::process::ExitStatusExt;
    let (temp, _root) = root();
    let mut child = super::children::child("owner_invalid_drop", temp.path()).unwrap();
    child
        .ready()
        .expect("invalid owner reached live statement state");
    child.send("continue").unwrap();
    let output = child.finish().unwrap();
    assert_eq!(output.status.signal(), Some(libc::SIGABRT), "{output:?}");
}

fn supervised(scenario: &str) {
    let (temp, _root) = root();
    let output = super::children::child(scenario, temp.path())
        .unwrap()
        .finish()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
}

#[test]
fn short_reads_zero_the_unread_tail() {
    supervised("owner_short_read");
}
#[test]
fn partial_writes_report_failure_after_the_exact_prefix() {
    supervised("owner_partial_write");
}
#[test]
fn callbacks_refuse_invalid_offsets_and_required_null_pointers() {
    supervised("owner_invalid_offsets");
}
#[test]
fn filesystem_callbacks_refuse_temporary_unknown_names_and_flags() {
    supervised("owner_unknown_open");
}
#[test]
fn lazy_journals_and_cleanup_retain_the_original_directory() {
    supervised("owner_root_switch");
}
#[test]
fn callback_panics_poison_the_connection_without_unwinding() {
    supervised("owner_panic");
}
#[test]
fn requested_directory_sync_failure_is_propagated_after_unlink() {
    supervised("owner_delete_sync");
}
#[test]
fn synchronization_and_truncation_failures_are_consumed() {
    supervised("owner_faults");
}

pub(super) fn callback_case(scenario: &str, root: crate::RootDir) {
    callbacks::run(scenario, root);
}

pub(super) fn failed_open(root: crate::RootDir) {
    let path = root.path().to_path_buf();
    let mut registration = crate::database::registration::Registration::new(
        root,
        Path::new("vpt.db"),
        AccessPolicy::WritableLedger,
    )
    .unwrap();
    let consumed = registration.fail_directory_sync();
    let name = registration.name().to_owned();
    let result = Database::open_registered(registration, Duration::from_millis(20));
    // SAFETY: registration lookup retains no pointer after checking for absence.
    assert!(unsafe { rusqlite::ffi::sqlite3_vfs_find(name.as_ptr()) }.is_null());
    assert!(
        consumed.get(),
        "directory synchronization must be reached after acquisition"
    );
    assert!(
        path.join("vpt.db").is_file(),
        "file must have been acquired"
    );
    assert!(matches!(
        result,
        Err(DatabaseError::Io {
            operation: "directory sync",
            ..
        })
    ));
    let root = crate::RootDir::open(&path).unwrap();
    let mut db = Database::open(
        root,
        Path::new("vpt.db"),
        AccessPolicy::WritableLedger,
        Duration::from_millis(20),
    )
    .unwrap();
    db.with_connection::<_, DatabaseError>(|connection| {
        connection.execute_batch("CREATE TABLE item(value)")?;
        Ok(())
    })
    .unwrap();
    db.try_close().unwrap();
}

#[test]
fn failed_open_releases_acquired_files_and_registration() {
    let (temp, _root) = root();
    let output = super::children::child("owner_failed_open", temp.path())
        .unwrap()
        .finish()
        .unwrap();
    assert!(output.status.success(), "{:?}", output);
}

#[test]
fn a_rollback_transaction_uses_only_authorized_files() {
    let (_temp, root) = root();
    let path = root.path().to_path_buf();
    let mut db = Database::open(
        root,
        Path::new("vpt.db"),
        AccessPolicy::WritableLedger,
        Duration::from_millis(20),
    )
    .expect("open");
    db.with_connection::<_, DatabaseError>(|connection| {
        connection.execute_batch(
            "CREATE TABLE item(value INTEGER); BEGIN IMMEDIATE;
            INSERT INTO item VALUES (7); ROLLBACK;",
        )?;
        let count: i64 = connection.query_row("SELECT count(*) FROM item", [], |row| row.get(0))?;
        assert_eq!(count, 0);
        Ok(())
    })
    .unwrap();
    db.try_close().unwrap();
    let names: Vec<_> = std::fs::read_dir(path)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(names, [std::ffi::OsString::from("vpt.db")]);
}

#[test]
fn an_attached_database_cannot_select_the_native_filesystem() {
    let (temp, root) = root();
    let outside = temp.path().join("outside.db");
    let mut db = Database::open(
        root,
        Path::new("vpt.db"),
        AccessPolicy::WritableLedger,
        Duration::from_millis(20),
    )
    .unwrap();
    let uri = format!("file:{}?vfs=unix", outside.display());
    let result = db.with_connection::<_, DatabaseError>(|connection| {
        connection.execute("ATTACH DATABASE ?1 AS outside", [&uri])?;
        Ok(())
    });
    assert!(result.is_err());
    assert!(!outside.exists());
    db.try_close().unwrap();
}

#[test]
fn operations_refuse_a_replaced_root_before_lending_the_connection() {
    let (temp, root) = root();
    let path = root.path().to_path_buf();
    let mut db = Database::open(
        root,
        Path::new("vpt.db"),
        AccessPolicy::WritableLedger,
        Duration::from_millis(20),
    )
    .unwrap();
    std::fs::rename(&path, temp.path().join("held")).unwrap();
    std::fs::create_dir(&path).unwrap();
    let mut ran = false;
    let result = db.with_connection::<_, DatabaseError>(|_| {
        ran = true;
        Ok(())
    });
    assert!(matches!(result, Err(DatabaseError::Contained(_))));
    assert!(!ran);
    db.try_close().unwrap();
}

#[test]
fn successful_operations_revalidate_before_returning() {
    let (temp, root) = root();
    let path = root.path().to_path_buf();
    let mut db = Database::open(
        root,
        Path::new("vpt.db"),
        AccessPolicy::WritableLedger,
        Duration::from_millis(20),
    )
    .unwrap();
    let result = db.with_connection::<_, DatabaseError>(|_| {
        std::fs::rename(&path, temp.path().join("held")).unwrap();
        std::fs::create_dir(&path).unwrap();
        Ok(())
    });
    assert!(matches!(result, Err(DatabaseError::Contained(_))));
    db.try_close().unwrap();
}

#[test]
fn closed_owners_refuse_work_and_close_idempotently() {
    let (_temp, root) = root();
    let mut db = Database::open(
        root,
        Path::new("vpt.db"),
        AccessPolicy::WritableLedger,
        Duration::from_millis(20),
    )
    .unwrap();
    db.try_close().unwrap();
    assert!(db.with_connection::<_, DatabaseError>(|_| Ok(())).is_err());
    db.try_close().unwrap();
}
