mod callbacks;
mod lifetime;
mod mapping;

use super::wal::supervised;
use crate::RootDir;
use crate::database::{AccessPolicy, DatabaseError, files::DatabaseFiles};
use rusqlite::ffi::*;
use std::{path::Path, rc::Rc};

const SHARED: i32 = SQLITE_SHM_LOCK | SQLITE_SHM_SHARED;
const EXCLUSIVE: i32 = SQLITE_SHM_LOCK | SQLITE_SHM_EXCLUSIVE;
const UNLOCK_SHARED: i32 = SQLITE_SHM_UNLOCK | SQLITE_SHM_SHARED;
const UNLOCK_EXCLUSIVE: i32 = SQLITE_SHM_UNLOCK | SQLITE_SHM_EXCLUSIVE;

fn files(root: &RootDir, policy: AccessPolicy) -> Rc<DatabaseFiles> {
    Rc::new(DatabaseFiles::new(root.try_clone().unwrap(), Path::new("vpt.db"), policy).unwrap())
}

fn busy<T: std::fmt::Debug>(result: Result<T, DatabaseError>) {
    assert!(
        matches!(result, Err(DatabaseError::Sqlite(rusqlite::Error::SqliteFailure(e, _))) if e.code == rusqlite::ErrorCode::DatabaseBusy),
        "{result:?}"
    );
}

#[test]
fn first_writable_opener_invalidates_stale_shared_memory() {
    supervised("shm_initialize");
}
#[test]
fn initialization_never_truncates_under_another_lifetime_holder() {
    supervised("shm_lifetime");
}
#[test]
fn exclusive_initializer_makes_other_openers_busy() {
    supervised("shm_busy_initializer");
}
#[test]
fn simultaneous_openers_keep_a_lifetime_holder_until_both_leave() {
    supervised("shm_racing_openers");
}
#[test]
fn readonly_initialization_retains_usable_slot_locks_and_retries() {
    supervised("shm_readonly_initialize");
}
#[test]
fn closing_one_connection_preserves_another_connections_slots() {
    supervised("shm_close");
}
#[test]
fn shared_and_exclusive_slot_contention_covers_the_native_byte_ranges() {
    supervised("shm_slots");
}
#[test]
fn invalid_flags_ranges_and_upgrades_leave_existing_slots_held() {
    supervised("shm_invalid_locks");
}
#[test]
fn missing_region_without_extension_leaves_the_file_size_unchanged() {
    supervised("shm_missing");
}
#[test]
fn multiple_regions_are_shared_at_their_requested_offsets() {
    supervised("shm_regions");
}
#[test]
fn unsupported_region_sizes_and_negative_indexes_are_refused() {
    supervised("shm_invalid_map");
}
#[test]
fn map_failure_releases_mappings_lifetime_and_slot_locks_on_close() {
    supervised("shm_map_failure");
}
#[test]
fn readonly_maps_never_extend_or_remove_companions() {
    supervised("shm_readonly_map");
}
#[test]
fn readonly_mapping_reports_its_policy_without_changing_files() {
    supervised("shm_readonly_policy");
}
#[test]
fn readonly_uninitialized_mapping_defers_initialization() {
    supervised("shm_uninitialized_map");
}
#[test]
fn readonly_extension_is_refused_without_changing_files() {
    supervised("shm_readonly_extension");
}
#[test]
fn invalid_slot_flags_and_ranges_are_typed_refusals() {
    supervised("shm_invalid_slot_arguments");
}
#[test]
fn mapping_and_cleanup_use_the_retained_directory_after_replacement() {
    supervised("shm_root_switch");
}
#[test]
fn deletion_waits_for_the_last_shared_memory_owner() {
    supervised("shm_delete_active");
}
#[test]
fn observational_mapping_has_no_write_permission() {
    use std::os::unix::process::ExitStatusExt;
    let (temp, _root) = super::support::root();
    let mut process = super::children::child("shm_protection", temp.path()).unwrap();
    process.ready().unwrap();
    process.send("continue").unwrap();
    let output = process.finish().unwrap();
    assert!(
        matches!(output.status.signal(), Some(libc::SIGBUS | libc::SIGSEGV)),
        "{output:?}"
    );
}

pub(super) fn run(scenario: &str, root: RootDir) {
    match scenario {
        s if s.starts_with("shm_callback_") => callbacks::run(s, root),
        "shm_initialize"
        | "shm_lifetime"
        | "shm_busy_initializer"
        | "shm_readonly_initialize"
        | "shm_uninitialized_map"
        | "shm_close"
        | "shm_slots"
        | "shm_invalid_locks" => lifetime::run(scenario, root),
        "shm_invalid_slot_arguments" => lifetime::run(scenario, root),
        "shm_racing_openers" => lifetime::race(root),
        _ => mapping::run(scenario, root),
    }
}

#[test]
fn callback_maps_missing_and_nonzero_extend_without_latching_errors() {
    supervised("shm_callback_extend");
}
#[test]
fn callback_reports_readonly_and_uninitialized_protocol_outcomes() {
    supervised("shm_callback_readonly");
}
#[test]
fn callback_null_output_is_a_typed_refusal() {
    supervised("shm_callback_null");
}
#[test]
fn callback_slot_contention_and_unmap_preserve_other_owners() {
    supervised("shm_callback_slots");
}
#[test]
fn callback_barrier_preserves_a_usable_connection() {
    supervised("shm_callback_barrier");
}
#[test]
fn callback_persistent_unmap_preserves_the_companion() {
    supervised("shm_callback_persistent");
}
#[test]
fn readonly_wal_open_returns_actual_access_without_changing_files() {
    supervised("shm_callback_wal_readonly");
}
