use super::{finish, map};
use crate::ContainedError;
use crate::database::DatabaseError;
use rusqlite::ffi::*;
use std::io::ErrorKind;
use std::path::PathBuf;
use vpt_application::ports::LedgerError;

fn sqlite(code: i32, text: &str) -> rusqlite::Error {
    rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(code), Some(text.into()))
}

#[test]
fn busy_and_locked_codes_map_to_busy() {
    for code in [
        SQLITE_BUSY,
        SQLITE_BUSY_SNAPSHOT,
        SQLITE_LOCKED,
        SQLITE_LOCKED_SHAREDCACHE,
    ] {
        assert_eq!(
            map(sqlite(code, "private database content")),
            LedgerError::Busy
        );
    }
}

#[test]
fn constraint_codes_map_to_content_free_conflict() {
    let first = map(sqlite(SQLITE_CONSTRAINT_UNIQUE, "private title one"));
    assert!(matches!(first, LedgerError::Conflict(_)), "{first:?}");
    assert_eq!(
        first,
        map(sqlite(SQLITE_CONSTRAINT_FOREIGNKEY, "private title two"))
    );
    assert!(!format!("{first:?}").contains("private title"));
}

#[test]
fn corruption_codes_map_to_content_free_corruption() {
    for code in [SQLITE_CORRUPT, SQLITE_NOTADB] {
        let error = map(sqlite(code, "private corrupt bytes"));
        assert!(matches!(error, LedgerError::Corrupt(_)), "{error:?}");
        assert!(!format!("{error:?}").contains("private corrupt bytes"));
    }
}

#[test]
fn other_sqlite_errors_never_include_their_payload() {
    for error in [
        sqlite(SQLITE_IOERR, "private database content"),
        rusqlite::Error::InvalidParameterName("private database content".into()),
    ] {
        let mapped = map(error);
        assert!(matches!(mapped, LedgerError::Io(_)), "{mapped:?}");
        assert!(!format!("{mapped:?}").contains("private database content"));
    }
}

#[test]
fn typed_containment_keeps_the_refused_path() {
    let path = PathBuf::from("owned-state/vpt.db");
    for contained in [
        ContainedError::Escape {
            root: "owned-state".into(),
            path: path.clone(),
        },
        ContainedError::NotRegular(path.clone()),
        ContainedError::NotADirectory(path.clone()),
    ] {
        assert_eq!(
            LedgerError::from(DatabaseError::Contained(contained)),
            LedgerError::PathEscape(path.clone())
        );
    }
}

#[test]
fn typed_sqlite_failure_keeps_its_classification() {
    assert_eq!(
        LedgerError::from(DatabaseError::Sqlite(sqlite(SQLITE_LOCKED, "private"))),
        LedgerError::Busy
    );
}

#[test]
fn typed_storage_failure_reports_only_its_operation_and_kind() {
    let error = LedgerError::from(DatabaseError::Io {
        operation: "file sync",
        kind: ErrorKind::PermissionDenied,
    });
    assert!(
        matches!(&error, LedgerError::Io(message) if message.contains("file sync") && message.contains("PermissionDenied")),
        "{error:?}"
    );
}

#[test]
fn an_ordinary_contained_io_error_is_not_path_escape() {
    let error = LedgerError::from(DatabaseError::Contained(ContainedError::Io {
        path: "owned-state/vpt.db".into(),
        kind: ErrorKind::NotFound,
    }));
    assert!(
        matches!(&error, LedgerError::Io(message) if message.contains("owned-state/vpt.db") && message.contains("NotFound")),
        "{error:?}"
    );
}

#[test]
fn close_containment_takes_precedence_over_operation_failure() {
    let closed = LedgerError::PathEscape("owned-state".into());
    assert_eq!(
        finish::<()>(Err(LedgerError::Busy), Err(closed.clone())),
        Err(closed)
    );
}

#[test]
fn operation_failure_survives_an_ordinary_close_failure() {
    let operation = LedgerError::Conflict("constraint violated".into());
    assert_eq!(
        finish::<()>(Err(operation.clone()), Err(LedgerError::Io("close".into()))),
        Err(operation)
    );
}

#[test]
fn close_failure_replaces_an_operation_success() {
    let closed = LedgerError::Io("close".into());
    assert_eq!(finish(Ok(7), Ok(())).unwrap(), 7);
    assert_eq!(finish(Ok(7), Err(closed.clone())), Err(closed));
}
