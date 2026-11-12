use super::*;
use crate::stores::tests::store;
use std::{
    os::unix::fs::{MetadataExt, symlink},
    path::Path,
};
use vpt_application::ports::{ArtifactKind, ArtifactOwner, RetentionStatus, Stores};
use vpt_domain::digest::Sha256Digest;
const ID: &str = "2026-10-01T090000-0123456789ab";

fn artifact(kind: ArtifactKind, path: &Path) -> RetentionArtifact {
    RetentionArtifact {
        kind,
        owner: ArtifactOwner::parse(kind, ID).unwrap(),
        path: path.into(),
        digest: Sha256Digest([99; 32]),
        status: RetentionStatus::Published,
        trashed_at: None,
    }
}
fn note(kind: ArtifactKind) -> String {
    if kind == ArtifactKind::Brief {
        format!(
            "---\nvpt-schema: 1\nvpt-kind: brief\nvpt-occasion: {ID}\n---\nOperator prose\n<!-- vpt:brief start -->\nSaved pack\n<!-- vpt:brief end -->\n"
        )
    } else {
        format!(
            "---\nvpt-schema: 1\nvpt-stage: {}\nvpt-recording: {ID}\n---\nOperator prose\n<!-- vpt:links start -->\n<!-- vpt:links end -->\n<!-- vpt:content start -->\nSaved content\n<!-- vpt:content end -->\n",
            kind.as_str()
        )
    }
}

#[test]
fn note_and_brief_checks_accept_operator_prose_and_return_exact_current_digest() {
    let (_temp, root, stores) = store();
    for kind in [
        ArtifactKind::Transcript,
        ArtifactKind::Analysis,
        ArtifactKind::Brief,
    ] {
        let selected = artifact(kind, &root.join("pinned.md"));
        let text = note(kind);
        std::fs::write(&selected.path, &text).unwrap();
        let before = std::fs::metadata(&selected.path).unwrap();
        assert_eq!(
            stores.check_retention(&selected),
            Ok(RetentionCheck::Owned(stores.digest_bytes(text.as_bytes())))
        );
        let after = std::fs::metadata(&selected.path).unwrap();
        assert_eq!(
            (
                before.ino(),
                before.mtime(),
                before.mtime_nsec(),
                before.mode()
            ),
            (after.ino(), after.mtime(), after.mtime_nsec(), after.mode())
        );
        assert_eq!(std::fs::read_to_string(&selected.path).unwrap(), text);
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
    }
}

#[test]
fn notes_require_matching_identity_stage_schema_and_complete_markers() {
    let (_temp, root, stores) = store();
    for kind in [
        ArtifactKind::Transcript,
        ArtifactKind::Analysis,
        ArtifactKind::Brief,
    ] {
        let selected = artifact(kind, &root.join("pinned.md"));
        let original = note(kind);
        for text in [
            original.replace(ID, "2026-10-01T090000-ffffffffffff"),
            original.replace(kind.as_str(), "foreign"),
            original.replace("vpt-schema: 1", "vpt-schema: 2"),
            original.replace(" end -->", " lost -->"),
            original.replace(" start -->", " lost -->"),
        ] {
            std::fs::write(&selected.path, &text).unwrap();
            assert_eq!(
                stores.check_retention(&selected),
                Ok(RetentionCheck::Modified),
                "{kind:?}"
            );
            assert_eq!(std::fs::read_to_string(&selected.path).unwrap(), text);
        }
    }
}

#[test]
fn non_note_artifacts_require_the_saved_digest_and_accept_arbitrary_binary_bytes() {
    let (_temp, root, stores) = store();
    for kind in [
        ArtifactKind::Audio,
        ArtifactKind::EngineOutput,
        ArtifactKind::Release,
        ArtifactKind::RedactionReport,
    ] {
        let mut selected = artifact(kind, &root.join("owned"));
        selected.digest = stores.digest_bytes(b"saved\xff");
        std::fs::write(&selected.path, b"saved\xff").unwrap();
        assert_eq!(
            stores.check_retention(&selected),
            Ok(RetentionCheck::Owned(selected.digest))
        );
        std::fs::write(&selected.path, b"other\xff").unwrap();
        assert_eq!(
            stores.check_retention(&selected),
            Ok(RetentionCheck::Modified)
        );
    }
}

#[test]
fn absent_pinned_file_stays_missing_even_when_another_note_has_the_same_owner() {
    let (_temp, root, stores) = store();
    std::fs::write(root.join("renamed.md"), note(ArtifactKind::Transcript)).unwrap();
    assert_eq!(
        stores.check_retention(&artifact(ArtifactKind::Transcript, &root.join("pinned.md"))),
        Ok(RetentionCheck::Missing)
    );
    assert_eq!(std::fs::read_dir(root).unwrap().count(), 1);
}

#[test]
fn malformed_and_oversized_notes_are_preserved_without_disclosing_content() {
    let (_temp, root, stores) = store();
    let selected = artifact(ArtifactKind::Transcript, &root.join("pinned.md"));
    for bytes in [b"SECRET\xff".as_slice(), b"SECRET malformed frontmatter"] {
        std::fs::write(&selected.path, bytes).unwrap();
        assert_eq!(
            stores.check_retention(&selected),
            Ok(RetentionCheck::Modified)
        );
        assert_eq!(std::fs::read(&selected.path).unwrap(), bytes);
    }
    std::fs::File::create(&selected.path)
        .unwrap()
        .set_len(vpt_domain::note::MAX_RENDERED as u64 + 1)
        .unwrap();
    assert_eq!(
        stores.check_retention(&selected),
        Ok(RetentionCheck::Modified)
    );
}

#[test]
fn retention_checks_refuse_symlinks_escape_and_replaced_roots() {
    let (temp, root, stores) = store();
    let outside = temp.path().join("outside.md");
    std::fs::write(&outside, note(ArtifactKind::Transcript)).unwrap();
    let link = root.join("linked.md");
    symlink(&outside, &link).unwrap();
    let directory = root.join("directory.md");
    std::fs::create_dir(&directory).unwrap();
    for path in [&outside, &link, &directory, &root.join("../outside.md")] {
        assert!(matches!(
            stores.check_retention(&artifact(ArtifactKind::Transcript, path)),
            Err(StoreError::Escape(_))
        ));
    }
    std::fs::rename(&root, temp.path().join("old")).unwrap();
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("new.md"), note(ArtifactKind::Transcript)).unwrap();
    assert!(
        stores
            .check_retention(&artifact(ArtifactKind::Transcript, &root.join("new.md")))
            .is_err()
    );
}

#[test]
fn retention_scan_refuses_more_than_65536_entries_without_changing_files() {
    let (_temp, root, stores) = store();
    for index in 0..65_537 {
        std::fs::File::create(root.join(format!("untracked-{index}"))).unwrap();
    }
    assert!(stores.retention_entries(&root).is_err());
    assert_eq!(std::fs::read_dir(root).unwrap().count(), 65_537);
}
