use super::*;

#[test]
fn a_displaced_note_that_cannot_be_read_goes_back_before_vpt_reports() {
    for support in [SWAP].into_iter().chain(volumes::NO_SWAP) {
        let (_temp, root, stores) = store();
        let target = root.join("note.md");
        fs::write(&target, b"old").unwrap();
        let result = stores.publish_with(
            &target,
            Some(stores.digest_bytes(b"old")),
            b"intended",
            support,
            |point| {
                if matches!(point, Checkpoint::Checked) {
                    fs::remove_file(&target).unwrap();
                    fs::create_dir(&target).unwrap();
                    fs::write(target.join("inside"), b"operator").unwrap();
                }
                Ok(())
            },
        );
        assert_eq!(
            result,
            Err(StoreError::Changed(target.clone())),
            "{support:?}"
        );
        assert_eq!(fs::read(target.join("inside")).unwrap(), b"operator");
        assert_eq!(entries(&root), ["note.md"], "{support:?}");
    }
}
