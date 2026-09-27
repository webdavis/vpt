use super::fixture::{Fixture, titled, unchanged};
use super::*;
use crate::VoiceMemosStore;
use crate::database::snapshot;
use std::fs;
use std::os::unix::fs::{FileExt, PermissionsExt, symlink};
use vpt_application::ports::{RecorderError, RecorderStore};

pub(super) fn missing_state(root: &RootDir) {
    let fixture = Fixture::simple(root);
    let before = snapshot(root.path());
    let store = VoiceMemosStore::open(&fixture.container.join("Recordings"))
        .unwrap()
        .with_titles(fixture.state.path().join("absent"), true);
    assert_eq!(store.refresh_titles(), Ok(()));
    assert_eq!(store.title("a.m4a"), TitleLookup::Unavailable);
    unchanged(&before, &snapshot(root.path()));
}

pub(super) fn state_replaced(root: &RootDir) {
    let fixture = Fixture::simple(root);
    let store = VoiceMemosStore::open(&fixture.container.join("Recordings"))
        .unwrap()
        .with_titles_root(&fixture.state, true);
    fs::rename(fixture.state.path(), root.path().join("saved")).unwrap();
    fs::create_dir(fixture.state.path()).unwrap();
    let before = snapshot(root.path());
    assert_eq!(
        store.refresh_titles(),
        Err(RecorderError::Escape(fixture.state.path().into()))
    );
    unchanged(&before, &snapshot(root.path()));
}

pub(super) fn leaves(root: &RootDir) {
    for (index, name) in SIDE_FILES.iter().enumerate() {
        let refreshed = root.subdirectory(&format!("refreshed{index}")).unwrap();
        refreshed_leaf(&refreshed, name);
        let observed = root.subdirectory(&format!("observed{index}")).unwrap();
        observed_leaf(&observed, name);
    }
}

fn replace_leaf(fixture: &Fixture, root: &RootDir, name: &str) {
    let outside = root.path().join("outside");
    fs::write(&outside, b"owned outside canary").unwrap();
    fs::rename(
        fixture.copy_path().join(name),
        fixture.copy_path().join(format!("{name}.saved")),
    )
    .unwrap();
    symlink(&outside, fixture.copy_path().join(name)).unwrap();
}

fn refreshed_leaf(root: &RootDir, name: &str) {
    let fixture = Fixture::simple(root);
    let store = VoiceMemosStore::open(&fixture.container.join("Recordings"))
        .unwrap()
        .with_titles_root(&fixture.state, true);
    store.refresh_titles().unwrap();
    assert_eq!(store.title("a.m4a"), TitleLookup::Titled("A".into()));
    replace_leaf(&fixture, root, name);
    let before = snapshot(root.path());
    assert_eq!(store.title("a.m4a"), TitleLookup::Unavailable);
    unchanged(&before, &snapshot(root.path()));
}

fn observed_leaf(root: &RootDir, name: &str) {
    let fixture = Fixture::simple(root);
    titled(&fixture.refresh(), "a.m4a", "A");
    let copy = TitleCopy::existing(fixture.state.path()).unwrap();
    titled(&copy, "a.m4a", "A");
    let store = VoiceMemosStore::open(&fixture.container.join("Recordings"))
        .unwrap()
        .with_titles_root(&fixture.state, true);
    replace_leaf(&fixture, root, name);
    let before = snapshot(root.path());
    assert_eq!(copy.title("a.m4a"), TitleLookup::Unavailable);
    drop(copy);
    unchanged(&before, &snapshot(root.path()));
    assert_eq!(
        store.refresh_titles(),
        Err(RecorderError::Escape(fixture.state.path().into()))
    );
    unchanged(&before, &snapshot(root.path()));
}

