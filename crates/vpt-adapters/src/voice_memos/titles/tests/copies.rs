use super::fixture::{Fixture, titled, unchanged};
use super::*;
use crate::VoiceMemosStore;
use crate::database::snapshot;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use vpt_application::ports::RecorderStore;

pub(super) fn modes(root: &RootDir) {
    let fixture = Fixture::simple(root);
    let copy = fixture.refresh();
    titled(&copy, "a.m4a", "A");
    assert_eq!(
        fs::metadata(fixture.copy_path()).unwrap().mode() & 0o777,
        0o700
    );
    for name in SIDE_FILES {
        assert_eq!(
            fs::metadata(fixture.copy_path().join(name)).unwrap().mode() & 0o777,
            0o600
        );
    }
}

pub(super) fn refresh_twice(root: &RootDir) {
    let fixture = Fixture::simple(root);
    titled(&fixture.refresh(), "a.m4a", "A");
    let main = fixture.copy_path().join(DATABASE);
    let identity = fs::metadata(&main).unwrap().ino();
    fixture
        .source
        .execute("UPDATE ZCLOUDRECORDING SET ZCUSTOMLABEL='B'", [])
        .unwrap();
    fs::write(fixture.copy_path().join("stray"), b"kept").unwrap();
    fs::set_permissions(&main, fs::Permissions::from_mode(0o644)).unwrap();
    titled(&fixture.refresh(), "a.m4a", "B");
    assert_eq!(fs::metadata(&main).unwrap().ino(), identity);
    assert_eq!(fs::metadata(&main).unwrap().mode() & 0o777, 0o600);
    assert_eq!(
        fs::read(fixture.copy_path().join("stray")).unwrap(),
        b"kept"
    );
}

pub(super) fn existing(root: &RootDir) {
    let fixture = Fixture::simple(root);
    titled(&fixture.refresh(), "a.m4a", "A");
    fixture
        .source
        .execute("UPDATE ZCLOUDRECORDING SET ZCUSTOMLABEL='B'", [])
        .unwrap();
    titled(
        &TitleCopy::existing(fixture.state.path()).unwrap(),
        "a.m4a",
        "A",
    );
    let fresh = root.subdirectory("fresh").unwrap();
    let before = snapshot(fresh.path());
    assert_eq!(
        TitleCopy::existing(fresh.path()).unwrap().title("a.m4a"),
        TitleLookup::Unavailable
    );
    unchanged(&before, &snapshot(fresh.path()));
}

pub(super) fn missing(root: &RootDir) {
    let source = root.subdirectory("source").unwrap();
    let state = root.subdirectory("private").unwrap();
    let before = snapshot(root.path());
    let copy = TitleCopy::refresh(source.path(), state.path()).unwrap();
    assert_eq!(copy.title("a.m4a"), TitleLookup::Unavailable);
    unchanged(&before, &snapshot(root.path()));
}

pub(super) fn source_unchanged(root: &RootDir) {
    let fixture = Fixture::simple(root);
    fixture.unchanged_source(|| titled(&fixture.refresh(), "a.m4a", "A"));
}

pub(super) fn store(root: &RootDir) {
    let fixture = Fixture::simple(root);
    let recordings = fixture.container.join("Recordings");
    let plain = VoiceMemosStore::open(&recordings).unwrap();
    assert_eq!(plain.title("a.m4a"), TitleLookup::Unavailable);
    let store = plain.with_titles(fixture.state.path().into(), true);
    fixture.unchanged_source(|| {
        store.refresh_titles().unwrap();
        assert_eq!(store.title("a.m4a"), TitleLookup::Titled("A".into()));
    });
    fixture
        .source
        .execute("UPDATE ZCLOUDRECORDING SET ZCUSTOMLABEL='B'", [])
        .unwrap();
    fixture.unchanged_source(|| {
        store.refresh_titles().unwrap();
        assert_eq!(store.title("a.m4a"), TitleLookup::Titled("B".into()));
    });
}

pub(super) fn companions(root: &RootDir) {
    let fixture = Fixture::simple(root);
    titled(&fixture.refresh(), "a.m4a", "A");
    let Fixture {
        container,
        state,
        source,
    } = fixture;
    source.close().unwrap();
    assert!(!container.join("CloudRecordings.db-wal").exists());
    assert!(!container.join("CloudRecordings.db-shm").exists());
    let copy = TitleCopy::refresh(&container, state.path()).unwrap();
    titled(&copy, "a.m4a", "A");
    assert_eq!(
        fs::metadata(
            state
                .path()
                .join(COPY_DIRECTORY)
                .join("CloudRecordings.db-wal")
        )
        .unwrap()
        .len(),
        0
    );
}

pub(super) fn refreshed_bytes(root: &RootDir) {
    let fixture = Fixture::simple(root);
    let source_main = fs::read(fixture.container.join(DATABASE)).unwrap();
    let source_wal = fs::read(fixture.container.join("CloudRecordings.db-wal")).unwrap();
    let copy = fixture.refresh();
    for _ in 0..2 {
        titled(&copy, "a.m4a", "A");
    }
    drop(copy);
    assert!(
        fs::read(fixture.copy_path().join(DATABASE)).unwrap() == source_main,
        "private main bytes changed"
    );
    assert!(
        fs::read(fixture.copy_path().join("CloudRecordings.db-wal")).unwrap() == source_wal,
        "private WAL bytes changed"
    );
}

pub(super) fn existing_snapshot(root: &RootDir) {
    let fixture = Fixture::simple(root);
    titled(&fixture.refresh(), "a.m4a", "A");
    let before = snapshot(&fixture.copy_path());
    {
        let existing = TitleCopy::existing(fixture.state.path()).unwrap();
        titled(&existing, "a.m4a", "A");
    }
    unchanged(&before, &snapshot(&fixture.copy_path()));
}
