mod cloning;
mod copying;
mod errors;
mod handles;
mod listing;

use super::*;
use crate::database::{child_for_root, snapshot, test_root};
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

struct Fixture {
    recordings: PathBuf,
    audio: PathBuf,
    store: VoiceMemosStore,
}

impl Fixture {
    fn new(root: &RootDir) -> Self {
        let recordings = root.path().join("Recordings");
        fs::create_dir(&recordings).expect("recordings");
        for name in APPLE_SUBDIRECTORIES {
            let path = recordings.join(name);
            fs::create_dir(&path).expect("Apple child");
            fs::write(path.join("inner.m4a"), b"not a candidate").expect("nested audio");
        }
        fs::write(recordings.join("a.m4a"), b"hello world").expect("source");
        fs::write(
            recordings.join("metadata.db"),
            b"synthetic private metadata",
        )
        .expect("database");
        let audio = root.path().join("audio");
        fs::create_dir(&audio).expect("destination");
        let store = VoiceMemosStore::open(&recordings).expect("store");
        Self {
            recordings,
            audio,
            store,
        }
    }

    fn source(&self) -> File {
        File::open(self.recordings.join("a.m4a")).expect("owned source descriptor")
    }

    fn identity(&self) -> (u64, u64) {
        RootDir::open(&self.audio)
            .expect("destination root")
            .identity()
            .expect("identity")
    }

    fn unchanged<T>(&self, action: impl FnOnce() -> T) -> T {
        let before = snapshot(&self.recordings);
        let outcome = action();
        assert_eq!(
            snapshot(&self.recordings),
            before,
            "complete source tree changed"
        );
        outcome
    }
}

fn bounded(scenario: &str) {
    let (temp, _root) = test_root();
    let child = child_for_root(scenario, temp.path(), std::process::id(), Instant::now())
        .expect("root-owned recorder child");
    let output = child.finish().expect("bounded recorder child");
    assert!(
        output.status.success(),
        "recorder child: stdout={:?} stderr={:?}",
        output.stdout,
        output.stderr
    );
}

pub(crate) fn run_scenario(scenario: &str, root: RootDir) {
    eprintln!("VPT_PHASE {scenario}");
    let fixture = Fixture::new(&root);
    match scenario {
        "recorder_listing" => listing::depth_one(&fixture),
        "recorder_flags" => listing::flags(&fixture),
        "recorder_candidate_guards" => listing::candidate_guards(&fixture),
        "recorder_counts" => listing::counts(&fixture),
        "recorder_root_guards" => listing::root_guards(&fixture),
        "recorder_open_guards" => handles::open_guards(&fixture),
        "recorder_descriptor" => handles::descriptor(&fixture),
        "recorder_reads" => handles::reads(&fixture),
        "recorder_native_clone" => cloning::native(&fixture),
        "recorder_destination_identity" => cloning::identity(&fixture),
        "recorder_clone_names" => cloning::names(&fixture),
        "recorder_clone_collisions" => cloning::collisions(&fixture),
        "recorder_exdev" => cloning::exdev(&fixture),
        "recorder_clone_errors" => cloning::errors(&fixture),
        "recorder_copy_collisions" => copying::collisions(&fixture),
        "recorder_copy_bounds" => copying::bounds(&fixture),
        "recorder_copy_errors" => copying::errors(&fixture),
        "recorder_copy_owned_error" => copying::owned_error(&fixture),
        _ => panic!("unknown recorder scenario"),
    }
}

#[test]
fn candidates_are_sorted_lowercase_m4a_files_at_depth_one() {
    bounded("recorder_listing");
}

#[test]
fn candidate_carries_exact_metadata_and_the_complete_nonzero_flags_word() {
    bounded("recorder_flags");
}

#[test]
fn candidate_refuses_links_directories_missing_and_escaped_names() {
    bounded("recorder_candidate_guards");
}

#[test]
fn apple_subdirectory_counts_are_shallow_and_never_follow_links() {
    bounded("recorder_counts");
}

#[test]
fn source_root_refuses_absence_links_and_listing_after_replacement() {
    bounded("recorder_root_guards");
}

#[test]
fn open_is_read_only_close_on_exec_and_refuses_nonregular_or_escaped_sources() {
    bounded("recorder_open_guards");
}

#[test]
fn metadata_and_container_reads_use_the_retained_source_descriptor() {
    bounded("recorder_descriptor");
}

#[test]
fn descriptor_reads_are_exact_positional_and_overflow_safe() {
    bounded("recorder_reads");
}

#[test]
fn native_clone_uses_the_open_descriptor_after_pathname_substitution() {
    bounded("recorder_native_clone");
}

#[test]
fn cloning_refuses_changed_destination_identity_before_creation() {
    bounded("recorder_destination_identity");
}

#[test]
fn cloning_checks_leaf_names_and_refuses_linked_destination_roots() {
    bounded("recorder_clone_names");
}

#[test]
fn native_clone_preserves_every_existing_destination_entry() {
    bounded("recorder_clone_collisions");
}

#[test]
fn exdev_alone_falls_back_to_a_private_descriptor_byte_copy() {
    bounded("recorder_exdev");
}

#[test]
fn clone_errors_keep_exact_classes_without_byte_copy_fallback() {
    bounded("recorder_clone_errors");
}

#[test]
fn byte_copy_never_claims_or_changes_an_existing_destination() {
    bounded("recorder_copy_collisions");
}

#[test]
fn byte_copy_bounds_each_chunk_and_stops_at_the_initial_descriptor_size() {
    bounded("recorder_copy_bounds");
}

#[test]
fn byte_copy_reports_short_reads_no_space_and_other_write_errors() {
    bounded("recorder_copy_errors");
}

#[test]
fn a_copy_writer_error_never_disclaims_destination_ownership() {
    bounded("recorder_copy_owned_error");
}