pub(super) fn copy_replaced(root: &RootDir) {
    for observation in [false, true] {
        let root = root
            .subdirectory(if observation { "observed" } else { "refreshed" })
            .unwrap();
        let fixture = Fixture::simple(&root);
        let copy = if observation {
            titled(&fixture.refresh(), "a.m4a", "A");
            TitleCopy::existing(fixture.state.path()).unwrap()
        } else {
            fixture.refresh()
        };
        titled(&copy, "a.m4a", "A");
        fs::rename(fixture.copy_path(), fixture.state.path().join("saved-copy")).unwrap();
        fs::create_dir(fixture.copy_path()).unwrap();
        let before = snapshot(root.path());
        assert_eq!(copy.title("a.m4a"), TitleLookup::Unavailable);
        unchanged(&before, &snapshot(root.path()));
        drop(copy);
        if observation {
            unchanged(&before, &snapshot(root.path()));
        }
    }
}

pub(super) fn copy_failure(root: &RootDir) {
    let fixture = Fixture::simple(root);
    let store = VoiceMemosStore::open(&fixture.container.join("Recordings"))
        .unwrap()
        .with_titles_root(&fixture.state, true);
    store.refresh_titles().unwrap();
    assert_eq!(store.title("a.m4a"), TitleLookup::Titled("A".into()));
    let path = fixture.container.join(DATABASE);
    let held = fs::File::open(&path).unwrap();
    let expected = fs::read(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o0)).unwrap();
    assert_eq!(store.refresh_titles(), Ok(()));
    assert_eq!(store.title("a.m4a"), TitleLookup::Unavailable);
    let mut actual = vec![0; expected.len()];
    held.read_exact_at(&mut actual, 0).unwrap();
    assert!(actual == expected, "source bytes changed");
}

pub(super) fn dry(root: &RootDir) {
    let fixture = Fixture::simple(root);
    titled(&fixture.refresh(), "a.m4a", "A");
    let copy = fixture.state.open_subdirectory(COPY_DIRECTORY).unwrap();
    let _exclusive =
        super::super::lease::CopyLease::refresh(&copy, Duration::from_millis(20)).unwrap();
    let before = snapshot(root.path());
    for store in [
        VoiceMemosStore::open(&fixture.container.join("Recordings"))
            .unwrap()
            .with_titles(fixture.state.path().into(), false),
        VoiceMemosStore::open(&fixture.container.join("Recordings"))
            .unwrap()
            .with_titles_root(&fixture.state, false),
    ] {
        store.refresh_titles().unwrap();
        assert_eq!(store.title("a.m4a"), TitleLookup::Unavailable);
    }
    unchanged(&before, &snapshot(root.path()));
}

pub(super) fn hardlinks(root: &RootDir) {
    for (index, name) in SIDE_FILES.iter().enumerate() {
        let child = root.subdirectory(&format!("fixture{index}")).unwrap();
        let fixture = Fixture::simple(&child);
        titled(&fixture.refresh(), "a.m4a", "A");
        fs::hard_link(fixture.copy_path().join(name), child.path().join("alias")).unwrap();
        let before = snapshot(child.path());
        assert!(matches!(
            TitleCopy::refresh(&fixture.container, fixture.state.path()),
            Err(ContainedError::Escape { .. })
        ));
        assert!(matches!(
            TitleCopy::existing(fixture.state.path()),
            Err(ContainedError::Escape { .. })
        ));
        unchanged(&before, &snapshot(child.path()));
    }
}

pub(super) fn convenience(root: &RootDir) {
    let fixture = Fixture::simple(root);
    let absent = fixture.state.path().join("absent");
    let before = snapshot(root.path());
    assert_eq!(
        TitleCopy::existing(&absent).unwrap().title("a.m4a"),
        TitleLookup::Unavailable
    );
    assert_eq!(
        TitleCopy::refresh(&fixture.container, &absent)
            .unwrap()
            .title("a.m4a"),
        TitleLookup::Unavailable
    );
    unchanged(&before, &snapshot(root.path()));
    let linked = root.path().join("linked-state");
    symlink(fixture.state.path(), &linked).unwrap();
    let before = snapshot(root.path());
    assert!(matches!(
        TitleCopy::refresh(&fixture.container, &linked),
        Err(ContainedError::NotADirectory(_))
    ));
    assert!(matches!(
        TitleCopy::existing(&linked),
        Err(ContainedError::NotADirectory(_))
    ));
    unchanged(&before, &snapshot(root.path()));
}
