use super::*;
use sha2::Digest;
use std::fs::OpenOptions;
use std::io::Write;
use vpt_domain::container::inspect;
use vpt_domain::fixtures::m4a;

#[test]
fn recorder_clone_stages_private_complete_bytes_with_digest_and_size() {
    let bytes = m4a(1_787_604_456, 5, &vec![42; crate::BUFFER + 17]);
    let (_temp, audio, store, handle) = fixture(&bytes);
    let source = audio
        .parent()
        .expect("parent")
        .join("Recordings/source.m4a");
    let source_before = std::fs::metadata(&source).expect("source metadata");
    let archive = ClonefileArchive::open(&audio).expect("archive");
    let staged = archive
        .stage(|directory, identity, name| store.clone_into(&handle, directory, identity, name))
        .expect("stage");
    assert_eq!(staged.path.parent(), Some(audio.as_path()));
    assert!(
        staged
            .path
            .file_name()
            .expect("name")
            .to_string_lossy()
            .starts_with(STAGING_PREFIX)
    );
    assert_eq!(std::fs::read(&staged.path).expect("bytes"), bytes);
    assert_eq!(mode(&staged.path), 0o600);
    assert_eq!(staged.size, bytes.len() as u64);
    assert_eq!(
        staged.digest,
        Sha256Digest(sha2::Sha256::digest(&bytes).into())
    );
    let source_after = std::fs::metadata(&source).expect("source metadata");
    assert_eq!(source_after.len(), source_before.len());
    assert_eq!(
        source_after.modified().expect("mtime"),
        source_before.modified().expect("mtime")
    );
    assert_eq!(
        source_after.permissions().mode(),
        source_before.permissions().mode()
    );
    assert_eq!(std::fs::read(source).expect("source bytes"), bytes);
    let opened = archive.open(&staged.path).expect("open archive");
    assert_eq!(archive.size(&opened).expect("size"), staged.size);
    let container = inspect(staged.size, |offset, buf| {
        archive.read_at(&opened, offset, buf)
    })
    .expect("inspect");
    assert_eq!(container.duration_secs, 5);
}

#[test]
fn stage_reports_each_clone_kind_without_assuming_volume_support() {
    let (_temp, audio, _, _) = fixture(b"source");
    let archive = ClonefileArchive::open(&audio).expect("archive");
    for kind in [CloneKind::CopyOnWrite, CloneKind::ByteCopy] {
        let staged = archive
            .stage(|directory, _, name| {
                std::fs::write(directory.join(name), b"staged").expect("synthetic clone");
                Ok(kind)
            })
            .expect("stage");
        assert_eq!(staged.copy_on_write, kind == CloneKind::CopyOnWrite);
        assert_eq!(staged.size, 6);
        assert_eq!(mode(&staged.path), 0o600);
    }
}

#[test]
fn a_clone_that_created_nothing_owns_nothing() {
    let (_temp, audio, _, _) = fixture(b"source");
    let archive = ClonefileArchive::open(&audio).expect("archive");
    let failure = archive
        .stage(|_, _, _| Err(CloneError::Io("boom".into())))
        .expect_err("fail");
    assert_eq!(failure.cause, ArchiveError::Io("boom".into()));
    assert_eq!(failure.owned_staging, None);
    assert!(archive.staged_leftovers().expect("list").is_empty());
}

#[test]
fn partial_copy_failure_owns_only_its_private_staging_file() {
    let (_temp, audio, _, _) = fixture(b"source");
    let archive = ClonefileArchive::open(&audio).expect("archive");
    let failure = archive
        .stage(|directory, _, name| {
            std::fs::write(directory.join(name), b"partial").expect("partial");
            Err(CloneError::NoSpace)
        })
        .expect_err("fail");
    assert_eq!(failure.cause, ArchiveError::NoSpace);
    let owned = failure.owned_staging.expect("owned");
    assert_eq!(owned.parent(), Some(audio.as_path()));
    assert_eq!(std::fs::read(&owned).expect("bytes"), b"partial");
    assert_eq!(mode(&owned), 0o600);
    assert_eq!(archive.staged_leftovers().expect("list"), vec![owned]);
}

#[test]
fn collision_claims_nothing_and_preserves_existing_bytes_and_mode() {
    let (_temp, audio, _, _) = fixture(b"source");
    let archive = ClonefileArchive::open(&audio).expect("archive");
    let mut occupied = None;
    let failure = archive
        .stage(|directory, _, name| {
            let path = directory.join(name);
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .expect("exclusive fixture");
            file.write_all(b"other owner").expect("bytes");
            file.set_permissions(std::fs::Permissions::from_mode(0o640))
                .expect("mode");
            occupied = Some(path);
            Err(CloneError::Exists)
        })
        .expect_err("collision");
    let occupied = occupied.expect("path");
    assert_eq!(failure.owned_staging, None);
    assert!(matches!(failure.cause, ArchiveError::Io(_)));
    assert_eq!(std::fs::read(&occupied).expect("bytes"), b"other owner");
    assert_eq!(mode(&occupied), 0o640);
}

#[test]
fn failed_finalization_never_claims_a_nonregular_staging_path() {
    let (_temp, audio, _, _) = fixture(b"source");
    let archive = ClonefileArchive::open(&audio).expect("archive");
    let outside = audio
        .parent()
        .expect("parent")
        .join("Recordings/source.m4a");
    let original_mode = mode(&outside);
    let failure = archive
        .stage(|directory, _, name| {
            std::os::unix::fs::symlink(&outside, directory.join(name)).expect("link");
            Ok(CloneKind::CopyOnWrite)
        })
        .expect_err("link refused");
    assert_eq!(failure.owned_staging, None);
    assert!(matches!(failure.cause, ArchiveError::Escape(_)));
    assert_eq!(std::fs::read(&outside).expect("outside"), b"source");
    assert_eq!(mode(&outside), original_mode);
}
