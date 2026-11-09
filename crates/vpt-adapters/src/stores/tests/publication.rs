use super::*;
use vpt_application::ports::DirtyPublication;
use vpt_application::{RepairError, repair_publication};
use vpt_domain::time::UtcInstant;

#[test]
fn publication_preserves_a_target_changed_after_the_application_digest_check() {
    for exists in [false, true] {
        let (_temp, root, stores) = store();
        let target = root.join("note.md");
        if exists {
            std::fs::write(&target, b"old").unwrap();
        }
        let entry = DirtyPublication {
            target: target.clone(),
            expected_previous: exists.then(|| stores.digest_bytes(b"old")),
            intended: stores.digest_bytes(b"intended"),
            recorded_at: UtcInstant { secs: 1 },
        };
        let result = repair_publication(&stores, &entry, &|_| {
            std::fs::write(&target, b"operator content").unwrap();
            Some(b"intended".to_vec())
        });
        assert_eq!(result, Err(RepairError::TargetModified(target.clone())));
        assert_eq!(std::fs::read(&target).unwrap(), b"operator content");
        assert_eq!(entries(&root), ["note.md"]);
    }
}
