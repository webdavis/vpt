# VPT Database Containment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended)
> or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax
> for tracking.

**Goal:** Keep every SQLite file operation attached to VPT's validated directory while preserving
concurrent readers, private title copies and observational commands.

**Architecture:** A private `database` module in `vpt-adapters` supplies one concrete connection owner
and a registered SQLite filesystem for that connection. It uses checked directory handles, macOS
descriptor-owned record locks and shared-memory mappings. The ledger and title reader use this owner;
application ports and database formats stay unchanged.

**Tech Stack:** Existing Rust edition 2024 workspace, `rusqlite =0.39.0` with `bundled` and no default
features, bundled SQLite 3.51.3, `libc 0.2.189`, and test-only `tempfile 3.27`. No dependency change.

**Spec:** [Approved database amendment](../specs/2026-09-22-vpt-database-containment-design.md), together
with the [original design](../specs/2026-09-20-vpt-design.md) and unaffected parts of the
[Stage 1 plan](2026-09-21-vpt-stage-1-extract-plan.md).

## Global Constraints

These requirements are copied from the approved amendment. The original plan's unaffected global
constraints remain in force.

- SQLite and `rusqlite` remain unmodified.
- There is no new daemon, configuration key or package.
- Domain, application and protocol crates keep their existing responsibilities and gain no SQLite or
  filesystem dependency.
- Use concrete types and a closed access-policy enum.
- Register the filesystem by an internal name and select it explicitly for each VPT connection.
- It must not become the process-wide default or replace SQLite's system-call hooks.
- Unregister before freeing storage.
- Create private directories at `0700` and persistent files at `0600`.
- A pre-existing multiply linked database or companion file is refused.
- Use memory for SQLite temporary storage.
- Optional database-page mapping is disabled initially; shared-memory mapping remains required.
- Use a five-second busy timeout on every production connection.
- Keep foreign-key enforcement on the ledger.
- Do not mark the live ledger immutable, because other commands may update it.
- Source files in the Voice Memos container are opened only through VPT's read-only file handles.
- Adapter-local signatures may change to retain these errors; application repository contracts remain
  unchanged.
- Every test finishes within one second under the workspace suite.
- No third-party source patch or SQLite system-call override is part of these tests.
- Keep unsafe operations private, source files below the hard 500-line ceiling and the command entry
  point below 150 lines.
- This amendment changes no shipped data format and authorizes no installation, live-data test,
  third-party modification, push or pull request.

## Review Focus

1. A valid directory disappears between validation and a lazy companion open or close. Retained authority
   must protect the replacement directory, including cleanup. Tests: D1 and D3.
1. Opening or closing an unrelated connection releases a writer's locks. Descriptor-owned locks must keep
   protecting it, with native interoperability tested in another process. Tests: D2 and D4.
1. The last writer has closed and the shared-memory file exists but has no initialization lock.
   Observation must read without repairing it, or return a typed failure without changing files. Tests:
   D4 and D5.
1. A failed open, abandoned statement or callback failure outlives its Rust owner. Registration and file
   state must survive until SQLite releases them. Tests: D3.
1. A refresh truncates a title copy another reader still uses. A separate descriptor lease must cover
   refresh and the reader's entire connection lifetime. Tests: D6.

______________________________________________________________________

## Scope and execution order

Tasks 1 through 3 are complete. Do not repeat them. Resume original Tasks 4, 5 and 5a with the execution
ledger's corrections, then Tasks 6 through 10. Execute D1 through D5 in place of original Task 11. Resume
Tasks 12 through 15, execute D6 in place of Task 16, then continue the original plan. D7 supplies the
database-related changes and acceptance checks within Tasks 27, 28 and 30 through 32.

Keep the existing separate implementer and reviewer workflow, one product writer at a time. Each task
below ends with its own reviewable deliverable and commit. A compiling behavioral failure is required
before each implementation change; unresolved imports and a zero-test selection are not red evidence.
Record exact mutations and unmodified controls in the existing ignored execution ledger.

Original Task 11's schema SQL, migration version and public ledger methods remain the contract. Its
pathname-based connection opener and native same-process contention fixture are replaced. Original Task
16's title selection SQL and missing-source-companion behavior remain; its connection, refresh and
failure-handling code are replaced. SQLite may remove its own rollback journal through this adapter; this
does not authorize deletion of recordings, artifacts or user-owned directories.

## File map

All paths below are relative to the VPT worktree. Add the private children when their task needs them.

| File                                                      | Responsibility                                                                   |
| --------------------------------------------------------- | -------------------------------------------------------------------------------- |
| `crates/vpt-adapters/src/contained.rs`                    | Narrow missing checked-root operations                                           |
| `crates/vpt-adapters/src/contained/tests.rs`              | Descriptor-relative operation regressions                                        |
| `crates/vpt-adapters/src/database/mod.rs`                 | Private module wiring, access policy and typed errors                            |
| `crates/vpt-adapters/src/database/files.rs`               | Exact name/role allowlist and owned regular files                                |
| `crates/vpt-adapters/src/database/locks.rs`               | Descriptor-owned record locks and database transitions                           |
| `crates/vpt-adapters/src/database/owner.rs`               | Open, borrowed use, explicit close and registration ownership                    |
| `crates/vpt-adapters/src/database/registration.rs`        | Stable registration and filesystem callbacks                                     |
| `crates/vpt-adapters/src/database/io.rs`                  | SQLite file callbacks and durability                                             |
| `crates/vpt-adapters/src/database/shared_memory.rs`       | Mappings, initialization and shared-memory lock slots                            |
| `crates/vpt-adapters/src/database/faults.rs`              | Private connection-local test faults with a zero-sized production implementation |
| `crates/vpt-adapters/src/database/tests/mod.rs`           | Private test-child declarations                                                  |
| `crates/vpt-adapters/src/database/tests/files.rs`         | Name, role, permission and containment cases                                     |
| `crates/vpt-adapters/src/database/tests/locks.rs`         | Lock transitions and independent-close cases                                     |
| `crates/vpt-adapters/src/database/tests/owner.rs`         | Failed open, close, callback and lifetime cases                                  |
| `crates/vpt-adapters/src/database/tests/wal.rs`           | Concurrent snapshots and native interoperability                                 |
| `crates/vpt-adapters/src/database/tests/failures.rs`      | Write, sync, crash and observational cases                                       |
| `crates/vpt-adapters/src/database/tests/support.rs`       | Owned directories, snapshots and bounded child supervision                       |
| `crates/vpt-adapters/src/database/tests/children.rs`      | Explicit test-only native and fault scenarios                                    |
| `crates/vpt-adapters/src/ledger/sqlite/connection.rs`     | Ledger policy selection and error mapping                                        |
| `crates/vpt-adapters/src/ledger/sqlite/mod.rs`            | Original public ledger API over the new owner                                    |
| `crates/vpt-adapters/src/ledger/sqlite/migrations.rs`     | Original schema and transactional migration                                      |
| `crates/vpt-adapters/src/ledger/sqlite/boundary_tests.rs` | Durable ledger and observation contract                                          |
| `crates/vpt-adapters/src/voice_memos/titles.rs`           | Copy, open, lookup and typed failure handling                                    |
| `crates/vpt-adapters/src/voice_memos/titles/lease.rs`     | Private copy reader/refresh lease                                                |
| `crates/vpt-adapters/src/voice_memos/titles/tests.rs`     | Copy, source snapshot and lease cases                                            |

D5 and D6 retain the original plan's application declarations and curated exports. D7 lists its command
files below. Keep implementation modules private. Split a file by responsibility before it exceeds the
hard size limit; do not hide code behind macros or broad lint allowances.

## Source anchors

Read the locked sources before implementing their contracts. These are implementation references, not
evidence that the new adapter already works.

- `libsqlite3-sys-0.37.0/sqlite3/sqlite3.c`: database lock bytes and transitions at 16313, 41041 and
  41325; shared-memory initialization and mapping at 44051 and 44305; shared-memory locks/barrier at
  44485 and 44681; persistent WAL close at 69255; read-only recovery at 69422; connection close at
  186159\. The exact pinned source controls if line numbers shift.
- `rusqlite-0.39.0/src/lib.rs`: `Connection::close`, `from_handle_owned`, and `handle`.
  `src/inner_connection.rs`: failed-open cleanup and `Drop`. The convenience opener discards the result
  of cleanup after an open error. This adapter must own that cleanup explicitly.
