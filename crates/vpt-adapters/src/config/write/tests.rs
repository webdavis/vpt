use super::*;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};

fn mode(path: &Path) -> u32 {
    fs::metadata(path).expect("metadata").permissions().mode() & 0o777
}
fn sandbox() -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().expect("owned sandbox");
    let base = temp.path().canonicalize().expect("canonical");
    fs::set_permissions(&base, fs::Permissions::from_mode(0o700)).expect("private sandbox");
    (temp, base)
}
#[test]
fn preparation_creates_nothing_and_writing_creates_private_paths() {
    let (_temp, base) = sandbox();
    let dirs = [base.join("home/.vpt"), base.join("state/local/vpt")];
    let writer =
        FilesystemSetupWriter::new(base.join("config/vpt/config.toml"), &dirs).expect("prepared");
    assert_eq!(fs::read_dir(&base).expect("entries").count(), 0);
    let path = writer
        .write_config("config_version = 1\n", false)
        .expect("write");
    assert_eq!(
        fs::read_to_string(&path).expect("text"),
        "config_version = 1\n"
    );
    assert_eq!(mode(&path), 0o600);
    assert_eq!(mode(path.parent().expect("parent")), 0o700);
    for dir in dirs {
        writer.create_private_dir(&dir).expect("directory");
        assert_eq!(mode(&dir), 0o700);
    }
}
#[test]
fn exclusive_creation_preserves_a_file_created_after_preparation() {
    let (_temp, base) = sandbox();
    let path = base.join("config.toml");
    let writer = FilesystemSetupWriter::new(path.clone(), &[]).expect("prepared");
    assert!(!writer.config_exists());
    fs::write(&path, "old").expect("late file");
    assert!(writer.config_exists());
    assert_eq!(
        writer.write_config("new", false),
        Err(SetupError::Exists(path.clone()))
    );
    assert_eq!(fs::read_to_string(path).expect("kept"), "old");
}
#[test]
fn force_truncates_and_repairs_config_without_changing_private_directory_metadata() {
    let (_temp, base) = sandbox();
    let path = base.join("config/config.toml");
    let dir = base.join("home");
    fs::create_dir(path.parent().expect("parent")).expect("config parent");
    fs::set_permissions(
        path.parent().expect("parent"),
        fs::Permissions::from_mode(0o700),
    )
    .expect("private config parent");
    fs::create_dir(&dir).expect("home");
    fs::write(&path, "long original").expect("existing");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).expect("file mode");
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).expect("dir mode");
    let before = fs::metadata(&dir).unwrap();
    let writer =
        FilesystemSetupWriter::new(path.clone(), std::slice::from_ref(&dir)).expect("prepared");
    writer.write_config("new", true).expect("forced");
    writer.create_private_dir(&dir).expect("private directory");
    assert_directory_metadata_unchanged(&dir, &before);
    assert_eq!(fs::read_to_string(&path).expect("text"), "new");
    assert_eq!(mode(&path), 0o600);
    assert_eq!(mode(&dir), 0o700);
    assert_eq!(mode(path.parent().expect("parent")), 0o700);
}

#[test]
fn existing_config_parent_must_be_private_without_chmod_or_creation() {
    let (_temp, base) = sandbox();
    let parent = base.join("shared");
    fs::create_dir(&parent).expect("parent");
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o755)).expect("shared mode");
    fs::write(parent.join("keep"), "unchanged").expect("sentinel");
    assert!(FilesystemSetupWriter::new(parent.join("config.toml"), &[]).is_err());
    assert_eq!(mode(&parent), 0o755);
    assert_eq!(
        fs::read_to_string(parent.join("keep")).expect("sentinel"),
        "unchanged"
    );
    assert!(!parent.join("config.toml").exists());
}

#[test]
fn config_parent_mode_change_after_preparation_refuses_without_repair() {
    let (_temp, base) = sandbox();
    let parent = base.join("config");
    fs::create_dir(&parent).expect("parent");
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).expect("private mode");
    let writer = FilesystemSetupWriter::new(parent.join("config.toml"), &[]).expect("prepared");
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o755)).expect("mode changed");
    assert!(writer.write_config("private", false).is_err());
    assert_eq!(mode(&parent), 0o755);
    assert!(!parent.join("config.toml").exists());
}

#[test]
fn config_parent_created_after_preparation_must_already_be_private() {
    let (_temp, base) = sandbox();
    let parent = base.join("new");
    let writer = FilesystemSetupWriter::new(parent.join("config.toml"), &[]).expect("prepared");
    fs::create_dir(&parent).expect("late parent");
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o755)).expect("shared mode");
    assert!(writer.write_config("private", false).is_err());
    assert_eq!(mode(&parent), 0o755);
    assert!(!parent.join("config.toml").exists());
}

