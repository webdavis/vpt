mod discovery;
mod synthesis;
use super::*;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};

fn fixture() -> (tempfile::TempDir, PathBuf, FilesystemStores) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("notes");
    std::fs::create_dir(&root).unwrap();
    let stores = FilesystemStores::open(std::slice::from_ref(&root)).unwrap();
    (temp, root, stores)
}

#[test]
fn reads_exact_utf8_and_digest_without_changing_the_note_or_creating_files() {
    let (_temp, root, stores) = fixture();
    let path = root.join("note.md");
    let bytes = "---\nstatus: complete\n---\n# café 世界\n";
    std::fs::write(&path, bytes).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640)).unwrap();
    let before = std::fs::metadata(&path).unwrap();
    let note = stores.read_note(&path).unwrap().unwrap();
    assert_eq!(note.text, bytes);
    assert_eq!(note.digest, stores.digest_bytes(bytes.as_bytes()));
    assert!(!format!("{note:?}").contains("café"));
    let after = std::fs::metadata(&path).unwrap();
    assert_eq!(
        (
            before.ino(),
            before.mtime(),
            before.mtime_nsec(),
            before.mode()
        ),
        (after.ino(), after.mtime(), after.mtime_nsec(), after.mode())
    );
    assert_eq!(stores.read_note(&root.join("missing.md")), Ok(None));
    assert_eq!(std::fs::read_dir(root).unwrap().count(), 1);
}

#[test]
fn refuses_symlinks_directories_escape_and_redirected_roots() {
    let (temp, root, stores) = fixture();
    let outside = temp.path().join("outside.md");
    std::fs::write(&outside, "SECRET").unwrap();
    let link = root.join("link.md");
    symlink(&outside, &link).unwrap();
    let directory = root.join("directory.md");
    std::fs::create_dir(&directory).unwrap();
    for path in [&link, &directory, &outside, &root.join("../outside.md")] {
        assert!(stores.read_note(path).is_err());
    }
    let moved = temp.path().join("moved");
    std::fs::rename(&root, &moved).unwrap();
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("new.md"), "SECRET").unwrap();
    assert!(stores.read_note(&root.join("new.md")).is_err());
}

#[test]
fn oversized_and_invalid_utf8_notes_fail_with_safe_errors() {
    let (_temp, root, stores) = fixture();
    let large = root.join("large.md");
    std::fs::File::create(&large)
        .unwrap()
        .set_len(vpt_domain::note::MAX_RENDERED as u64 + 1)
        .unwrap();
    assert!(stores.read_note(&large).is_err());
    let bad = root.join("bad.md");
    std::fs::write(&bad, b"SECRET\xff").unwrap();
    let error = stores.read_note(&bad).unwrap_err();
    assert!(!format!("{error:?}").contains("SECRET"));
}

#[test]
fn replacing_modifying_or_unlinking_the_file_during_read_is_detected() {
    let (_temp, root, stores) = fixture();
    let path = root.join("note.md");
    for action in 0..3 {
        std::fs::write(&path, "before").unwrap();
        let result = stores.read_note_with(&path, || match action {
            0 => {
                let other = root.join("other.md");
                std::fs::write(&other, "after!").unwrap();
                std::fs::rename(other, &path).unwrap();
            }
            1 => std::fs::write(&path, "after!").unwrap(),
            _ => std::fs::remove_file(&path).unwrap(),
        });
        assert!(result.is_err());
    }
}

#[test]
fn a_store_redirected_after_reading_is_refused() {
    let (temp, root, stores) = fixture();
    let path = root.join("note.md");
    std::fs::write(&path, "before").unwrap();
    let result = stores.read_note_with(&path, || {
        std::fs::rename(&root, temp.path().join("moved")).unwrap();
        std::fs::create_dir(&root).unwrap();
        std::fs::write(&path, "after!").unwrap();
    });
    assert!(result.is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "after!");
}

#[test]
fn expected_version_checks_raw_bytes_and_size_before_text_decoding() {
    let (_temp, root, stores) = fixture();
    let path = root.join("release.md");
    let expected = stores.digest_bytes(b"original");
    assert_eq!(stores.read_note_version(&path, expected), Ok(None));
    std::fs::write(&path, b"original").unwrap();
    let found = stores.read_note_version(&path, expected).unwrap().unwrap();
    assert_eq!(found.text, "original");
    assert_eq!(found.digest, expected);
    std::fs::write(&path, b"\xff\xfe").unwrap();
    assert_eq!(
        stores.read_note_version(&path, expected),
        Err(StoreError::Changed(path.clone()))
    );
    std::fs::File::create(&path)
        .unwrap()
        .set_len(16 * 1024 * 1024 + 1)
        .unwrap();
    assert_eq!(
        stores.read_note_version(&path, expected),
        Err(StoreError::Changed(path))
    );
}
