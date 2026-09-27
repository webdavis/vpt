mod boundaries;
mod copies;
mod fixture;
mod leasing;
mod peers;
mod queries;

use super::*;
use crate::database::{child_for_root, test_root};
use std::time::Instant;

fn bounded(scenario: &str) {
    let (temp, _root) = test_root();
    let child = child_for_root(scenario, temp.path(), std::process::id(), Instant::now())
        .expect("owned title child");
    let output = child.finish().expect("bounded title child");
    assert!(
        output.status.success(),
        "title child: stdout={} stderr={}",
        output.stdout.escape_ascii(),
        output.stderr.escape_ascii()
    );
}

pub(crate) fn run_scenario(scenario: &str, root: RootDir, ready: fn()) {
    eprintln!("VPT_PHASE {scenario}");
    match scenario {
        "title_wal" => queries::wal(&root),
        "title_modes" => copies::modes(&root),
        "title_refresh_twice" => copies::refresh_twice(&root),
        "title_existing" => copies::existing(&root),
        "title_schema" => queries::schema(&root),
        "title_missing" => copies::missing(&root),
        "title_source_unchanged" => copies::source_unchanged(&root),
        "title_metacharacters" => queries::metacharacters(&root),
        "title_store" => copies::store(&root),
        "title_missing_state" => boundaries::missing_state(&root),
        "title_state_replaced" => boundaries::state_replaced(&root),
        "title_leaves" => boundaries::leaves(&root),
        "title_copy_replaced" => boundaries::copy_replaced(&root),
        "title_empty" => queries::empty(&root),
        "title_case" => queries::case_sensitive(&root),
        "title_quoted" => queries::quoted(&root),
        "title_companions" => copies::companions(&root),
        "title_copy_failure" => boundaries::copy_failure(&root),
        "title_refreshed_bytes" => copies::refreshed_bytes(&root),
        "title_existing_snapshot" => copies::existing_snapshot(&root),
        "title_dry" => boundaries::dry(&root),
        "title_hardlinks" => boundaries::hardlinks(&root),
        "title_lease_shared" => leasing::shared(&root),
        "title_lease_exclusive" => leasing::exclusive(&root),
        "title_lease_entries" => leasing::entries(&root),
        "title_lease_create" => leasing::create(&root),
        "title_lease_owner" => leasing::owner(&root),
        "title_lease_replaced" => leasing::replaced(&root),
        "title_convenience" => boundaries::convenience(&root),
        "title_peer_seed" | "title_peer_reader" | "title_peer_busy" | "title_peer_refresh" => {
            peers::participant(scenario, &root, ready)
        }
        _ => panic!("unknown owned title scenario"),
    }
}

#[test]
fn the_title_is_read_from_the_copy_by_file_name_including_rows_still_in_the_live_wal() {
    bounded("title_wal");
}

#[test]
fn the_copy_lives_under_the_state_directory_with_restrictive_modes() {
    bounded("title_modes");
}

#[test]
fn a_second_refresh_overwrites_the_copy_in_place_and_removes_nothing() {
    bounded("title_refresh_twice");
}

#[test]
fn an_existing_copy_is_read_without_refreshing_and_an_absent_one_is_unavailable() {
    bounded("title_existing");
}

#[test]
fn a_changed_schema_yields_unavailable() {
    bounded("title_schema");
}

#[test]
fn a_missing_live_database_yields_unavailable_and_makes_no_copy() {
    bounded("title_missing");
}

#[test]
fn the_live_container_is_left_with_no_new_entries() {
    bounded("title_source_unchanged");
}

#[test]
fn like_metacharacters_in_a_file_name_match_only_that_name() {
    bounded("title_metacharacters");
}

#[test]
fn the_store_answers_through_its_port_and_unavailable_without_titles() {
    bounded("title_store");
}

#[test]
fn a_missing_title_state_is_unavailable_without_refusing_the_recorder() {
    bounded("title_missing_state");
}

#[test]
fn retained_title_state_refuses_replacement_before_writing_a_copy() {
    bounded("title_state_replaced");
}

#[test]
fn every_copied_database_leaf_is_checked_before_refresh_or_query() {
    bounded("title_leaves");
}

#[test]
fn an_open_title_copy_refuses_a_replaced_copy_directory() {
    bounded("title_copy_replaced");
}

#[test]
fn empty_null_and_missing_labels_are_unavailable() {
    bounded("title_empty");
}

#[test]
fn filename_matching_is_literal_and_case_sensitive() {
    bounded("title_case");
}

#[test]
fn discovered_table_identifiers_are_quoted() {
    bounded("title_quoted");
}

#[test]
fn missing_source_companions_clear_stale_private_companions() {
    bounded("title_companions");
}

#[test]
fn copy_failure_discards_previous_titles_without_reopening() {
    bounded("title_copy_failure");
}

#[test]
fn refreshed_queries_preserve_private_database_and_wal_bytes() {
    bounded("title_refreshed_bytes");
}

#[test]
fn existing_queries_preserve_the_entire_private_tree() {
    bounded("title_existing_snapshot");
}

#[test]
fn dry_run_neither_opens_nor_refreshes_title_databases() {
    bounded("title_dry");
}

#[test]
fn copy_acquisition_refuses_multiply_linked_destinations() {
    bounded("title_hardlinks");
}

#[test]
fn shared_copy_lease_blocks_refresh_without_changing_metadata() {
    bounded("title_lease_shared");
}

#[test]
fn exclusive_copy_lease_downgrades_and_releases_on_drop() {
    bounded("title_lease_exclusive");
}

#[test]
fn copy_lease_refuses_multiple_links_and_nonregular_entries() {
    bounded("title_lease_entries");
}

#[test]
fn copy_lease_creates_a_private_main_file() {
    bounded("title_lease_create");
}

#[test]
fn actual_title_reader_keeps_lease_and_allows_an_independent_ledger() {
    bounded("title_lease_owner");
}

#[test]
fn copy_lease_refuses_replaced_main_or_root_identity() {
    bounded("title_lease_replaced");
}

#[test]
fn convenience_openers_preserve_typed_containment_and_absence() {
    bounded("title_convenience");
}
