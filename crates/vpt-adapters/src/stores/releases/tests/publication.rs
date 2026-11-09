use super::*;
use crate::{
    MemoryLedger,
    ledger::contract::releases::{committed, setup},
};
use std::os::unix::fs::MetadataExt;
use vpt_application::ports::{ReleaseHistory, ReleaseLedger, ReleasePayload};

fn prepared(roots: &Roots, files: &ReleaseFiles) -> ReleasePayload {
    let ledger = MemoryLedger::new();
    let (run, mut draft) = setup(&ledger);
    draft.destination = files
        .select(
            &draft.recording,
            draft.stage,
            draft.text.as_bytes(),
            draft.created_at,
            draft.provenance.captured_offset,
        )
        .unwrap()
        .path;
    draft.report_root = roots.stores.drafts.clone();
    let row = committed(ledger.prepare_release(run, &draft));
    ledger
        .release_payload(row.release.sequence)
        .unwrap()
        .unwrap()
}
#[test]
fn release_publishes_saved_copy_and_private_report_then_recovery_preserves_inodes() {
    let (_temp, roots) = fixture();
    let files = ReleaseFiles::open(&roots, &roots.stores.released).unwrap();
    let payload = prepared(&roots, &files);
    assert_eq!(files.publish(&payload).unwrap(), payload.release.version);
    assert_eq!(
        fs::read_to_string(&payload.release.destination).unwrap(),
        payload.text
    );
    assert_eq!(
        fs::read_to_string(&payload.release.report_path).unwrap(),
        payload.report
    );
    assert_eq!(
        fs::metadata(&payload.release.report_path).unwrap().mode() & 0o7777,
        0o600
    );
    let copy = fs::metadata(&payload.release.destination).unwrap().ino();
    let report = fs::metadata(&payload.release.report_path).unwrap().ino();
    drop(files);
    let files = ReleaseFiles::open(&roots, &roots.stores.released).unwrap();
    files.publish(&payload).unwrap();
    assert_eq!(
        fs::metadata(&payload.release.destination).unwrap().ino(),
        copy
    );
    assert_eq!(
        fs::metadata(&payload.release.report_path).unwrap().ino(),
        report
    );
}
#[test]
fn release_recovers_each_half_without_rewriting_existing_matching_file() {
    for report_first in [false, true] {
        let (_temp, roots) = fixture();
        let files = ReleaseFiles::open(&roots, &roots.stores.released).unwrap();
        let payload = prepared(&roots, &files);
        let (path, bytes) = if report_first {
            (&payload.release.report_path, &payload.report)
        } else {
            (&payload.release.destination, &payload.text)
        };
        fs::write(path, bytes).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
        let original = fs::metadata(path).unwrap().ino();
        files.publish(&payload).unwrap();
        assert_eq!(fs::metadata(path).unwrap().ino(), original);
        assert_eq!(
            fs::read_to_string(&payload.release.destination).unwrap(),
            payload.text
        );
        assert_eq!(
            fs::read_to_string(&payload.release.report_path).unwrap(),
            payload.report
        );
    }
}
#[test]
fn release_refuses_changed_copy_or_report_before_publishing_either_missing_half() {
    for report_changed in [false, true] {
        let (_temp, roots) = fixture();
        let files = ReleaseFiles::open(&roots, &roots.stores.released).unwrap();
        let payload = prepared(&roots, &files);
        let (changed, absent) = if report_changed {
            (&payload.release.report_path, &payload.release.destination)
        } else {
            (&payload.release.destination, &payload.release.report_path)
        };
        fs::write(changed, b"operator replacement").unwrap();
        fs::set_permissions(changed, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(files.publish(&payload).is_err());
        assert_eq!(fs::read(changed).unwrap(), b"operator replacement");
        assert!(!absent.exists());
    }
}
#[test]
fn release_refuses_public_or_hardlinked_report_without_changing_permissions() {
    let (_temp, roots) = fixture();
    let files = ReleaseFiles::open(&roots, &roots.stores.released).unwrap();
    let payload = prepared(&roots, &files);
    let report = &payload.release.report_path;
    fs::write(report, &payload.report).unwrap();
    fs::set_permissions(report, fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(
        files.publish(&payload),
        Err(ReleaseFileError::PrivateReport)
    );
    assert_eq!(fs::metadata(report).unwrap().mode() & 0o7777, 0o644);
    fs::set_permissions(report, fs::Permissions::from_mode(0o600)).unwrap();
    let link = roots.home.join("linked-report");
    fs::hard_link(report, &link).unwrap();
    assert_eq!(
        files.publish(&payload),
        Err(ReleaseFileError::PrivateReport)
    );
    fs::remove_file(link).unwrap();
    fs::set_permissions(&roots.stores.drafts, fs::Permissions::from_mode(0o755)).unwrap();
    files.publish(&payload).unwrap();
    assert_eq!(
        fs::metadata(&roots.stores.drafts).unwrap().mode() & 0o7777,
        0o755
    );
    assert_eq!(fs::metadata(report).unwrap().mode() & 0o7777, 0o600);
}
#[test]
fn release_rejects_corrupt_saved_bytes_or_foreign_report_path_before_writes() {
    let (_temp, roots) = fixture();
    let files = ReleaseFiles::open(&roots, &roots.stores.released).unwrap();
    let payload = prepared(&roots, &files);
    for which in 0..4 {
        let mut corrupt = payload.clone();
        match which {
            0 => corrupt.text.push_str("changed"),
            1 => corrupt.report.push_str("changed"),
            2 => corrupt.release.report_path = roots.stores.released.join("report.json"),
            _ => corrupt.release.destination = roots.home.join("copy.md"),
        }
        assert!(files.publish(&corrupt).is_err());
    }
    assert!(!payload.release.destination.exists());
    assert!(!payload.release.report_path.exists());
}

#[test]
fn release_refuses_a_replaced_report_root_before_writing_the_copy() {
    let (_temp, roots) = fixture();
    let files = ReleaseFiles::open(&roots, &roots.stores.released).unwrap();
    let payload = prepared(&roots, &files);
    fs::rename(&roots.stores.drafts, roots.home.join("original-drafts")).unwrap();
    fs::create_dir(&roots.stores.drafts).unwrap();
    assert!(files.publish(&payload).is_err());
    assert!(!payload.release.destination.exists());
    assert!(!payload.release.report_path.exists());
}
