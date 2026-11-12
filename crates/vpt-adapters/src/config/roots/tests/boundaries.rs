use super::*;
use std::os::macos::fs::MetadataExt as _;

fn source_snapshot(root: &Path) -> Vec<(PathBuf, u64, i64, i64, u32, u32)> {
    [
        "voice-memos",
        "voice-memos/Recordings",
        "voice-memos/CloudRecordings.db",
    ]
    .into_iter()
    .map(|leaf| {
        let path = root.join(leaf);
        let meta = fs::symlink_metadata(&path).expect("source entry");
        (
            path,
            meta.len(),
            meta.mtime(),
            meta.mtime_nsec(),
            meta.mode(),
            meta.st_flags(),
        )
    })
    .collect()
}

#[test]
fn source_overlaps_are_refused_before_any_creation() {
    let temp = fixture();
    fs::create_dir(temp.path().join("h")).expect("store parent");
    let before = source_snapshot(temp.path());
    let container = temp.path().join("voice-memos");
    for path in [container.clone(), container.join("new")] {
        let mut settings = settings_in(temp.path());
        settings.home = path.clone();
        assert_eq!(
            resolve(&settings, &temp.path().join("cfg")),
            Err(RootError::Overlap {
                first: "home.path".into(),
                second: "source.recordings_dir".into()
            })
        );
        assert_eq!(
            resolve(&settings_in(temp.path()), &path),
            Err(RootError::Overlap {
                first: "config".into(),
                second: "source.recordings_dir".into()
            })
        );
    }
    let mut settings = settings_in(temp.path());
    settings.stores.drafts = container.join("drafts");
    assert_eq!(
        resolve(&settings, &temp.path().join("cfg")),
        Err(RootError::Overlap {
            first: "source.recordings_dir".into(),
            second: "stores.drafts".into()
        })
    );
    settings = settings_in(temp.path());
    settings.state_dir = container.join("state");
    assert_eq!(
        resolve(&settings, &temp.path().join("cfg")),
        Err(RootError::Overlap {
            first: "home.state_dir".into(),
            second: "source.recordings_dir".into()
        })
    );
    assert_eq!(source_snapshot(temp.path()), before);
    assert_eq!(
        entries(&container),
        vec!["CloudRecordings.db", "Recordings"]
    );
    assert_eq!(
        fs::read(container.join("CloudRecordings.db")).expect("database"),
        b"live"
    );
    assert!(entries(&temp.path().join("h")).is_empty());
    assert!(!temp.path().join("state").exists());
}

#[test]
fn home_and_config_ancestors_of_source_remain_valid() {
    let temp = fixture();
    fs::create_dir(temp.path().join("h")).expect("store parent");
    let mut settings = settings_in(temp.path());
    settings.home = temp.path().to_path_buf();
    let roots = resolve(&settings, &temp.path().join("cfg")).expect("home ancestor");
    assert_eq!(roots.home, temp.path().canonicalize().expect("canonical"));
    settings.source.recordings_dir = temp.path().join("shared/source/Recordings");
    let roots = resolve(&settings, &temp.path().join("shared")).expect("config ancestor");
    assert_eq!(roots.config_dir, roots.home.join("shared"));
}

#[test]
fn canonical_alias_into_source_is_refused() {
    let temp = fixture();
    symlink(temp.path().join("voice-memos"), temp.path().join("alias")).expect("alias");
    assert_eq!(
        resolve(&settings_in(temp.path()), &temp.path().join("alias/new")),
        Err(RootError::Overlap {
            first: "config".into(),
            second: "source.recordings_dir".into()
        })
    );
    assert!(!temp.path().join("voice-memos/new").exists());
}

#[test]
fn released_cannot_overlap_the_configuration_directory() {
    let temp = fixture();
    let mut settings = settings_in(temp.path());
    settings.stores.released = temp.path().join("cfg/out");
    assert_eq!(
        resolve(&settings, &temp.path().join("cfg")),
        Err(RootError::Overlap {
            first: "config".into(),
            second: "stores.released".into()
        })
    );
    assert!(!settings.home.exists());
}

#[test]
fn relative_roots_and_regular_files_are_refused() {
    let temp = fixture();
    let mut settings = settings_in(temp.path());
    settings.stores.briefs = "briefs".into();
    assert_eq!(
        resolve(&settings, &temp.path().join("cfg")),
        Err(RootError::NotAbsolute {
            key: "stores.briefs".into()
        })
    );
    assert_eq!(
        planned_directory(Path::new("relative"), "home.path"),
        Err(RootError::NotAbsolute {
            key: "home.path".into()
        })
    );
    settings.stores.briefs = temp.path().join("file");
    fs::write(&settings.stores.briefs, b"file").expect("file");
    assert_eq!(
        resolve(&settings, &temp.path().join("cfg")),
        Err(RootError::NotADirectory {
            key: "stores.briefs".into()
        })
    );
    assert_eq!(
        planned_directory(&settings.stores.briefs, "config"),
        Err(RootError::NotADirectory {
            key: "config".into()
        })
    );
}

#[test]
fn parent_traversal_below_a_missing_folder_is_refused() {
    let temp = fixture();
    let before = source_snapshot(temp.path());
    let mut settings = settings_in(temp.path());
    settings.stores.audio = temp.path().join("h/../voice-memos/evil");
    assert_eq!(
        resolve(&settings, &temp.path().join("cfg")),
        Err(RootError::UnresolvedParent {
            key: "stores.audio".into()
        })
    );
    assert_eq!(source_snapshot(temp.path()), before);
    assert!(!temp.path().join("h").exists());
}

#[test]
fn deep_store_below_a_missing_home_is_planned_without_writes() {
    let temp = fixture();
    let mut settings = settings_in(temp.path());
    settings.stores.audio = temp.path().join("h/deep/audio");
    let roots = resolve(&settings, &temp.path().join("cfg")).expect("resolve");
    let canonical = temp.path().canonicalize().expect("canonical");
    assert_eq!(roots.stores.audio, canonical.join("h/deep/audio"));
    assert!(!temp.path().join("h").exists());
}

#[test]
fn dangling_directory_links_are_refused() {
    let temp = fixture();
    let selected = temp.path().join("config-link");
    symlink(temp.path().join("absent"), &selected).expect("link");
    assert_eq!(
        resolve(&settings_in(temp.path()), &selected),
        Err(RootError::PathEscape {
            key: "config".into()
        })
    );
    let mut settings = settings_in(temp.path());
    settings.stores.audio = selected;
    assert_eq!(
        resolve(&settings, &temp.path().join("cfg")),
        Err(RootError::PathEscape {
            key: "stores.audio".into()
        })
    );
    assert_eq!(
        planned_directory(&temp.path().join("config-link/deeper"), "config"),
        Err(RootError::PathEscape {
            key: "config".into()
        })
    );
}
