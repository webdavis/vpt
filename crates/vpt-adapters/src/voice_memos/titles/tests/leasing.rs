use super::fixture::{Fixture, titled, unchanged};
use super::*;
use crate::database::{DatabaseError, snapshot};
use crate::voice_memos::titles::lease::CopyLease;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::time::Instant;

const SHORT: Duration = Duration::from_millis(20);

fn copy(root: &RootDir) -> RootDir {
    let copy = root.subdirectory(COPY_DIRECTORY).unwrap();
    fs::write(copy.path().join(DATABASE), b"unchanged").unwrap();
    fs::set_permissions(
        copy.path().join(DATABASE),
        fs::Permissions::from_mode(0o644),
    )
    .unwrap();
    copy
}

fn busy<T>(result: Result<T, DatabaseError>) {
    assert!(
        matches!(result, Err(DatabaseError::Sqlite(rusqlite::Error::SqliteFailure(code, _)))
        if code.code == rusqlite::ErrorCode::DatabaseBusy),
        "lease contention must be Busy"
    );
}

pub(super) fn shared(root: &RootDir) {
    let copy = copy(root);
    let before = snapshot(root.path());
    let reader = CopyLease::read(&copy, SHORT).unwrap();
    let other = CopyLease::read(&copy, SHORT).unwrap();
    assert_eq!(
        reader.file().metadata().unwrap().ino(),
        fs::metadata(copy.path().join(DATABASE)).unwrap().ino()
    );
    let started = Instant::now();
    busy(CopyLease::refresh(&copy, SHORT));
    assert!(started.elapsed() >= SHORT);
    assert!(started.elapsed() < crate::database::WATCHDOG);
    unchanged(&before, &snapshot(root.path()));
    drop((reader, other));
    assert!(CopyLease::refresh(&copy, SHORT).is_ok());
}

pub(super) fn exclusive(root: &RootDir) {
    let copy = copy(root);
    let before = snapshot(root.path());
    let mut writer = CopyLease::refresh(&copy, SHORT).unwrap();
    busy(CopyLease::read(&copy, SHORT));
    unchanged(&before, &snapshot(root.path()));
    writer.downgrade().unwrap();
    let reader = CopyLease::read(&copy, SHORT).unwrap();
    busy(CopyLease::refresh(&copy, SHORT));
    drop(writer);
    busy(CopyLease::refresh(&copy, SHORT));
    drop(reader);
    assert!(CopyLease::refresh(&copy, SHORT).is_ok());
}

pub(super) fn entries(root: &RootDir) {
    let copy = copy(root);
    let main = copy.path().join(DATABASE);
    fs::hard_link(&main, root.path().join("alias")).unwrap();
    let before = snapshot(root.path());
    for result in [
        CopyLease::read(&copy, SHORT),
        CopyLease::refresh(&copy, SHORT),
    ] {
        assert!(matches!(
            result,
            Err(DatabaseError::Contained(ContainedError::Escape { .. }))
        ));
    }
    unchanged(&before, &snapshot(root.path()));
    let link = root.subdirectory("link").unwrap();
    symlink(&main, link.path().join(DATABASE)).unwrap();
    assert!(matches!(
        CopyLease::refresh(&link, SHORT),
        Err(DatabaseError::Contained(ContainedError::NotRegular(_)))
    ));
    let directory = root.subdirectory("directory").unwrap();
    fs::create_dir(directory.path().join(DATABASE)).unwrap();
    assert!(CopyLease::read(&directory, SHORT).is_err());
}

pub(super) fn create(root: &RootDir) {
    let copy = root.subdirectory(COPY_DIRECTORY).unwrap();
    let lease = CopyLease::refresh(&copy, SHORT);
    assert!(
        lease.is_ok(),
        "refresh must acquire a newly created main file"
    );
    let lease = lease.unwrap();
    let metadata = lease.file().metadata().unwrap();
    assert_eq!(metadata.len(), 0);
    assert_eq!(metadata.nlink(), 1);
    assert_eq!(metadata.mode() & 0o077, 0);
    assert_eq!(
        metadata.ino(),
        fs::metadata(copy.path().join(DATABASE)).unwrap().ino()
    );
}

pub(super) fn owner(root: &RootDir) {
    let fixture = Fixture::simple(root);
    let copy = fixture.refresh();
    titled(&copy, "a.m4a", "A");
    let private = fixture.state.open_subdirectory(COPY_DIRECTORY).unwrap();
    let reader = CopyLease::read(&private, SHORT).expect("title owner must downgrade to shared");
    drop(reader);
    let before = snapshot(&fixture.copy_path());
    let blocked =
        TitleCopy::refresh_with_timeout(&fixture.container, &fixture.state, SHORT).unwrap();
    assert_eq!(blocked.title("a.m4a"), TitleLookup::Unavailable);
    unchanged(&before, &snapshot(&fixture.copy_path()));
    let ledger_root = root.subdirectory("ledger").unwrap();
    let ledger = crate::SqliteLedger::open(&ledger_root).unwrap();
    assert_eq!(ledger.schema_version().unwrap(), crate::SCHEMA_VERSION);
    drop(copy);
    titled(&fixture.refresh(), "a.m4a", "A");
}

pub(super) fn replaced(root: &RootDir) {
    let copy = copy(root);
    let lease = CopyLease::refresh(&copy, SHORT).unwrap();
    let original = lease.file().metadata().unwrap().ino();
    fs::rename(copy.path().join(DATABASE), copy.path().join("saved.db")).unwrap();
    fs::write(copy.path().join(DATABASE), b"replacement").unwrap();
    let before = snapshot(root.path());
    assert!(matches!(
        lease.revalidate(),
        Err(DatabaseError::Contained(ContainedError::Escape { .. }))
    ));
    assert_eq!(lease.file().metadata().unwrap().ino(), original);
    unchanged(&before, &snapshot(root.path()));
    let lease = CopyLease::read(&copy, SHORT).unwrap();
    fs::rename(copy.path(), root.path().join("saved-copy")).unwrap();
    fs::create_dir(copy.path()).unwrap();
    let before = snapshot(root.path());
    assert!(matches!(
        lease.revalidate(),
        Err(DatabaseError::Contained(ContainedError::Escape { .. }))
    ));
    drop(lease);
    unchanged(&before, &snapshot(root.path()));
}
