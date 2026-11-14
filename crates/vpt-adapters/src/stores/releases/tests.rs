use super::*;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use vpt_application::StorePaths;

fn fixture() -> (tempfile::TempDir, Roots) {
    let temp = crate::test_temp::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let roots = Roots {
        home: base.join("home"),
        state_dir: base.join("state"),
        recordings_dir: base.join("memos/Recordings"),
        container: base.join("memos"),
        config_dir: base.join("config"),
        stores: StorePaths {
            audio: base.join("home/audio"),
            transcripts: base.join("home/transcripts"),
            analysis: base.join("home/analysis"),
            briefs: base.join("home/briefs"),
            engine_outputs: base.join("home/engine-outputs"),
            drafts: base.join("home/drafts"),
            released: base.join("released"),
        },
    };
    fs::create_dir_all(&roots.stores.drafts).unwrap();
    fs::set_permissions(&roots.stores.drafts, fs::Permissions::from_mode(0o700)).unwrap();
    (temp, roots)
}
fn select(files: &ReleaseFiles, bytes: &[u8]) -> Result<ReleaseTarget, ReleaseFileError> {
    files.select(
        &RecordingId::parse("2026-08-24T144736-4f3ab19c02de").unwrap(),
        NoteStage::Transcript,
        bytes,
        UtcInstant { secs: 0 },
        UtcOffset { secs: -3600 },
    )
}
#[test]
fn release_destination_protects_all_private_roots_and_resolved_aliases() {
    let (temp, roots) = fixture();
    let mut protected = vpt_domain::layout::StoreKey::all()
        .into_iter()
        .filter(|key| key.is_private())
        .map(|key| (roots.stores.get(key), key.key_name()))
        .collect::<Vec<_>>();
    protected.extend([
        (roots.state_dir.as_path(), "state"),
        (roots.config_dir.as_path(), "config"),
        (roots.container.as_path(), "voice_memos"),
    ]);
    for (path, name) in protected {
        fs::create_dir_all(path).unwrap();
        for destination in [path.to_path_buf(), path.join("new/subfolder")] {
            assert!(
                matches!(ReleaseFiles::open(&roots, &destination),
                Err(ReleaseFileError::Protected(found)) if found == name),
                "{destination:?}"
            );
        }
        assert!(!path.join("new").exists());
    }
    let alias = temp.path().join("alias");
    symlink(&roots.stores.transcripts, &alias).unwrap();
    assert!(matches!(
        ReleaseFiles::open(&roots, &alias.join("nested")),
        Err(ReleaseFileError::Protected("transcripts"))
    ));
}
#[test]
fn release_destination_preserves_shared_permissions_and_allows_private_root_ancestor() {
    let (_temp, roots) = fixture();
    fs::set_permissions(&roots.home, fs::Permissions::from_mode(0o755)).unwrap();
    ReleaseFiles::open(&roots, &roots.home).unwrap();
    assert_eq!(
        fs::metadata(&roots.home).unwrap().permissions().mode() & 0o777,
        0o755
    );
    let new = roots.stores.released.join("nested");
    let files = ReleaseFiles::open(&roots, &new).unwrap();
    assert_eq!(
        select(&files, b"public").unwrap(),
        ReleaseTarget {
            path: new.join("2026-08-24-transcript-4f3ab19c.md"),
            collision: false
        }
    );
    assert_eq!(fs::read_dir(&new).unwrap().count(), 0);
}
#[test]
fn release_names_use_local_time_and_smallest_unused_suffix_without_overwriting() {
    let (_temp, roots) = fixture();
    let files = ReleaseFiles::open(&roots, &roots.stores.released).unwrap();
    let base = select(&files, b"first").unwrap().path;
    fs::write(&base, b"first").unwrap();
    let stamped = roots
        .stores
        .released
        .join("2026-08-24-transcript-4f3ab19c-19691231T230000.md");
    assert_eq!(
        select(&files, b"next").unwrap(),
        ReleaseTarget {
            path: stamped.clone(),
            collision: true
        }
    );
    fs::create_dir(&stamped).unwrap();
    let second = stamped.with_file_name("2026-08-24-transcript-4f3ab19c-19691231T230000-2.md");
    let third = stamped.with_file_name("2026-08-24-transcript-4f3ab19c-19691231T230000-3.md");
    symlink("missing", &second).unwrap();
    assert_eq!(select(&files, b"next").unwrap().path, third);
    fs::remove_file(&second).unwrap();
    assert_eq!(select(&files, b"next").unwrap().path, second);
    assert_eq!(fs::read(&base).unwrap(), b"first");
}
#[test]
fn release_duplicates_include_every_direct_regular_filename_but_not_nested_or_linked_files() {
    let (temp, roots) = fixture();
    let files = ReleaseFiles::open(&roots, &roots.stores.released).unwrap();
    let outside = temp.path().join("outside");
    fs::write(&outside, b"public").unwrap();
    symlink(&outside, roots.stores.released.join("linked")).unwrap();
    fs::create_dir(roots.stores.released.join("nested")).unwrap();
    fs::write(roots.stores.released.join("nested/file"), b"public").unwrap();
    assert!(select(&files, b"public").is_ok());
    let duplicate = roots.stores.released.join("unrelated.bin");
    fs::write(&duplicate, b"public").unwrap();
    assert_eq!(
        select(&files, b"public"),
        Err(ReleaseFileError::Duplicate(duplicate))
    );
}
#[test]
fn release_preserves_existing_report_directory_permissions() {
    let (_temp, roots) = fixture();
    fs::set_permissions(&roots.stores.drafts, fs::Permissions::from_mode(0o755)).unwrap();
    ReleaseFiles::open(&roots, &roots.stores.released).unwrap();
    assert_eq!(
        fs::metadata(&roots.stores.drafts)
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o755
    );
}

mod publication;
