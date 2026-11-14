use super::*;

const LINKS_ONLY: RenameSupport = RenameSupport {
    swap: false,
    exclusive: false,
    hard_links: true,
};

#[test]
fn a_staging_name_left_after_linking_never_reports_the_replaced_version_as_a_save() {
    let (_temp, root, stores) = store();
    let target = root.join("note.md");
    fs::write(&target, b"old").unwrap();
    let result = stores.publish_with(
        &target,
        Some(stores.digest_bytes(b"old")),
        b"intended",
        LINKS_ONLY,
        |point| {
            if matches!(point, Checkpoint::Linked) {
                let staged = entries(&root)
                    .into_iter()
                    .find(|name| name.starts_with(".note.md.vpt-") && !name.contains("kept"))
                    .map(|name| root.join(name))
                    .unwrap();
                fs::remove_file(&staged).unwrap();
                fs::create_dir(&staged).unwrap();
                fs::write(staged.join("inside"), b"blocks removal").unwrap();
            }
            Ok(())
        },
    );
    assert!(matches!(result, Err(StoreError::Io(_))), "{result:?}");
    assert_eq!(fs::read(&target).unwrap(), b"intended");
    assert!(
        !entries(&root).iter().any(|name| name.contains("vpt-kept")),
        "{:?}",
        entries(&root)
    );
}
