use super::*;
use crate::config::{from_table, load_text};
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use vpt_domain::layout::StoreKey;

mod boundaries;

fn fixture() -> tempfile::TempDir {
    let temp = tempfile::tempdir().expect("owned fixture");
    fs::create_dir_all(temp.path().join("voice-memos/Recordings")).expect("source");
    fs::write(temp.path().join("voice-memos/CloudRecordings.db"), b"live").expect("database");
    fs::create_dir(temp.path().join("cfg")).expect("config");
    temp
}

fn settings_in(root: &Path) -> Settings {
    let mut settings =
        from_table(&load_text("config_version = 1").expect("table"), root).expect("settings");
    settings.home = root.join("h");
    settings.state_dir = root.join("state");
    settings.source.recordings_dir = root.join("voice-memos/Recordings");
    for key in StoreKey::all() {
        settings
            .stores
            .set(key, settings.home.join(key.default_leaf()));
    }
    settings
}

fn entries(path: &Path) -> Vec<std::ffi::OsString> {
    let mut entries = fs::read_dir(path)
        .expect("entries")
        .map(|entry| entry.expect("entry").file_name())
        .collect::<Vec<_>>();
    entries.sort();
    entries
}

#[test]
fn resolution_writes_nothing_and_creation_makes_private_leaves() {
    let temp = fixture();
    let before = entries(temp.path());
    let roots = resolve(&settings_in(temp.path()), &temp.path().join("cfg")).expect("resolve");
    assert_eq!(entries(temp.path()), before);
    assert!(!roots.home.exists());
    assert!(!roots.state_dir.exists());
    roots.create_leaves().expect("leaves");
    assert!(!roots.state_dir.exists());
    roots.create_state_dir().expect("state");
    for path in std::iter::once(roots.home.as_path())
        .chain(std::iter::once(roots.state_dir.as_path()))
        .chain(StoreKey::all().iter().map(|key| roots.stores.get(*key)))
    {
        assert!(path.is_dir(), "{path:?}");
        assert_eq!(
            fs::metadata(path).expect("metadata").permissions().mode() & 0o777,
            0o700
        );
    }
    roots.create_leaves().expect("idempotent leaves");
    roots.create_state_dir().expect("idempotent state");
}

#[test]
fn external_store_requires_an_existing_parent_and_creates_only_its_leaf() {
    let temp = fixture();
    let mut settings = settings_in(temp.path());
    settings.stores.audio = temp.path().join("missing/audio");
    assert_eq!(
        resolve(&settings, &temp.path().join("cfg")),
        Err(RootError::ParentMissing {
            key: "stores.audio".into()
        })
    );
    assert!(!settings.home.exists());
    fs::create_dir(temp.path().join("missing")).expect("parent");
    let roots = resolve(&settings, &temp.path().join("cfg")).expect("resolve");
    roots.create_leaves().expect("leaves");
    assert_eq!(entries(&temp.path().join("missing")), vec!["audio"]);
}

#[test]
fn symlinked_home_resolves_stores_against_its_real_directory() {
    let temp = fixture();
    fs::create_dir(temp.path().join("real")).expect("real home");
    symlink(temp.path().join("real"), temp.path().join("h")).expect("home link");
    let settings = settings_in(temp.path());
    let roots = resolve(&settings, &temp.path().join("cfg")).expect("resolve");
    let real = temp.path().join("real").canonicalize().expect("canonical");
    assert_eq!(roots.home, real);
    assert_eq!(roots.stores.audio, real.join("audio"));
    assert_eq!(resolve_stores(&settings).expect("stores"), roots.stores);
    roots.create_leaves().expect("leaves");
    assert!(
        fs::symlink_metadata(temp.path().join("h"))
            .expect("link")
            .is_symlink()
    );
}

#[test]
fn absent_source_and_deep_default_paths_are_planned_without_writes() {
    let temp = fixture();
    let mut settings = settings_in(temp.path());
    settings.state_dir = temp.path().join("new/local/state/vpt");
    settings.source.recordings_dir = temp.path().join("absent-source/Recordings");
    let before = entries(temp.path());
    let roots = resolve(&settings, &temp.path().join("new/config/vpt")).expect("resolve");
    let base = temp.path().canonicalize().expect("canonical");
    assert_eq!(roots.state_dir, base.join("new/local/state/vpt"));
    assert_eq!(roots.config_dir, base.join("new/config/vpt"));
    assert_eq!(roots.container, base.join("absent-source"));
    assert_eq!(roots.recordings_dir, base.join("absent-source/Recordings"));
    assert_eq!(entries(temp.path()), before);
}

#[test]
fn stores_only_resolution_does_not_require_source_or_state() {
    let temp = fixture();
    let mut settings = settings_in(temp.path());
    settings.source.recordings_dir = "relative-source".into();
    settings.state_dir = "relative-state".into();
    let stores = resolve_stores(&settings).expect("stores");
    assert_eq!(
        stores.drafts,
        temp.path()
            .canonicalize()
            .expect("canonical")
            .join("h/drafts")
    );
    settings.stores.drafts = settings.stores.audio.clone();
    assert_eq!(
        resolve_stores(&settings),
        Err(RootError::Overlap {
            first: "stores.audio".into(),
            second: "stores.drafts".into()
        })
    );
    assert!(!settings.home.exists());
}

#[test]
fn planned_directory_resolves_an_existing_symlink_and_missing_suffix() {
    let temp = fixture();
    symlink(temp.path().join("cfg"), temp.path().join("alias")).expect("alias");
    let target = temp.path().join("cfg").canonicalize().expect("canonical");
    assert_eq!(
        planned_directory(&temp.path().join("alias/one/two"), "config").expect("plan"),
        target.join("one/two")
    );
    assert!(entries(&target).is_empty());
}
