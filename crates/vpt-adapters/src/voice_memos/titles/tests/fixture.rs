use super::*;
use crate::database::snapshot;
use rusqlite::Connection;
use std::fs;
use std::path::PathBuf;
use vpt_application::ports::TitleLookup;

pub(super) struct Fixture {
    pub(super) container: PathBuf,
    pub(super) state: RootDir,
    pub(super) source: Connection,
}

impl Fixture {
    pub(super) fn new(root: &RootDir, rows: &[(&str, Option<&str>)]) -> Self {
        Self::table(root, "ZCLOUDRECORDING", rows)
    }

    pub(super) fn table(root: &RootDir, table: &str, rows: &[(&str, Option<&str>)]) -> Self {
        let container = root.subdirectory("source").unwrap();
        fs::create_dir(container.path().join("Recordings")).unwrap();
        let source = Connection::open(container.path().join(DATABASE)).unwrap();
        source.busy_timeout(Duration::ZERO).unwrap();
        source.pragma_update(None, "journal_mode", "WAL").unwrap();
        source.execute_batch(&format!(
            "CREATE TABLE \"{}\" (ZPATH TEXT, ZCUSTOMLABEL TEXT); PRAGMA wal_checkpoint(TRUNCATE);",
            table.replace('"', "\"\"")
        )).unwrap();
        let insert = format!(
            "INSERT INTO \"{}\" VALUES (?1, ?2)",
            table.replace('"', "\"\"")
        );
        for (path, title) in rows {
            source
                .execute(&insert, rusqlite::params![path, title])
                .unwrap();
        }
        Self {
            container: container.path().into(),
            state: root.subdirectory("private").unwrap(),
            source,
        }
    }

    pub(super) fn simple(root: &RootDir) -> Self {
        Self::new(root, &[("a.m4a", Some("A"))])
    }

    pub(super) fn refresh(&self) -> TitleCopy {
        TitleCopy::refresh(&self.container, self.state.path()).unwrap()
    }

    pub(super) fn copy_path(&self) -> PathBuf {
        self.state.path().join(COPY_DIRECTORY)
    }

    pub(super) fn unchanged_source<T>(&self, action: impl FnOnce() -> T) -> T {
        let before = snapshot(&self.container);
        let result = action();
        unchanged(&before, &snapshot(&self.container));
        result
    }
}

pub(super) fn titled(copy: &TitleCopy, file: &str, expected: &str) {
    assert_eq!(copy.title(file), TitleLookup::Titled(expected.into()));
}

pub(super) fn unchanged<T: std::fmt::Debug + PartialEq>(before: &[T], after: &[T]) {
    assert_eq!(before.len(), after.len(), "snapshot entry count");
    for (index, (before, after)) in before.iter().zip(after).enumerate() {
        assert!(
            before == after,
            "snapshot entry {index}: before={} after={}",
            format!("{before:?}").split(", bytes:").next().unwrap(),
            format!("{after:?}").split(", bytes:").next().unwrap()
        );
    }
}
