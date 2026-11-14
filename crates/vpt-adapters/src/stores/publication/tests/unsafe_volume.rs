use super::*;

const NEITHER: RenameSupport = RenameSupport {
    swap: false,
    exclusive: false,
    hard_links: false,
};

#[test]
fn a_volume_without_exclusive_rename_or_hard_links_refuses_before_touching_anything() {
    for previous in [Some(&b"old"[..]), None] {
        let (_temp, root, stores) = store();
        let target = root.join("note.md");
        if let Some(previous) = previous {
            fs::write(&target, previous).unwrap();
        }
        let expected = previous.map(|bytes| stores.digest_bytes(bytes));
        let mut reached = Vec::new();
        let result = stores.publish_with(&target, expected, b"intended", NEITHER, |point| {
            reached.push(point);
            Ok(())
        });
        assert_eq!(result, Err(StoreError::UnsafeVolume(target.clone())));
        assert!(reached.is_empty(), "{previous:?}");
        let names: Vec<String> = previous.map(|_| "note.md".into()).into_iter().collect();
        assert_eq!(entries(&root), names);
        if let Some(previous) = previous {
            assert_eq!(fs::read(&target).unwrap(), previous);
        }
    }
}
