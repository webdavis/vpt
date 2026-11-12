use crate::support::Sandbox;
use std::os::darwin::fs::MetadataExt as DarwinMetadataExt;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use vpt_adapters::{RootDir, SqliteLedger};
use vpt_application::ports::{
    DirtyPublication, LedgerCommit, RecordingLedger, RecordingRecord, StageStates, TitleOrigin,
};
use vpt_domain::digest::Sha256Digest;
use vpt_domain::identity::RecordingId;
use vpt_domain::time::{UtcInstant, UtcOffset};

pub(super) fn configured(name: &str) -> Sandbox {
    let sandbox = Sandbox::new(name);
    write_config(&sandbox, "");
    sandbox
}

pub(super) fn write_config(sandbox: &Sandbox, extra: &str) {
    let config = sandbox.config_path();
    let parent = config.parent().unwrap();
    std::fs::create_dir_all(parent).unwrap();
    std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::write(
        &config,
        format!(
            "config_version = 1\n[home]\nstate_dir = {:?}\n{extra}",
            sandbox.path().join("state/vpt")
        ),
    )
    .unwrap();
    std::fs::set_permissions(&config, std::fs::Permissions::from_mode(0o600)).unwrap();
}

pub(super) fn record(sandbox: &Sandbox, seed: u8, stages: StageStates) -> RecordingRecord {
    let digest = Sha256Digest([seed; 32]);
    let captured_at = UtcInstant {
        secs: 1_787_604_456 + i64::from(seed),
    };
    let captured_offset = UtcOffset { secs: -21_600 };
    let id = RecordingId::derive(captured_at, captured_offset, &digest).unwrap();
    RecordingRecord {
        id: id.clone(),
        source_path: Some(
            sandbox
                .path()
                .join(format!("voice-memos/Recordings/{seed}.m4a")),
        ),
        digest,
        captured_at,
        captured_offset,
        duration_secs: 19,
        title: Some(format!("title-{seed}")),
        title_source: TitleOrigin::VoiceMemos,
        ingested_at: UtcInstant {
            secs: 1_787_700_000,
        },
        audio_path: sandbox
            .path()
            .join(format!("home/.vpt/audio/{}.m4a", id.as_str())),
        stages,
        audio_trashed_at: None,
    }
}

pub(super) fn seed(sandbox: &Sandbox, records: Vec<RecordingRecord>) {
    let state = sandbox.path().join("state/vpt");
    std::fs::create_dir(&state).unwrap();
    std::fs::set_permissions(&state, std::fs::Permissions::from_mode(0o700)).unwrap();
    let root = RootDir::open(&state).unwrap();
    let ledger = SqliteLedger::open(&root).unwrap();
    ledger
        .commit(&LedgerCommit {
            recordings: records,
            ..Default::default()
        })
        .unwrap();
}

pub(super) fn preserve_wal(connection: &rusqlite::Connection) {
    let mut enabled: std::ffi::c_int = 1;
    // SAFETY: the connection and integer remain valid throughout the call.
    let result = unsafe {
        rusqlite::ffi::sqlite3_file_control(
            connection.handle(),
            c"main".as_ptr(),
            rusqlite::ffi::SQLITE_FCNTL_PERSIST_WAL,
            (&mut enabled as *mut std::ffi::c_int).cast(),
        )
    };
    assert_eq!(result, rusqlite::ffi::SQLITE_OK);
}

pub(super) fn dirty(sandbox: &Sandbox, record: RecordingRecord) {
    seed(sandbox, vec![record]);
    let state = RootDir::open(&sandbox.path().join("state/vpt")).unwrap();
    let ledger = SqliteLedger::open(&state).unwrap();
    ledger
        .commit(&LedgerCommit {
            publications: vec![DirtyPublication {
                target: sandbox.path().join("home/.vpt/transcripts/operator.md"),
                expected_previous: None,
                intended: Sha256Digest([0x42; 32]),
                recorded_at: UtcInstant { secs: 11 },
            }],
            ..Default::default()
        })
        .unwrap();
    std::fs::create_dir_all(sandbox.path().join("home/.vpt/transcripts")).unwrap();
    std::fs::write(
        sandbox.path().join("home/.vpt/transcripts/operator.md"),
        b"operator prose",
    )
    .unwrap();
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct SnapshotEntry {
    path: PathBuf,
    kind: u32,
    device: u64,
    inode: u64,
    links: u64,
    size: u64,
    ctime: i64,
    ctime_nanos: i64,
    flags: u32,
    mode: u32,
    mtime: i64,
    mtime_nanos: i64,
    bytes: Vec<u8>,
    link_target: Option<PathBuf>,
}

#[allow(
    dead_code,
    reason = "source preservation checks use the shared snapshot"
)]
impl SnapshotEntry {
    pub(super) fn is_within(&self, root: &Path) -> bool {
        self.path.starts_with(root)
    }
}

pub(super) fn writer_ready(state: &Path) -> bool {
    matches!(std::fs::read(state.join("writer-ready")), Ok(bytes) if bytes == b"ready")
}

/// Writes back the WAL index pages a test's own SQLite connection left dirty through its shared
/// mapping. The kernel otherwise writes them back seconds later and changes the file's times
/// after the snapshot was taken.
fn settle(path: &Path) {
    if !path.to_string_lossy().ends_with("-shm") {
        return;
    }
    let file = std::fs::File::open(path).unwrap();
    // SAFETY: fsync receives the descriptor this function owns.
    let result = unsafe { libc::fsync(std::os::fd::AsRawFd::as_raw_fd(&file)) };
    assert_eq!(
        result,
        0,
        "snapshot fsync: {}",
        std::io::Error::last_os_error()
    );
}

pub(super) fn snapshot(sandbox: &Sandbox) -> Vec<SnapshotEntry> {
    fn visit(path: &Path, rows: &mut Vec<SnapshotEntry>) {
        let metadata = match path.symlink_metadata() {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
            Err(error) => panic!("snapshot: {error}"),
        };
        let (metadata, bytes) = if metadata.is_file() {
            settle(path);
            (
                path.symlink_metadata().unwrap(),
                std::fs::read(path).unwrap(),
            )
        } else {
            (metadata, Vec::new())
        };
        let link_target = if metadata.file_type().is_symlink() {
            Some(std::fs::read_link(path).unwrap())
        } else {
            None
        };
        rows.push(SnapshotEntry {
            path: path.to_path_buf(),
            kind: metadata.mode() & libc::S_IFMT as u32,
            device: metadata.dev(),
            inode: metadata.ino(),
            links: metadata.nlink(),
            size: metadata.size(),
            ctime: metadata.ctime(),
            ctime_nanos: metadata.ctime_nsec(),
            flags: metadata.st_flags(),
            mode: metadata.permissions().mode() & 0o777,
            mtime: metadata.mtime(),
            mtime_nanos: metadata.mtime_nsec(),
            bytes,
            link_target,
        });
        if metadata.is_dir() {
            for entry in std::fs::read_dir(path).unwrap() {
                visit(&entry.unwrap().path(), rows);
            }
        }
    }
    let mut rows = Vec::new();
    visit(sandbox.path(), &mut rows);
    rows.sort_by(|left, right| left.path.cmp(&right.path));
    rows
}