- macOS SDK `fcntl(2)`: `F_OFD_SETLK`, `F_OFD_GETLK`, byte ranges and descriptor lifetime. Closing any
  descriptor can release the same process's traditional record locks on that inode. Consequently all
  production VPT opens use this adapter; overlapping native reference connections run in a separate
  process. Native fixture connections in the same process must use different files or close before the
  rooted connection opens.
- SQLite's [filesystem interface](https://www.sqlite.org/c3ref/vfs.html),
  [file methods](https://www.sqlite.org/c3ref/io_methods.html) and
  [WAL locking format](https://www.sqlite.org/walformat.html#wal_locks).

## D1: Checked database files and access policies

**Files:** Modify `contained.rs`, `contained/tests.rs` and adapters `lib.rs`; create `database/mod.rs`,
`database/files.rs`, `database/tests/mod.rs`, `database/tests/files.rs` and `database/tests/support.rs`.

**Interfaces:** Consume original Task 5a's `RootDir`, `Access`, `ContainedError`, `Kind`, `leaf`,
`open_file`, `stat`, `try_clone`, `revalidate` and `sync`. Add only these checked-root methods:

```rust
pub(crate) fn create_read_write_file(
    &self, leaf: &Path, mode: u32,
) -> Result<File, ContainedError>;
pub(crate) fn unlink_file(&self, leaf: &Path) -> Result<(), ContainedError>;
```

Create the following private adapter types. `FileRole` and `DatabaseFiles` remain inside `database`; the
policy and error are visible to sibling adapters through curated crate-private exports.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AccessPolicy {
    WritableLedger,
    ObservationalLedger,
    RefreshedTitleCopy,
    ExistingTitleCopy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FileRole { Database, Journal, Wal, SharedMemory }

#[derive(Debug)]
pub(crate) enum DatabaseError {
    Contained(ContainedError),
    Sqlite(rusqlite::Error),
    Io { operation: &'static str, kind: std::io::ErrorKind },
}

struct DatabaseFiles {
    root: RootDir,
    leaf: std::ffi::CString,
    policy: AccessPolicy,
}
```

Define
`DatabaseFiles::new(root: RootDir, leaf: &Path, policy: AccessPolicy) -> Result<Self, DatabaseError>`,
`open(&self, role: FileRole, create: bool) -> Result<File, DatabaseError>`,
`remove(&self, role: FileRole, sync: bool) -> Result<(), DatabaseError>` and
`revalidate(&self) -> Result<(), DatabaseError>`. `DatabaseError` implements `From<ContainedError>`,
`From<rusqlite::Error>` and a private `is_containment(&self) -> bool`. Unsafe file kinds, hard links and
root substitution are containment refusals. Plain permission, space and device failures remain
input/output errors.

- [ ] **Step 1: Add compiling red tests and the smallest signatures they require.**

Use explicit `Unsupported` errors as temporary red stubs, never panics or successful dummy values. Add
the owned fixture below to `database/tests/support.rs`; extend it only when later tests need a helper.
Each test child imports the named production types from its parent, `std::path::Path`,
`std::time::Duration`, and `super::support::root` when used. Keep these imports explicit in the files.

```rust
pub(super) fn root() -> (tempfile::TempDir, crate::RootDir) {
    use std::os::unix::fs::DirBuilderExt;
    let temp = tempfile::tempdir().expect("owned temp");
    let path = temp.path().canonicalize().expect("canonical temp").join("state");
    std::fs::DirBuilder::new().mode(0o700).create(&path).expect("state");
    let root = crate::RootDir::open(&path).expect("checked root");
    (temp, root)
}
```

Add these tests in `database/tests/files.rs`, importing the parent types and `support::root`:

```rust
#[test]
fn lazy_open_stays_with_the_acquired_directory() {
    use std::io::Write;
    let (temp, root) = root();
    let original = root.path().to_path_buf();
    let saved = temp.path().join("saved");
    let files = DatabaseFiles::new(root, Path::new("vpt.db"), AccessPolicy::WritableLedger)
        .expect("files");
    files.revalidate().expect("valid before replacement");
    std::fs::rename(&original, &saved).expect("retain original inode");
    std::fs::create_dir(&original).expect("replacement");
    files.open(FileRole::Journal, true).expect("held root open")
        .write_all(b"owned").expect("write");
    assert_eq!(std::fs::read(saved.join("vpt.db-journal")).unwrap(), b"owned");
    assert!(!original.join("vpt.db-journal").exists());
    assert!(files.revalidate().unwrap_err().is_containment());
    files.remove(FileRole::Journal, true).expect("contained cleanup");
    assert!(!saved.join("vpt.db-journal").exists());
    assert!(std::fs::read_dir(original).unwrap().next().is_none());
}

#[test]
fn observation_neither_creates_nor_repairs() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let (_temp, root) = root();
    let path = root.path().join("vpt.db");
    std::fs::write(&path, b"fixture").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640)).unwrap();
    let files = DatabaseFiles::new(root, Path::new("vpt.db"), AccessPolicy::ObservationalLedger)
        .unwrap();
    assert!(files.open(FileRole::Database, false).is_ok());
    assert!(files.open(FileRole::Wal, true).is_err());
    assert_eq!(std::fs::metadata(path).unwrap().mode() & 0o777, 0o640);
}
```

Add table-driven leaves for each role with a symbolic link, directory and multiply linked regular file in
its place. The outside target's bytes and metadata must remain unchanged. Add rejected leaf values
`../vpt.db`, an absolute path, `vpt.db/child`, an empty name and an embedded NUL. Cover a root ancestor
replaced after acquisition, not only the final directory. D2 adds first-in, first-out (FIFO) special-file
cases under its child watchdog and mutation-tests the nonblocking guard. Task 5a's existing type checks
remain active throughout.

- [ ] **Step 2: Run the selected tests red.**

```bash
cargo test --locked -p vpt-adapters database::tests::files -- --nocapture
```

Expected: the new leaves run and fail at their specific successful-open or containment assertions.
Compilation failure does not count.

- [ ] **Step 3: Implement the descriptor operations and policy table.**

`create_read_write_file` validates a single leaf, calls `openat` with
`O_RDWR | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC | O_NONBLOCK`, verifies the descriptor is a regular
file with link count one, then uses descriptor permission repair. Never use `O_TRUNC` while opening an
unchecked file. `unlink_file` checks the named entry without following links, refuses non-regular or
multiply linked entries, and calls `unlinkat` relative to the retained descriptor. It never accepts a
directory. File creation/removal is followed by `RootDir::sync`; propagate failure.

Apply this policy at every callback, not only the initial database open:

```rust
impl AccessPolicy {
    fn writable(self, role: FileRole) -> bool {
        match self {
            Self::WritableLedger => true,
            Self::RefreshedTitleCopy => role == FileRole::SharedMemory,
            Self::ObservationalLedger | Self::ExistingTitleCopy => false,
        }
    }
}
```

`create` is permission to create only when the policy permits it. A read-only policy may satisfy SQLite's
writable WAL probe with an existing read-only descriptor and must report that actual mode; it must never
create or repair the file. An existing writable open repairs to `0600` only after the type and link
checks. Preserve `O_EXCL` creation races: an `AlreadyExists` result permits a fresh checked open, never
an unchecked fallback. Build companions by appending exactly `-journal`, `-wal` or `-shm` to the
validated database leaf. All physical operations consume those names relative to the root.

Revalidation detects changed device/inode, links, or a now-missing acquired pathname as
`ContainedError::Escape`. Initial acquisition of an absent state directory remains the caller's
empty-state case. Low-level callbacks continue using retained authority during error cleanup even after
revalidation has refused a new query.

- [ ] **Step 4: Verify green and mutation controls.**

Run the red command again. Mutate the descriptor-relative open to a pathname open, remove the link-count
guard, and enable observation's creation/permission repair one at a time. Each corresponding leaf must
fail; restore exact original bytes and rerun the control. Check both replacement-directory and ancestor
cases. Record created file modes and unchanged outside snapshots.

- [ ] **Step 5: Format, review and commit this deliverable.**

```bash
cargo fmt --all
cargo clippy --locked -p vpt-adapters --all-targets -- -D warnings
git add crates/vpt-adapters/src/contained.rs crates/vpt-adapters/src/contained/tests.rs
git add crates/vpt-adapters/src/lib.rs crates/vpt-adapters/src/database
git commit -m "feat(database): contain database file access"
```

## D2: Descriptor-owned locks and bounded process fixtures

**Files:** Create `database/locks.rs`, `database/tests/locks.rs` and `database/tests/children.rs`; modify
`database/mod.rs`, `database/tests/mod.rs` and `database/tests/support.rs`.

**Interfaces:** Consume D1's owned regular files. Produce crate-private
`LockKind::{Shared, Exclusive, Unlock}`,
`set_record_lock(file: &File, start: i64, length: i64, kind: LockKind) -> Result<(), DatabaseError>`, and
private `has_conflict(file: &File, start: i64, length: i64) -> Result<bool, DatabaseError>`.
`DatabaseLocks::new(file: File)` owns its file; `lock(&mut self, level: i32)` and
`unlock(&mut self, level: i32)` return `Result<(), DatabaseError>`; `reserved(&self)` returns
`Result<bool, DatabaseError>`. Private `file(&self) -> &File` serves the file callbacks and
`level(&self) -> i32` serves SQLite's lock-state query. A file state owns this lock owner instead of
duplicating its descriptor. The `level` arguments are checked SQLite lock constants, not arbitrary states
stored unchecked. Keep each connection on an independently opened file description.

The private test harness produces these interfaces. `ChildGuard` owns the child process group and output
readers.

```rust
pub(super) fn child(scenario: &str, directory: &Path) -> std::io::Result<ChildGuard>;
impl ChildGuard {
    pub(super) fn ready(&mut self) -> std::io::Result<()>;
    pub(super) fn send(&mut self, command: &str) -> std::io::Result<()>;
    pub(super) fn finish(self) -> std::io::Result<std::process::Output>;
}
```

- [ ] **Step 1: Write compiling lock and watchdog tests.**

```rust
#[test]
fn closing_another_description_keeps_the_writer_lock() {
    let (_temp, root) = root();
    let leaf = Path::new("vpt.db");
    let first = root.create_read_write_file(leaf, 0o600).unwrap();
    let unrelated = root.open_file(leaf, Access::ReadWrite).unwrap();
    let contender = root.open_file(leaf, Access::ReadWrite).unwrap();
    set_record_lock(&first, 0x4000_0001, 1, LockKind::Exclusive).unwrap();
    drop(unrelated);
    let error = set_record_lock(&contender, 0x4000_0001, 1, LockKind::Exclusive).unwrap_err();
    assert!(matches!(error, DatabaseError::Sqlite(rusqlite::Error::SqliteFailure(code, _))
        if code.code == rusqlite::ErrorCode::DatabaseBusy));
    drop(first);
    set_record_lock(&contender, 0x4000_0001, 1, LockKind::Exclusive).unwrap();
}
```

Add distinct transition leaves: two shared holders coexist; reserved excludes another reserved but allows
readers; pending blocks new readers; failed promotion retains pending; downgrade releases
pending/reserved but retains shared; unlock releases this description only. Verify the reserved query
with the current owner and a different description.

Add `fifo_database`, `fifo_journal`, `fifo_wal` and `fifo_shared_memory` child scenarios. Each creates
one owned FIFO, attempts the corresponding D1 open, and asserts a containment refusal. No child opens the
FIFO's writing end. These tests pin the existing nonblocking type check without risking a stalled test
runner.

The child entry point is a `#[test]` leaf selected with the exact fully qualified test name by
`std::env::current_exe()`. With no scenario marker it returns immediately. Its scenario is a fixed match
over test-owned names, not arbitrary commands. Clear the environment; set owned `HOME`, all three XDG
roots, `VPT_CONFIG` and `PATH`. Pass the synthetic root as one `OsString` argument or environment value.
Use pipe handshakes for ready/continue messages.

```rust
#[test]
fn child_watchdog_reaps_a_stalled_lock_test() {
    let (temp, _root) = root();
    let started = std::time::Instant::now();
    let process = child("stall", temp.path()).unwrap();
    assert_eq!(process.finish().unwrap_err().kind(), std::io::ErrorKind::TimedOut);
    assert!(started.elapsed() < std::time::Duration::from_secs(1));
}
```

- [ ] **Step 2: Run red, with a watchdog independent of the lock implementation.**

```bash
cargo test --locked -p vpt-adapters database::tests::locks -- --nocapture
```

Expected: lock assertions fail against explicit red stubs. The deliberate `stall` child is killed and
reaped. Its parent deadline is 500 ms, including readiness and output collection; cleanup has a bounded
50 ms graceful phase followed by termination of the owned group and reaping. Reuse the existing command
test harness's ownership pattern within this crate, without including files from another crate.

- [ ] **Step 3: Implement the lock protocol with nonblocking calls.**

```rust
const PENDING_BYTE: i64 = 0x4000_0000;
const RESERVED_BYTE: i64 = PENDING_BYTE + 1;
const SHARED_FIRST: i64 = PENDING_BYTE + 2;
const SHARED_SIZE: i64 = 510;
```

Build a zeroed `libc::flock`, assign `l_whence = SEEK_SET`, the checked range, and `F_RDLCK`, `F_WRLCK`
or `F_UNLCK`. Use `fcntl(fd, F_OFD_SETLK, &lock)`, never the blocking variant. Map `EACCES` and `EAGAIN`
to SQLite busy, interrupted calls to a retry of this nonblocking operation, and unsupported locking or
other errors to a typed failure. Query conflicts with `F_OFD_GETLK`; include this connection's known
reserved-or-higher state because a query does not conflict with its own description.

| Transition                   | Exact operations                                                                                   |
| ---------------------------- | -------------------------------------------------------------------------------------------------- |
| None to shared               | Read-lock pending byte; read-lock all 510 shared bytes; release pending byte, including on failure |
| Shared to reserved           | Write-lock reserved byte; keep shared lock                                                         |
| Shared/reserved to exclusive | Write-lock pending byte, then all shared bytes; on contention retain pending and report busy       |
| Pending to exclusive         | Retry the exclusive shared-range lock                                                              |
| Exclusive/pending to shared  | Convert shared range to read lock, then release pending and reserved bytes                         |
| Any to none                  | Release this description's locks and update its level                                              |

Update recorded levels only after successful transitions, except the deliberate retained pending state.
Propagate unlock errors. No process-global inode table or application `write.lock` substitutes for these
locks.

- [ ] **Step 4: Run green, native lock contention and hang controls.**

Add native child scenarios using the bundled SQLite `unix-posix` filesystem when registered. If that name
is absent, inspect the bundled build's `unix` implementation and select it only when it is the POSIX
record-lock implementation. Unsupported reference locking is a failed test with a clear reason, not a
passing skip. Test native writer versus rooted contender and rooted writer versus native contender. Do
not overlap native/rooted connections on the same inode in one process.

For the native contention tests, create a valid rollback-mode fixture with a native connection and close
it first. The rooted participant uses D2's file locks; the other process uses a real SQLite transaction.
This deliverable needs no future D3 owner. D4 adds full rooted SQL interoperability.

Remove `O_NONBLOCK` in the FIFO control mutation. The independent parent watchdog must fail the named
test and reap all owned children within one second. Mutate `F_OFD_SETLK` to `F_SETLK`; the
independent-close regression must fail. Restore each mutation and run the unmodified control. D4 adds the
real SQLite busy-timeout mutation once that bounded production operation exists.

- [ ] **Step 5: Format, review and commit.**

```bash
cargo fmt --all
cargo test --locked -p vpt-adapters database::tests::locks
git add crates/vpt-adapters/src/database
git commit -m "feat(database): coordinate descriptor-owned locks"
```

## D3: Registered filesystem, connection lifetime and rollback transactions

**Files:** Create `database/owner.rs`, `database/registration.rs`, `database/io.rs`, `database/faults.rs`
and `database/tests/owner.rs`; modify D1's files and test support as required by the interfaces below.

**Interfaces:** Produce the concrete crate-private owner:

```rust
pub(crate) struct Database {
    connection: Option<rusqlite::Connection>,
    registration: Option<Registration>,
    not_send_or_sync: std::marker::PhantomData<std::rc::Rc<()>>,
}

impl Database {
    pub(crate) fn open(
        root: RootDir, leaf: &Path, policy: AccessPolicy, busy: Duration,
    ) -> Result<Self, DatabaseError>;
    pub(crate) fn with_connection<T, E>(
        &mut self, operation: impl FnOnce(&rusqlite::Connection) -> Result<T, E>,
    ) -> Result<T, E> where E: From<DatabaseError>;
    pub(crate) fn try_close(&mut self) -> Result<(), DatabaseError>;
}
```

`Registration` owns pinned backing storage for `sqlite3_vfs`, its NUL-terminated unique name,
`DatabaseFiles`, the connection-local error latch and callback state. Name allocation uses an atomic
counter, checked for overflow. Registration is non-default and scoped to one database. The owner has no
bare-connection, raw-handle, statement, backup or blob export. Statements stay inside the borrowing
operation; the existing dependency features remain unchanged. Its constructor is
`Registration::new(root: RootDir, leaf: &Path, policy: AccessPolicy) -> Result<Self, DatabaseError>` and
`name(&self) -> &std::ffi::CStr` supplies the raw opener's explicit selection. Both remain private to the
database module. Lend only `&Connection`: lending `&mut Connection` would let a caller replace it and
extract the registered connection. The closure's output type cannot borrow from its connection argument.

For failure tests, define test-only `Fault::{Write, FileSync, DirectorySync, Truncate}`,
`Database::fail_next(&mut self, fault: Fault)` and `Database::short_write_next(&mut self, bytes: usize)`.
The next matching VPT operation returns `EIO` before performing it and disarms; the short-write case
writes the specified prefix before returning `EIO`. A test-only consumed flag proves the intended
operation was reached. Use one concrete `FaultState` slot in the connection's state. In `faults.rs`, its
non-test definition is a zero-sized no-op, followed by its test-only state and behavior. Put `cfg(test)`
declarations after production items so the size audit still counts production code. No public fault API,
production trait or global injection is needed.

The private opener separates registration from adoption with
`Database::open_registered(registration: Registration, busy: Duration) -> Result<Self, DatabaseError>`.
Production `open` constructs its registration then calls this method. Tests can arm a registration-local
fault before this call to cover failed open; `Registration::fail_next(&mut self, fault: Fault)` is
test-only. Keep that helper and `open_registered` visible only inside `database`.

- [ ] **Step 1: Add compiling red transaction and lifetime tests.**

```rust
#[test]
fn a_rollback_transaction_uses_only_authorized_files() {
    let (_temp, root) = root();
    let path = root.path().to_path_buf();
    let mut db = Database::open(root, Path::new("vpt.db"), AccessPolicy::WritableLedger,
        Duration::from_millis(20)).expect("open");
    db.with_connection::<_, DatabaseError>(|connection| {
        connection.execute_batch("CREATE TABLE item(value INTEGER); BEGIN IMMEDIATE;
            INSERT INTO item VALUES (7); ROLLBACK;")?;
        let count: i64 = connection.query_row("SELECT count(*) FROM item", [], |row| row.get(0))?;
        assert_eq!(count, 0);
        Ok(())
    }).unwrap();
    db.try_close().unwrap();
    let names: Vec<_> = std::fs::read_dir(path).unwrap()
        .map(|entry| entry.unwrap().file_name()).collect();
    assert_eq!(names, [std::ffi::OsString::from("vpt.db")]);
}

#[test]
fn an_attached_database_cannot_select_the_native_filesystem() {
    let (temp, root) = root();
    let outside = temp.path().join("outside.db");
    let mut db = Database::open(root, Path::new("vpt.db"), AccessPolicy::WritableLedger,
        Duration::from_millis(20)).unwrap();
    let uri = format!("file:{}?vfs=unix", outside.display());
    let result = db.with_connection::<_, DatabaseError>(|connection| {
        connection.execute("ATTACH DATABASE ?1 AS outside", [&uri])?;
        Ok(())
    });
    assert!(result.is_err());
    assert!(!outside.exists());
    db.try_close().unwrap();
}
```

Add separate leaves for: short read zero-filling; partial writes; invalid offsets; failed open after a
file was acquired; refusing disk temporary requests and unknown companion/open flags; directory
replacement during lazy journal access and final cleanup; independent connection error latches; and a
callback panic converted to failure without crossing the C boundary. Root-switch tests call VPT's
production callback/helper directly after a completed validation, without a global hook or third-party
system-call override.

For the lifetime regression, prepare a statement through a test-only raw handle and intentionally keep it
alive while calling `try_close`. Assert SQLite busy, the registration still discoverable by its internal
name, and the connection still usable after finalizing that statement. Retry close and assert
registration removal and released locks. A separate supervised child drops this deliberately invalid
owner without finalizing; it must terminate safely rather than free live callback storage.

- [ ] **Step 2: Run the selected owner tests red.**

```bash
cargo test --locked -p vpt-adapters database::tests::owner -- --nocapture
```

Expected: named behavior assertions fail against a compiling unsupported opener. Every blocking or
deliberately invalid-lifetime case runs under D2's independent watchdog.

- [ ] **Step 3: Implement registration, open and close ownership.**

Allocate registration storage before passing its address to `sqlite3_vfs_register`; retain stable
addresses until unregister completes. Use `iVersion = 2` for the filesystem and file-method tables;
database-page fetch/unfetch remain disabled. Files use an initialized `#[repr(C)]` header followed by a
pointer to an owned Rust file state. Set `pMethods` only after successful initialization. Failed opens
leave `pMethods` null and release any partially acquired state. Static method tables outlive every file.

Use raw `sqlite3_open_v2` with the selected registration name so failed-open handles stay owned. When it
succeeds, adopt the handle with `Connection::from_handle_owned`; the raw guard relinquishes ownership
only after adoption succeeds. On failure, close a non-null handle while registration remains alive, then
return the latched typed failure or the numeric SQLite error. Do not format SQLite's message text. Do not
call an unavailable `sqlite3_close_v2` binding or change the dependency to acquire one.

Check `sqlite3_threadsafe()` and `sqlite3_initialize()` before registration, fail closed on an unsafe
build or initialization failure, and keep the owner confined to one thread. VPT does not change SQLite's
process-wide threading configuration. Pass `SQLITE_OPEN_NOMUTEX` and `SQLITE_OPEN_EXRESCODE`, then check
extended-result setup and the supplied busy timeout. Set `temp_store=MEMORY` and `mmap_size=0`. Omit URI
opening, do not enable extension loading, and install an authorizer with `sqlite3_set_authorizer` that
denies `SQLITE_ATTACH`. This blocks an attachment that names another filesystem before its opener can
run. Unexpected disk temporary requests also fail at `xOpen`, regardless of later pragma changes.

Before any database work, disable checkpoint-on-close for all three policies whose main database is
read-only. The pinned pager otherwise attempts an exclusive lock during close, even on a read-only
descriptor. Check the result of this supported setting:

```rust
if policy != AccessPolicy::WritableLedger {
    connection.set_db_config(rusqlite::config::DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE, true)?;
}
```

Latch definitive containment, storage and durability failures. SQLite's expected busy, read-only mapping,
read-only-uninitialized and short-read/zero-fill protocol results are not latched faults; SQLite may
handle them and complete the operation successfully. Keep unexpected descriptor errors visible rather
than exempting all errors from a callback.

The owner uses this operation order:

```text
with_connection:
  refuse a closed owner
  revalidate the retained root; clear this operation's error latch
  lend the connection to the closure
  read the latch even if the closure returned success or mapped its SQL error
  prefer a latched containment/IO failure; otherwise retain the closure result
  revalidate before returning a successful result

try_close:
  take the Connection; keep Registration owned
  call Connection::close
  on any close failure: restore Connection and return the typed error
  on success: collect callback errors, unregister, release backing storage
  repeated close of an already closed owner succeeds
```

`Drop` attempts the same synchronous close. If that leaves a connection open, abort before freeing
registration storage. This detects an internal ownership-invariant violation, including a forgotten
statement; scoped Rust borrowing alone does not make `mem::forget` impossible. Ordinary query contention,
filesystem errors and invalid database contents are returned by operations. If close has completed but
its callbacks recorded an input/output error, the owner can release its storage normally; explicit
callers still receive that error. Apply the fail-stop rule to a raw failed-open guard whose handle cannot
be closed too. Disable core dumps in the dedicated invariant test child. Do not walk and finalize
arbitrary `sqlite3_next_stmt` results: bundled virtual-table modules can own those statements.

Every C callback checks counts, offsets, required non-null pointers and initialized state before use;
buffer validity otherwise follows SQLite's documented calling contract. Contain Rust unwinding with a
small `catch_unwind` boundary. On panic, latch an internal failure and poison further operations. Status
callbacks return their operation-specific SQLite error; count, pointer or void callbacks return their
contract's zero/null/no-value failure form. The outer owner still observes the latch. Cleanup releases
its owned state. `xClose` clears its file-state pointer once and drops that state once. Document these
pointer and lifetime invariants beside the unsafe operations.

- [ ] **Step 4: Implement file methods and explicit filesystem dispatch.**

Use one internal filename token per registration. `xFullPathname` copies the token with a checked output
length; `xOpen`, `xAccess` and `xDelete` accept only that token and its three known suffixes with
matching roles. Neither the token nor SQLite's incoming filename reaches a native pathname opener. Null
temporary names, super-journals, attached databases and unrecognized mode bits are refused.

| Callback                                 | Implementation                                                                                                                                            |
| ---------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `xRead`                                  | Checked nonnegative offset and length; `FileExt::read_at` loop; retry interrupted reads; zero any unread tail and return `SQLITE_IOERR_SHORT_READ` on EOF |
| `xWrite`                                 | Refuse non-writable role; `FileExt::write_at` loop with checked offset advancement; zero progress is `WriteZero`; propagate failure                       |
| `xTruncate`, `xFileSize`                 | Descriptor `set_len`/metadata, checked integer conversion, policy enforcement before truncation                                                           |
| `xSync`                                  | Honor data-only versus full sync; honor full durability with macOS `F_FULLFSYNC` when requested; propagate failure, including pending directory sync      |
| `xLock`, `xUnlock`, `xCheckReservedLock` | D2's protocol; operation-specific SQLite errors for operating-system failures                                                                             |
| `xFileControl`                           | Implement lock state, persistent WAL flag and moved-file detection; return `SQLITE_NOTFOUND` for unsupported controls                                     |
| `xSectorSize`, `xDeviceCharacteristics`  | Use the pinned Unix default of 4096 bytes; advertise zero optional device capabilities                                                                    |
| `xDelete`                                | D1's exact-role removal and requested directory sync; writable ledger only                                                                                |
| `xAccess`                                | Descriptor-relative no-follow metadata plus policy-aware existence/read/write answer; refuse unsafe entries                                               |
| `xRandomness`, `xSleep`, time callbacks  | Delegate only these path-free services to the installed native implementation                                                                             |
| Dynamic loading callbacks                | Unavailable; no library pathname opening                                                                                                                  |

Set the persistent WAL file-control flag on writable ledgers before WAL initialization. Support querying
it and changing it through the documented integer argument. A moved-file check compares the open file
identity with its descriptor-relative directory entry and reports substitution without following it.

- [ ] **Step 5: Verify green, cleanup and mutations.**

Run the owner test command, then all `database` tests. Inject write and file/directory synchronization
failures only at VPT's own operation boundary using connection-local test state; no global injection.
Assert the outer operation returns failure. Mutations: native pathname delegation, ignored sync result,
missing attach denial, lost error latch and unregister-before-close. The lifetime mutation runs in a
supervised child so a bad foreign pointer cannot crash or hang the whole suite. Restore exact bytes and
run each control.

- [ ] **Step 6: Format, review and commit.**

```bash
cargo fmt --all
cargo clippy --locked -p vpt-adapters --all-targets -- -D warnings
cargo test --locked -p vpt-adapters database
git add crates/vpt-adapters/src/database
git commit -m "feat(database): own contained SQLite connections"
```

## D4: Shared memory, WAL concurrency and recovery

**Files:** Create `database/shared_memory.rs`, `database/tests/wal.rs` and `database/tests/failures.rs`;
modify `database/io.rs`, `database/registration.rs`, `database/locks.rs`, `database/faults.rs`,
`database/tests/children.rs` and test support.

**Interfaces:** A private `SharedMemory` owns its independently opened companion file, initialization
lock, mappings, retained directory authority and held slot locks.
`SharedMemory::open(files: &DatabaseFiles) -> Result<Self, DatabaseError>` acquires these through D1's
policy. Its `map(region: i32, size: i32, extend: bool) -> Result<MappedRegion, DatabaseError>` returns
one of these explicit outcomes:

```rust
enum Initialization { Acquired, ReadOnlyUninitialized }
enum MappedRegion {
    Missing,
    ReadOnlyUninitialized,
    ReadOnly(std::ptr::NonNull<std::ffi::c_void>),
    Writable(std::ptr::NonNull<std::ffi::c_void>),
}
```

`Initialization` is retained with the shared-memory file. A read-only uninitialized file is a successful
open of that state, not an error that drops the descriptor.
`lock(offset: i32, count: i32, flags: i32) -> Result<(), DatabaseError>` checks SQLite's shared-memory
flags; `barrier()` supplies the required memory ordering;
`unmap(delete: bool) -> Result<(), DatabaseError>` releases mappings and locks, then applies policy and
persistence rules to any deletion. `io.rs` translates `Missing` to null/success, `ReadOnlyUninitialized`
to null/`SQLITE_READONLY_CANTINIT`, `ReadOnly` to the address/`SQLITE_READONLY`, and `Writable` to the
address/success. These protocol outcomes do not poison the error latch.

- [ ] **Step 1: Write compiling red snapshot tests.**

```rust
#[test]
fn a_reader_keeps_its_snapshot_while_a_writer_commits() {
    let (_temp, root) = root();
    let mut writer = Database::open(root.try_clone().unwrap(), Path::new("vpt.db"),
        AccessPolicy::WritableLedger, Duration::from_millis(20)).unwrap();
    writer.with_connection::<_, DatabaseError>(|c| {
        c.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE item(value INTEGER);
            INSERT INTO item VALUES (1);")?;
        Ok(())
    }).unwrap();
    let mut reader = Database::open(root, Path::new("vpt.db"),
        AccessPolicy::ObservationalLedger, Duration::from_millis(20)).unwrap();
    reader.with_connection::<_, DatabaseError>(|c| {
        c.execute_batch("BEGIN")?;
        let first: i64 = c.query_row("SELECT sum(value) FROM item", [], |r| r.get(0))?;
        assert_eq!(first, 1);
        writer.with_connection::<_, DatabaseError>(|w| {
            w.execute("INSERT INTO item VALUES (2)", [])?;
            Ok(())
        })?;
        let pinned: i64 = c.query_row("SELECT sum(value) FROM item", [], |r| r.get(0))?;
        assert_eq!(pinned, 1);
        c.execute_batch("COMMIT")?;
        let latest: i64 = c.query_row("SELECT sum(value) FROM item", [], |r| r.get(0))?;
        assert_eq!(latest, 3);
        Ok(())
    }).unwrap();
    reader.try_close().unwrap();
    writer.try_close().unwrap();
}
```

Extend test support with `snapshot(root: &Path) -> BTreeMap<OsString, FileSnapshot>`. `FileSnapshot`
derives `Debug, PartialEq, Eq` and records bytes, entry kind, device/inode, mode, link count, size,
mtime/nanoseconds, ctime/nanoseconds and flags. Include directory metadata and entries; exclude atime.
Use descriptor-safe reads of the synthetic tree, with writers quiescent around each comparison. Run a
readonly query and close between snapshots after the last writer has closed, with the writer still alive,
with missing companions, and with recovery required. Each gets its own leaf.

Extend D3's connection-local faults with `Fault::Map`, returning `EIO` before a mapping. Keep all
injection at VPT's own operation boundary; do not change SQLite or its system-call table.

- [ ] **Step 2: Run WAL tests red.**

```bash
cargo test --locked -p vpt-adapters database::tests::wal -- --nocapture
cargo test --locked -p vpt-adapters database::tests::failures -- --nocapture
```

Expected: WAL operations fail with the unimplemented shared-memory methods, not a compile error. All
contention/crash scenarios use the already-green process watchdog.

- [ ] **Step 3: Implement shared-memory initialization, mapping and locks.**

```rust
const SHM_LOCK_BASE: i64 = 120;
const SHM_LOCK_COUNT: i32 = 8;
const SHM_LIFETIME_BYTE: i64 = 128;
const SHM_REGION_BYTES: usize = 32 * 1024;
```

Use descriptor-owned locks on these native-compatible bytes. Slots 0, 1 and 2 coordinate writer,
checkpoint and recovery; slots 3 through 7 protect readers. Shared locks cover one slot; exclusive locks
may cover a checked contiguous range. Reject invalid flag combinations, ranges and upgrades that SQLite's
contract does not allow. The shared-memory descriptor must be separate from the database descriptor and
remain open for the mappings' lifetime.

Initialization follows the pinned native protocol:

1. Inspect byte 128 for a conflicting exclusive holder. That holder means busy.
1. With an existing shared lifetime holder, acquire a shared lifetime lock and use its initialized file.
1. With no lifetime holder and writable shared memory, acquire exclusive byte 128, invalidate stale
   shared memory by truncating to the pinned implementation's three-byte initialization length, then
   downgrade to shared. Recheck after a raced acquisition; do not truncate under another holder.
1. With no lifetime holder and read-only shared memory, retain the descriptor without a lifetime lock as
   `ReadOnlyUninitialized`. Mapping reports `SQLITE_READONLY_CANTINIT`, allowing SQLite's heap-index
   recovery. Later mapping calls retry initialization and transition to `Acquired` only after a
   successful shared lifetime lock. Slot locking remains available through the retained descriptor
   without requiring that transition. Never create a fake initialized state or repair the file.

Map regions with checked arithmetic and host page alignment. Provision a writable region before mapping
so reads cannot fault beyond the file. Use `PROT_READ` for observational policies and add `PROT_WRITE`
only where the policy permits. A missing region with `extend=false` returns null and success. Once a
connection reports read-only shared memory, keep that mode until unmap. Honor the requested region size;
reject unsupported sizes rather than silently mapping a different layout. Implement the cross-process
synchronization barrier with the platform memory fence required by SQLite, then release locks and
mappings in reverse acquisition order on every error path.

Persistent WAL preserves companion entries on close; it does not prohibit writable checkpointing or
truncation. SQLite owns transaction, reader-mark and checkpoint decisions. The adapter enforces access
policy on every write/truncate/map/delete so an observational connection cannot perform recovery that
requires writes.

- [ ] **Step 4: Add native interoperability, crash and durability assertions.**

Use child scenarios `native_writer`, `native_reader`, `rooted_uncommitted` and `rooted_committed` in
`tests/children.rs`. They select one synthetic database, announce a completed SQL milestone through the
pipe, then wait for a command. Test both native-to-rooted and rooted-to-native reads, WAL snapshots and
writer contention. Terminate the two rooted crash scenarios only after their milestone; writable reopen
must respectively exclude the uncommitted row and retain the committed row. No sleep determines when the
transaction is ready.

For write, partial-write and file-sync faults, record rows before the transaction, inject at the named
operation, attempt a two-row transaction, assert failure, close and reopen writable, then assert that no
partial pair exists. Trigger directory-sync failure on a newly created rollback journal, truncate failure
during journal finalization, and map failure during the first WAL read. Assert the fault was consumed in
every case; a dormant fault is not evidence. A failed commit may be absent or fully present according to
SQLite's documented commit point; it must never be reported successful after the injected failure. Check
database integrity and successful subsequent transactions. Test interrupted rollback-journal recovery as
well as WAL.

Add individual shared-memory leaves for lifetime initialization racing another opener, read-only
initialization refusal, missing region without extension, multi-region mapping, map failure cleanup, slot
contention and connection-close lock retention. Test busy waits with a 20 ms implementation timeout and
the independent 500 ms watchdog.

Pin the uninitialized read-only path with committed frames still in the WAL after its last writer closes:
retain slot-lock functionality, read through SQLite's heap index, and leave the snapshot unchanged.
Mutate dropping that retained state to prove the case matters. Mutate the read-only checkpoint-on-close
setting and require the normal successful-close test to catch the unwanted probe.

- [ ] **Step 5: Verify green, mutations and independent review.**

Mutate one shared-memory lock offset, enable write protection on an observational map, suppress a sync
failure, remove persistent companions, and release another connection's lock. The matching leaves must
fail. Repeat the busy-bound hang control through a real WAL transaction. Restore exact bytes and run each
control. Request independent review of byte ranges, initialization, mapped-memory lifetime and close
cleanup before continuing to D5.

```bash
cargo fmt --all
cargo test --locked -p vpt-adapters database
cargo clippy --locked -p vpt-adapters --all-targets -- -D warnings
git add crates/vpt-adapters/src/database
git commit -m "feat(database): support contained concurrent WAL access"
```

## D5: Ledger integration and observational behavior

**Files:** Original Task 11's application error declaration, port exports, ledger module, schema,
connection and boundary tests; adapters `lib.rs`. Preserve the schema SQL and `SCHEMA_VERSION = 1` from
that task exactly. D1 through D4 replace its opening mechanism, not its tables or migration contract. Add
only test-configuration exports of `root` and `snapshot` at the end of `database/mod.rs` so the ledger
and title tests can reuse crate-owned fixtures. The production module stays private.

**Interfaces:** Keep all original Task 11 public signatures, including `OpenError`, `SqliteLedger`,
`BUSY_TIMEOUT`, `open`, `open_with_timeout`, `open_read_only`, `database_path`, `schema_version`, `read`
and `transaction`. Private
`connection::open(state: &RootDir, busy: Duration, policy: AccessPolicy) -> Result<Database, LedgerError>`
returns the retained owner. Implement `From<DatabaseError> for LedgerError`. Keep
`map(rusqlite::Error) -> LedgerError` for repository closures; it uses error codes and fixed safe
messages, never raw SQLite error text. `database_path()` remains diagnostic only. Change private
`migrations::migrate` to accept `&Connection`; it and `SqliteLedger::transaction` create their immediate
transaction with `rusqlite::Transaction::new_unchecked(connection, TransactionBehavior::Immediate)`. The
wrapper admits one operation at a time and SQLite rejects nested transactions. This keeps the connection
borrow immutable without changing the application contract.

- [ ] **Step 1: Write compiling red ledger tests.**

```rust
#[test]
fn observation_after_last_writer_close_changes_nothing() {
    let (_temp, root) = root();
    let ledger = SqliteLedger::open(&root).unwrap();
    assert_eq!(ledger.schema_version().unwrap(), SCHEMA_VERSION);
    drop(ledger);
    let before = snapshot(root.path());
    let observed = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
    assert_eq!(observed.schema_version().unwrap(), SCHEMA_VERSION);
    drop(observed);
    assert_eq!(snapshot(root.path()), before);
}

#[test]
fn populated_ledger_with_missing_companion_is_not_repaired() {
    let (_temp, root) = root();
    drop(SqliteLedger::open(&root).unwrap());
    root.unlink_file(Path::new("vpt.db-shm")).unwrap();
    let before = snapshot(root.path());
    assert!(SqliteLedger::open_read_only(root.path()).is_err());
    assert_eq!(snapshot(root.path()), before);
}
```

Reuse the crate-owned snapshot/fixture helpers through test-only curated exports, not a public production
module. Add original migration, future-version, rollback, privacy, busy and foreign-key tests with rooted
connection owners. Add separate absent state, absent database, empty file and zero-schema tests returning
`None` without changes. Root replacement after constructing a ledger must yield `PathEscape`, including
if the old pathname is now missing. Inject a containment failure during a query and verify it survives a
repository closure that already mapped the SQLite result.

- [ ] **Step 2: Run red.**

```bash
cargo test --locked -p vpt-adapters ledger::sqlite -- --nocapture
```

Expected: new tests compile and fail on the missing behavior. They must not accidentally call the old
native pathname opener.

- [ ] **Step 3: Compose operations through the retained owner.**

Writable open creates/repairs private state files through D1, opens with `WritableLedger`, enables
foreign keys and WAL, and runs original migrations in an immediate transaction. Observational open uses
`ObservationalLedger`, never writes pragmas that change journal format, and never migrates. Check version
through that connection; version zero yields `None`. Require existing regular WAL and shared-memory
companions for a populated ledger. Missing or zero-schema state retains the original empty behavior even
when companions do not yet exist. The initial inspection must itself obey the read-only policy.

Wrap each read/transaction in `Database::with_connection`, including migrations. Close explicitly after
the closure returns, on both success and failure. Preserve the original operation error when ordinary
cleanup also fails; a containment failure takes precedence so it cannot be disguised. Closing must finish
before registration is released. Use this combination rule in one private helper:

```rust
fn finish<T>(result: Result<T, LedgerError>, closed: Result<(), LedgerError>) -> Result<T, LedgerError> {
    match (result, closed) {
        (_, Err(error @ LedgerError::PathEscape(_))) => Err(error),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
        (Ok(value), Ok(())) => Ok(value),
    }
}
```

`transaction` retains the original immediate transaction and read-only refusal. Revalidate before commit;
rollback and close still use the acquired authority if that validation fails. Map containment to
`PathEscape`, busy/locked to `Busy`, constraints to `Conflict`, corruption/not-a-database to `Corrupt`,
and other storage failures to `Io`. Safe messages can name the operation and configured path but cannot
interpolate titles, SQL values or database content. Keep the original command exit mapping.

- [ ] **Step 4: Verify green and repository compatibility.**

Run the ledger tests and all database tests. Mutate policy selection to writable for observation, discard
the connection error latch, skip foreign keys, and bypass migration rollback one at a time. Each
corresponding behavioral test must fail. Restore controls. During original Task 12, run the same contract
against `MemoryLedger` and this durable ledger; retain the preflight's signed-integer, time and
path-encoding rejection cases. Repeat that shared contract when Task 31 adds retention repositories.

- [ ] **Step 5: Format, review and commit.**

```bash
cargo fmt --all
cargo test --locked -p vpt-adapters
git add crates/vpt-application/src/ports/ledger.rs crates/vpt-application/src/ports/mod.rs
git add crates/vpt-adapters/src/ledger crates/vpt-adapters/src/lib.rs
git add crates/vpt-adapters/src/database/mod.rs
git commit -m "feat(ledger): use contained database connections"
```

## D6: Private title-copy leases and contained title reads

**Files:** Original Task 16's recorder port additions, exports, `voice_memos/store.rs`,
`voice_memos/titles.rs` and its tests; create `voice_memos/titles/lease.rs`. Extend the existing
`database/tests/children.rs` and `database/tests/support.rs` for actual title-reader scenarios.

**Interfaces:** Keep `RecorderStore::refresh_titles(&self) -> Result<(), RecorderError>` and
`title(&self, file_name: &str) -> TitleLookup` unchanged. `TitleCopy` holds an optional
`RefCell<Database>`, discovered table and lease. Replace private `open_copy` with
`open_copy(root: RootDir, policy: AccessPolicy, lease: CopyLease) -> Result<TitleCopy, DatabaseError>`.
Change adapter convenience methods `refresh(container: &Path, state_dir: &Path)` and
`existing(state_dir: &Path)` to return `Result<TitleCopy, ContainedError>` so callers must classify
containment; ordinary copy/schema/query failures still produce an unavailable copy. Keep
`refresh_at(container: &Path, state: &RootDir) -> Result<TitleCopy, ContainedError>`.

`CopyLease::read(copy: &RootDir, timeout: Duration) -> Result<Self, DatabaseError>` opens the existing
main copy read-only. `CopyLease::refresh(copy: &RootDir, timeout: Duration)` opens or exclusively creates
it read/write, without truncating or repairing permissions before the exclusive lease is held.
`downgrade(&mut self) -> Result<(), DatabaseError>` converts the exclusive lock to shared. The lease
holds an independently opened file description for byte 0, length 1, and stays alive until after the
title database closes. It is never a duplicate of SQLite's description. D2 supplies the lock primitive.

- [ ] **Step 1: Write compiling red title and lease tests.**

```rust
#[test]
fn an_existing_title_reader_prevents_refresh_truncation() {
    let (_temp, root) = root();
    let copy = root.subdirectory(COPY_DIRECTORY).unwrap();
    std::fs::write(copy.path().join("CloudRecordings.db"), b"unchanged").unwrap();
    let before = snapshot(copy.path());
    let reader = CopyLease::read(&copy, Duration::from_millis(20)).unwrap();
    assert!(CopyLease::refresh(&copy, Duration::from_millis(20)).is_err());
    assert_eq!(snapshot(copy.path()), before);
    drop(reader);
    assert!(CopyLease::refresh(&copy, Duration::from_millis(20)).is_ok());
}
```

Preserve original Task 16's concrete title fixtures and SQL assertions, adapting calls to the typed
result. Verify a committed title that exists only in the copied WAL, unchanged source snapshots,
missing/empty titles, escaped SQL filename patterns, unexpected schema, absent source companions and copy
failure. Add separate refreshed-copy and existing-copy snapshots: the former may change private shared
memory but not copied database/WAL bytes; the latter changes nothing. Assert dry run neither opens nor
refreshes title databases.

Add a child reader holding an actual `TitleCopy`, then attempt refresh in another process. Assert busy or
bounded unavailable without truncation, close the reader, and assert refresh succeeds. While the title
lease is held, query a separate ledger successfully. The timeout must be covered by the existing
watchdog, including a mutation that removes the refresh deadline.

- [ ] **Step 2: Run red.**

```bash
cargo test --locked -p vpt-adapters voice_memos::titles -- --nocapture
```

Expected: the selected tests run and fail against the missing lease/owner behavior. Compilation errors
from changed convenience signatures must be resolved before recording red evidence.

- [ ] **Step 3: Implement the lease and refresh order.**

Use a monotonic deadline and nonblocking `set_record_lock` attempts. Retry contention only until the
supplied deadline; other failures return immediately. A bounded sleep between attempts is permitted in
production retry handling, not as test synchronization. All production callers supply five seconds. Byte
0 is outside SQLite's database locking range; the separate description keeps the lease independent of
SQLite's unlock-all operation. This lease coordinates VPT copy readers and refreshers only.

```text
refresh_titles:
  remove and drop the previous TitleCopy before acquiring a refresh lease
  revalidate source and checked private state roots
  acquire exclusive CopyLease before any destination truncate, chmod or copy
  open every source through Access::Read
  copy the three fixed files through destination descriptors
  leave a missing source companion represented by the existing empty-copy rule
  on any copy failure, keep this sweep's titles unavailable and do not reopen
  after the complete copy succeeds, open with RefreshedTitleCopy policy
  discover the title table and initialize any needed shared memory while still exclusive
  downgrade the lease to shared and retain it with the owner

existing:
  open existing private root without creating or repairing it
  acquire shared CopyLease before opening SQLite
  open with ExistingTitleCopy policy; retain lease through final close
```

The lease's main descriptor and the descriptor used for main-file copying must identify the same file.
Use the retained lease file for truncation/copy, or verify identity when a second descriptor is needed.
Do not repair its mode before the lease is acquired. Refuse unsafe entries and multiple links on every
destination acquisition. Permission repair is confined to the exclusive refresh phase.

Store the owner in `RefCell` because the existing `title(&self, ...)` port borrows immutably. Execute the
original parameterized title query inside `with_connection`, with the original table-name quoting and
LIKE escaping. Match ordinary query/schema failures to `Unavailable` only after checking the typed error.
`refresh_at` propagates containment; the store maps it to `RecorderError::Escape`. Existing-copy callers
explicitly choose their documented observation outcome rather than silently calling `.ok()` on the shared
opener. Keep application port types unchanged.

Implement `Drop` for `TitleCopy` so the database is closed/dropped before the copy lease. Field order
alone must not release a lease while a dependent connection remains. Root substitution during close
retains the same acquired descriptors and never switches to a new copy directory.

- [ ] **Step 4: Verify green and lease/error mutations.**

Run title and database tests. Mutate removal of the old owner before refresh, the exclusive lease, lease
lifetime, the existing-copy policy and typed-containment propagation individually. Each relevant leaf
must fail; restore exact bytes and controls. Test both same-process VPT readers and separate process
readers. Native source-fixture SQLite may stay open because its inode differs from the private copy's
inode; snapshot its source only after its fixture writer is quiescent.

- [ ] **Step 5: Format, review and commit.**

```bash
cargo fmt --all
cargo test --locked -p vpt-adapters voice_memos
git add crates/vpt-application/src/ports/recorder.rs crates/vpt-application/src/ports/mod.rs
git add crates/vpt-adapters/src/voice_memos crates/vpt-adapters/src/lib.rs
git add crates/vpt-adapters/src/database/tests
git commit -m "feat(titles): protect private database refresh and reads"
```

## D7: Command integration and final evidence

Execute these steps with the owning original tasks, after their non-database prerequisites are ready. Do
not add stub commands early solely to finish this checklist.

**Files:** `crates/vpt/src/compose/ledger.rs`, `compose/runtime.rs`, `compose/observations.rs`,
`compose/errors.rs`, `compose/tests.rs`, `commands/ingest.rs`, `commands/show.rs`, `commands/list.rs`,
`commands/retention.rs`, `doctor/probes.rs`; existing command tests `crates/vpt/tests/ingest.rs`,
`show_list_storage.rs`, `doctor.rs`, `retention.rs` and `retention_startup.rs`; adapt the existing
`tests/support/mod.rs` only where a bounded fixture needs it.

**Interfaces:** Consume the unchanged ledger/recorder ports and D5/D6's typed adapters. Keep existing
command documents, refusal rule tokens, output streams and exit codes. Mutating commands alone acquire
the existing application `write.lock`; observations never acquire it.

Original Task 27's test-only `Sandbox::ledger() -> rusqlite::Connection` must preserve native WAL
companions too, or fixture updates would remove files that observation correctly requires. Before
returning that native fixture connection, call this private helper in the same test support file:

```rust
fn preserve_wal(connection: &rusqlite::Connection) {
    let mut enabled: std::ffi::c_int = 1;
    // SAFETY: the connection and integer remain valid throughout the call.
    let result = unsafe {
        rusqlite::ffi::sqlite3_file_control(
            connection.handle(), c"main".as_ptr(), rusqlite::ffi::SQLITE_FCNTL_PERSIST_WAL,
            (&mut enabled as *mut std::ffi::c_int).cast(),
        )
    };
    assert_eq!(result, rusqlite::ffi::SQLITE_OK);
}
```

Close fixture writers before snapshot comparisons. They may overlap a VPT command subprocess for the
explicit concurrency cases, but may not overlap a rooted connection in the fixture's own process.

- [ ] **Step 1: Add compiling command-level red regressions in the owning task.**

Use the existing `Sandbox` and bounded `run` helper. The acceptance table is the assertion contract:

Add this leaf to original Task 28's `show_list_storage.rs`. Its `ingested` fixture is defined in that
task. Extend `support::state_snapshot` to include entry kinds, device/inode, link count, size, ctime and
flags alongside its existing bytes, modes and mtime fields; exclude atime. Keep the snapshot code inside
the command crate's own test support.

```rust
#[test]
fn list_after_the_ingest_writer_exits_changes_no_state() {
    let (sandbox, id) = ingested("list-after-writer-exit");
    let before = support::state_snapshot(&sandbox);
    let output = run(sandbox.vpt().args(["list", "--json"]));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(output.stderr.is_empty());
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["command"], "list");
    assert_eq!(result["recordings"][0]["id"], id);
    assert_eq!(support::state_snapshot(&sandbox), before);
}
```

| Command scenario                                      | Assertions                                                                                                |
| ----------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| `show`, `list`, `storage`, read-only `doctor`         | Existing command result/verdict; no state creation, permission repair or database metadata changes        |
| Ingest dry run                                        | No title database open/refresh; no ledger or source writes                                                |
| `show`/`list` with populated ledger missing companion | Exit 1, empty stdout in JSON mode, safe error on stderr; unchanged tree                                   |
| Root/companion containment refusal                    | Exit 3, `path_escape` rule, empty JSON stdout, unchanged replacement/source tree                          |
| Another process holds a database write transaction    | Observations still read committed state where SQLite permits; mutating contention remains bounded         |
| Another command holds application `write.lock`        | Read-only commands never wait on that application lock                                                    |
| Failed storage/sync/constraint/corruption operation   | Distinct ledger classification reaches the established exit-1 error document without titles or row values |
| Retention repositories                                | In-memory and durable implementations pass the same contract before command acceptance                    |

Preserve doctor's aggregate contract: it runs all 18 checks and reports failures with exit 3 and
`doctor_checks`. A ledger containment or storage error must appear in the relevant check without turning
doctor into an early-exit ledger command. Storage remains stores-only and never opens a ledger. For
errors that need D3/D4's private fault injection, command composition unit tests feed the resulting typed
`LedgerError` through the existing converter and emitter; do not add production environment variables or
fault controls to the command binary. Add tests to the command suite using the original task's concrete
fixture builders and assertions. For JSON refusals, preserve the already implemented `vpt.error/1` parser
and assert the whole envelope, not a substring of human text. Reuse D4's native child pattern when a
second process must hold a SQLite transaction; a test may not keep a native and rooted connection on the
same inode in one process.

- [ ] **Step 2: Run each new named test red before routing changes.**

```bash
cargo test --locked -p vpt --features dev-tools --test show_list_storage
cargo test --locked -p vpt --features dev-tools --test doctor
cargo test --locked -p vpt --features dev-tools --test ingest
cargo test --locked -p vpt --features dev-tools --test retention
cargo test --locked -p vpt --features dev-tools --test retention_startup
```

Run only test targets already introduced by their original task. Record the exact discovered leaf and its
intended failed assertion; a missing target is not red evidence.

- [ ] **Step 3: Route composition through the strengthened adapter.**

```text
mutating runtime -> checked state -> application write.lock -> SqliteLedger::open
observational runtime -> SqliteLedger::open_read_only -> durable or empty memory ledger
title refresh -> checked private state -> TitleCopy::refresh_at
existing title observation -> typed TitleCopy::existing result
dry-run runtime -> no title-copy opener
```

Remove every production native database opener outside `database/owner.rs`. Keep native fixture opens
only under test configuration. Adapt actual `TitleCopy` result callers explicitly; do not introduce
title-database work into doctor or dry run. Containment becomes `path_escape` where a command reports the
operation, optional title unavailability remains optional, and doctor keeps its aggregate check contract.
No command should derive a writable path from `database_path()`.

- [ ] **Step 4: Verify green and the full Stage 1 gates.**

Run each command target and the shared ledger contracts. Mutate the common observation composition to
writable and the shared error conversion to exit 1 for containment. The acceptance tests must fail;
restore controls. Search all production connection construction and descriptor/path operations for a
missed route. This source inspection supplements the tests, it does not replace them.

```bash
just test-rust
just lint-check
just ship
cargo check --locked --workspace --all-targets --features dev-tools
```

Measure every new test leaf under the workspace run, including parallel contention. Every leaf must
finish below one second. Run the clean-code Rust file-size command after staging owned files so new files
are included, and verify the command entry point is below 150 lines. Do not claim an installed binary,
live Voice Memos processing or notifications from these sandboxed tests.

- [ ] **Step 5: Commit with the owning command tasks and request whole-branch review.**

Keep logically distinct original tasks in separate commits. Request the established independent
whole-branch review with explicit attention to callback storage lifetime, shared-memory initialization,
native lock interoperability, read-only snapshots, source isolation and failure cleanup. Supply red,
green, mutation/control, timing and full-gate evidence. Resolve findings and rerun affected tests before
calling the Stage 1 build complete. No push, pull request or installation is part of this plan.

## Acceptance coverage and plan self-review

| Approved requirement                                           | Implementation and evidence                                      |
| -------------------------------------------------------------- | ---------------------------------------------------------------- |
| Root and ancestor replacement; all companions and cleanup      | D1 checked operations, D3 callback route and lifetime tests      |
| Exact names/roles, no attachment or disk temporary fallback    | D1 policy, D3 filename/authorizer tests                          |
| Private permissions, no-follow/type/link checks                | D1 descriptor tests, D6 exclusive refresh checks                 |
| Stable registration, failed open, busy close, no unwind        | D3 owner and supervised callback/lifetime tests                  |
| Database locks, native interoperability, independent close     | D2 lock transitions and D4 separate-process WAL tests            |
| Shared-memory initialization, barrier, mapping and recovery    | D4 protocol and read-only/recovery leaves                        |
| Write/sync/crash failures and durability                       | D3 error propagation, D4 failure injection and crash recovery    |
| Four access policies, five-second production timeout           | D1 policy table, D5 ledger, D6 title integration                 |
| Absent/zero-schema state; populated missing companions         | D5 empty-state and observational snapshot tests                  |
| Source read-only, WAL-only title, dry run, title refresh lease | D6 title/source snapshots and D7 command acceptance              |
| Typed containment, distinct ledger errors, safe diagnostics    | D3 local latch, D5 mapping, D7 output/exit assertions            |
| Existing application contracts and schema                      | D5 original migration contract, Tasks 12/31 shared contract      |
| Test-first, mutations, bounded children, file sizes and review | Every task's red/green/control steps, D2 watchdog, D7 full gates |

Before handing this plan to the operator, the controller checks every spec section against this table,
scans for placeholders, checks signatures across tasks and verifies all five Review Focus cases have
owning tests. The plan is not runtime evidence. Product implementation waits for the operator's plan
review, then resumes with the existing implementer/reviewer workflow.
