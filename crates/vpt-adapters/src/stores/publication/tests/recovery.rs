use super::*;
use vpt_application::ports::DirtyPublication;
use vpt_application::{RepairError, repair_publication};
use vpt_domain::time::UtcInstant;

pub(super) fn pending(stores: &FilesystemStores, target: &Path) -> DirtyPublication {
    DirtyPublication {
        target: target.to_path_buf(),
        expected_previous: Some(stores.digest_bytes(b"old")),
        intended: stores.digest_bytes(b"intended"),
        recorded_at: UtcInstant { secs: 1 },
    }
}

fn kept_at_published_note(target: &Path, kept: PathBuf, cause: Option<&str>) -> StoreError {
    StoreError::EditKept(KeptEdit {
        note: target.to_path_buf(),
        kept,
        versions: KeptVersions::PublishedAtNote,
        cause: cause.map(|cause| Box::new(StoreError::Io(cause.into()))),
    })
}

#[test]
fn a_failed_swap_back_keeps_the_edit_and_recovery_keeps_reporting_it() {
    let (_temp, root, stores) = store();
    let target = root.join("note.md");
    fs::write(&target, b"old").unwrap();
    let result = publish_swapping(
        &stores,
        &target,
        Some(stores.digest_bytes(b"old")),
        b"intended",
        |point| match point {
            Checkpoint::Checked => {
                editor_save(&target, b"operator");
                Ok(())
            }
            Checkpoint::SwapBack => Err(StoreError::Io("injected swap-back failure".into())),
            _ => Ok(()),
        },
    );
    let kept = kept_file(&root);
    assert_eq!(
        result,
        Err(kept_at_published_note(
            &target,
            kept.clone(),
            Some("injected swap-back failure")
        ))
    );
    assert_eq!(fs::read(&target).unwrap(), b"intended");
    assert_eq!(fs::read(&kept).unwrap(), b"operator");
    let entry = pending(&stores, &target);
    let repaired = repair_publication(&stores, &entry, &|_| Some(b"intended".to_vec()));
    assert_eq!(
        repaired,
        Err(RepairError::Stores(kept_at_published_note(
            &target, kept, None
        )))
    );
}

#[test]
fn recovery_after_a_crash_past_the_swap_reports_the_kept_version_until_it_is_removed() {
    let (_temp, root, stores) = store();
    let target = root.join("note.md");
    let kept = root.join(".note.md.vpt-kept-1-1");
    fs::write(&target, b"intended").unwrap();
    fs::write(&kept, b"operator").unwrap();
    fs::write(root.join(".note.md.vpt-kept-1-2"), b"intended").unwrap();
    fs::write(root.join(".note.md.vpt-1-3"), b"partial").unwrap();
    let entry = pending(&stores, &target);
    let render = |_: &Path| Some(b"intended".to_vec());
    assert_eq!(
        repair_publication(&stores, &entry, &render),
        Err(RepairError::Stores(kept_at_published_note(
            &target,
            kept.clone(),
            None
        )))
    );
    assert_eq!(fs::read(&kept).unwrap(), b"operator");
    assert!(!root.join(".note.md.vpt-kept-1-2").exists());
    fs::remove_file(&kept).unwrap();
    assert_eq!(repair_publication(&stores, &entry, &render), Ok(false));
}