#[test]
fn force_refuses_a_hard_link_to_owned_source_without_changing_the_inode() {
    let (_temp, base) = sandbox();
    let source =
        base.join("home/Library/Group Containers/group.com.apple.VoiceMemos.shared/Recordings");
    fs::create_dir_all(&source).expect("synthetic source");
    let recording = source.join("keep");
    fs::write(&recording, "source bytes").expect("recording");
    fs::set_permissions(&recording, fs::Permissions::from_mode(0o644)).expect("source mode");
    let parent = base.join("config");
    fs::create_dir(&parent).expect("config parent");
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).expect("private parent");
    let config = parent.join("config.toml");
    fs::hard_link(&recording, &config).expect("owned hard link");
    let before_bytes = fs::read(&recording).expect("before bytes");
    let before = fs::metadata(&recording).expect("before metadata");
    let writer = FilesystemSetupWriter::new(config, &[]).expect("prepared");
    let result = writer.write_config("replacement", true);
    let after = fs::metadata(&recording).expect("after metadata");
    let after_bytes = fs::read(&recording).expect("after bytes");
    assert!(result.is_err(), "multiply linked config was accepted");
    assert_eq!(after_bytes, before_bytes);
    assert_eq!(after.mode(), before.mode());
    assert_eq!(after.nlink(), before.nlink());
    assert_eq!(
        (after.atime(), after.atime_nsec()),
        (before.atime(), before.atime_nsec())
    );
    assert_eq!(
        (after.mtime(), after.mtime_nsec()),
        (before.mtime(), before.mtime_nsec())
    );
    assert_eq!(
        (after.ctime(), after.ctime_nsec()),
        (before.ctime(), before.ctime_nsec())
    );
}
#[test]
fn force_creates_an_absent_file() {
    let (_temp, base) = sandbox();
    let path = base.join("config.toml");
    let writer = FilesystemSetupWriter::new(path.clone(), &[]).expect("prepared");
    writer.write_config("new", true).expect("forced new");
    assert_eq!(fs::read_to_string(path).expect("text"), "new");
}
#[test]
fn substituted_ancestor_refuses_writes_to_both_directories() {
    let (_temp, base) = sandbox();
    let parent = base.join("parent");
    let outside = base.join("outside");
    fs::create_dir(&parent).expect("parent");
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).expect("private parent");
    fs::create_dir(&outside).expect("outside");
    let dir = parent.join("state");
    let writer = FilesystemSetupWriter::new(parent.join("config.toml"), std::slice::from_ref(&dir))
        .expect("prepared");
    fs::rename(&parent, base.join("held")).expect("move");
    symlink(&outside, &parent).expect("replace");
    assert!(matches!(
        writer.write_config("new", true),
        Err(SetupError::PathEscape(_))
    ));
    assert!(matches!(
        writer.create_private_dir(&dir),
        Err(SetupError::PathEscape(_))
    ));
    assert_eq!(fs::read_dir(outside).expect("outside entries").count(), 0);
    assert_eq!(
        fs::read_dir(base.join("held"))
            .expect("held entries")
            .count(),
        0
    );
}
#[test]
fn force_refuses_a_leaf_link_without_changing_its_target() {
    let (_temp, base) = sandbox();
    let path = base.join("config.toml");
    let target = base.join("target");
    fs::write(&target, "keep").expect("target");
    symlink(&target, &path).expect("link");
    let writer = FilesystemSetupWriter::new(path.clone(), &[]).expect("prepared");
    assert_eq!(
        writer.write_config("new", true),
        Err(SetupError::PathEscape(path))
    );
    assert_eq!(fs::read_to_string(target).expect("target"), "keep");
}
#[test]
fn an_unprepared_directory_is_refused() {
    let (_temp, base) = sandbox();
    let path = base.join("unprepared");
    let writer = FilesystemSetupWriter::new(base.join("config.toml"), &[]).expect("prepared");
    assert_eq!(
        writer.create_private_dir(&path),
        Err(SetupError::PathEscape(path.clone()))
    );
    assert!(!path.exists());
}

#[test]
fn existing_shared_directory_is_refused_during_preparation_without_chmod() {
    let (_temp, base) = sandbox();
    let dir = base.join("shared");
    fs::create_dir(&dir).unwrap();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(dir.join("operator"), b"keep").unwrap();
    let before = fs::metadata(&dir).unwrap();
    let config = base.join("new-config/config.toml");
    let result = FilesystemSetupWriter::new(config.clone(), std::slice::from_ref(&dir));
    assert!(
        result.is_err(),
        "shared directory must be refused before writing configuration"
    );
    assert_directory_metadata_unchanged(&dir, &before);
    assert_eq!(fs::read(dir.join("operator")).unwrap(), b"keep");
    assert!(!config.parent().unwrap().exists());
}

#[test]
fn directory_made_shared_after_preparation_is_refused_without_chmod() {
    for exists_at_preparation in [false, true] {
        let (_temp, base) = sandbox();
        let dir = base.join("store");
        if exists_at_preparation {
            fs::create_dir(&dir).unwrap();
            fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let writer =
            FilesystemSetupWriter::new(base.join("config.toml"), std::slice::from_ref(&dir))
                .unwrap();
        if !exists_at_preparation {
            fs::create_dir(&dir).unwrap();
        }
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
        fs::write(dir.join("operator"), b"keep").unwrap();
        let before = fs::metadata(&dir).unwrap();
        assert!(
            writer.create_private_dir(&dir).is_err(),
            "late shared directory must not be made private"
        );
        assert_directory_metadata_unchanged(&dir, &before);
        assert_eq!(fs::read(dir.join("operator")).unwrap(), b"keep");
    }
}

fn assert_directory_metadata_unchanged(path: &Path, before: &fs::Metadata) {
    let after = fs::metadata(path).unwrap();
    assert_eq!(
        (
            after.dev(),
            after.ino(),
            after.mode(),
            after.ctime(),
            after.ctime_nsec(),
            after.mtime(),
            after.mtime_nsec()
        ),
        (
            before.dev(),
            before.ino(),
            before.mode(),
            before.ctime(),
            before.ctime_nsec(),
            before.mtime(),
            before.mtime_nsec()
        )
    );
}
