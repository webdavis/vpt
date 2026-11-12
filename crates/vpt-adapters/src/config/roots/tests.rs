use super::*;
use crate::config::{from_table, load_text};
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use vpt_domain::layout::StoreKey;

mod boundaries;
mod vault;
mod vocabulary;

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
fn creation_makes_every_missing_folder_of_a_root_private() {
    let temp = fixture();
    let mut settings = settings_in(temp.path());
    settings.stores.audio = temp.path().join("missing/deeper/audio");
    settings.state_dir = temp.path().join("local/state/vpt");
    let roots = resolve(&settings, &temp.path().join("cfg")).expect("resolve");
    assert!(!temp.path().join("missing").exists());
    roots.create_leaves().expect("leaves");
    roots.create_state_dir().expect("state");
    for leaf in [
        "missing",
        "missing/deeper",
        "missing/deeper/audio",
        "local",
        "local/state",
        "local/state/vpt",
    ] {
        let path = temp.path().join(leaf);
        assert_eq!(
            fs::metadata(&path).expect("created").permissions().mode() & 0o777,
            0o700,
            "{leaf}"
        );
    }
    assert_eq!(entries(&temp.path().join("missing/deeper")), vec!["audio"]);
}

#[test]
fn creation_refuses_a_symlink_or_file_where_a_missing_folder_was_planned() {
    let temp = fixture();
    let mut settings = settings_in(temp.path());
    settings.stores.audio = temp.path().join("planned/deeper/audio");
    let roots = resolve(&settings, &temp.path().join("cfg")).expect("resolve");
    symlink(temp.path().join("voice-memos"), temp.path().join("planned")).expect("link");
    assert_eq!(
        roots.create_leaves(),
        Err(RootError::PathEscape {
            key: "stores.audio".into()
        })
    );
    fs::remove_file(temp.path().join("planned")).expect("unlink");
    fs::write(temp.path().join("planned"), b"file").expect("file");
    assert!(roots.create_leaves().is_err());
    assert_eq!(
        entries(&temp.path().join("voice-memos")),
        vec!["CloudRecordings.db", "Recordings"]
    );
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

#[test]
fn creating_roots_refuses_a_link_added_after_resolution() {
    let temp = fixture();
    let roots = resolve(&settings_in(temp.path()), &temp.path().join("cfg")).expect("roots");
    let before = entries(&roots.container);
    symlink(&roots.container, &roots.state_dir).expect("state link");
    assert_eq!(
        roots.create_state_dir(),
        Err(RootError::PathEscape {
            key: "home.state_dir".into()
        })
    );
    fs::create_dir(&roots.home).expect("home");
    fs::rename(&roots.home, temp.path().join("original-home")).expect("move home");
    symlink(&roots.container, &roots.home).expect("home link");
    assert_eq!(
        roots.create_leaves(),
        Err(RootError::PathEscape {
            key: "home.path".into()
        })
    );
    assert_eq!(entries(&roots.container), before);
}

#[test]
fn creating_stores_refuses_a_link_in_the_parent_or_leaf() {
    let temp = fixture();
    let mut settings = settings_in(temp.path());
    fs::create_dir(temp.path().join("external")).expect("external");
    settings.stores.audio = temp.path().join("external/audio");
    let roots = resolve(&settings, &temp.path().join("cfg")).expect("roots");
    let before = entries(&roots.container);
    fs::rename(temp.path().join("external"), temp.path().join("held")).expect("move");
    symlink(&roots.container, temp.path().join("external")).expect("parent link");
    assert_eq!(
        roots.create_leaves(),
        Err(RootError::PathEscape {
            key: "stores.audio".into()
        })
    );
    assert_eq!(entries(&roots.container), before);

    let temp = fixture();
    let roots = resolve(&settings_in(temp.path()), &temp.path().join("cfg")).expect("roots");
    fs::create_dir(&roots.home).expect("home");
    symlink(&roots.container, &roots.stores.audio).expect("leaf link");
    assert_eq!(
        roots.create_leaves(),
        Err(RootError::PathEscape {
            key: "stores.audio".into()
        })
    );
    assert!(!roots.stores.released.exists());
}
#[test]
fn known_tags_refuse_protected_roots_and_overlapping_confirmation_lists() {
    let temp = fixture();
    let mut settings = settings_in(temp.path());
    let paths = [
        settings.state_dir.join("tags"),
        settings.source.recordings_dir.join("tags"),
        settings.stores.released.join("tags"),
        settings.reconcile.known_terms_path.clone(),
        settings.reconcile.known_terms_path.join("tags"),
    ];
    for path in paths {
        settings.tags.known_tags_path = path;
        assert!(
            matches!(resolve(&settings, &temp.path().join("cfg")), Err(RootError::Overlap { first, .. }) if first == "tags.known_tags_path")
        );
    }
    settings.tags.known_tags_path = "relative-tags".into();
    assert_eq!(
        resolve(&settings, &temp.path().join("cfg")),
        Err(RootError::NotAbsolute {
            key: "tags.known_tags_path".into()
        })
    );
}
