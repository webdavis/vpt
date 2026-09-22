# VPT database containment amendment

Status: approach and written amendment approved by the operator on 2026-09-22.

This amends sections 4.1, 4.4 and 5.1 of the [VPT design](2026-09-20-vpt-design.md) and replaces the
database-opening approach in Tasks 11 and 16 of the
[Stage 1 plan](../plans/2026-09-21-vpt-stage-1-extract-plan.md). Other requirements remain in force.

## Outcome

VPT keeps database access attached to the storage directory it opened and validated. Replacing that
directory's pathname must not redirect a later database or journal operation. Reads can continue while a
writer commits, subject to SQLite's ordinary contention and recovery rules.

Use a VPT-owned adapter through SQLite's supported filesystem interface. SQLite and `rusqlite` remain
unmodified. Keep the ordinary database format, write-ahead log (WAL), five-second busy timeout, read-only
commands and existing command output. There is no new daemon, configuration key or package. SQLite
documents this extension point in its
[operating-system interface](https://www.sqlite.org/c3ref/vfs.html).

## Containment contract

The authority is an open directory descriptor obtained through VPT's checked-root adapter. A pathname is
a diagnostic label. The connection and all its database files retain the directory authority through
queries, transactions, rollback, checkpointing and final close.

Every database-related open, creation, metadata check, permission change, synchronization and removal
uses that directory descriptor or an already-open file. Symbolic links and non-regular database files are
refused at the operation boundary. Directory replacement between a check and an operation cannot cause
the operation to follow the replacement. Normal root revalidation still detects a changed path and stops
new work; it is not the mechanism that confines access.

The guarantee covers replacement or redirection of the root pathname and its ancestors after the root has
been acquired. It does not make files tamper-proof against a process that can alter their contents,
change the mount, control VPT's process or move the original directory itself into another tree. The
root's identity remains the same after a rename. Existing checks that exclude the Voice Memos container
remain mandatory before acquiring writable authority.

A refused operation does not undo earlier committed work. Work already completed through retained handles
stays in the original directory. VPT never switches to a replacement directory to finish a transaction or
clean up after an error.

## Adapter boundary

One private module under `vpt-adapters` owns the SQLite filesystem implementation and a concrete
connection owner. The ledger and private title-copy reader use the same boundary. Domain, application and
protocol crates keep their existing responsibilities and gain no SQLite or filesystem dependency.

The opener takes a retained checked directory, a validated database leaf, an explicit access policy and a
busy timeout. It returns an owner that keeps the connection, filesystem registration and directory
authority alive together. Callers borrow the connection for queries or transactions; they cannot extract
a bare connection that outlives its authority. A saved `database_path()` is diagnostic only.

Responsibilities within that module are registration and callback lifetime, contained file operations,
database locking, and shared-memory mapping and locking. Use concrete types and a closed access-policy
enum. The existing checked-root operations supply directory-relative access; extend those operations only
where the database adapter needs a missing capability.

Register the filesystem by an internal name and select it explicitly for each VPT connection. It must not
become the process-wide default or replace SQLite's system-call hooks. Registration state, callback
tables and their backing storage remain valid until SQLite has released every dependent object, including
objects retained during a failed open or deferred close. Unregister before freeing storage. Confine
unsafe code to these adapter responsibilities, document pointer and ownership invariants, and prevent
unwinding across the C callback boundary.

### Files and temporary storage

Each registration authorizes one database and its exact `-journal`, `-wal` and `-shm` companion names.
Validate names and requested file roles before any filesystem operation. Reject traversal, another
database, an unknown companion or an unrecognized open mode. SQLite-facing names are internal identifiers
mapped to this allowlist; they are never passed to an ordinary pathname opener.

Create private directories at `0700` and persistent files at `0600`. Check opened descriptors for the
required file type before reading, writing, truncating or mapping them. Preserve exclusive creation,
close-on-exec and nonblocking type checks. Apply permission repairs through writable file descriptors
only. A pre-existing multiply linked database or companion file is refused.

Use memory for SQLite temporary storage. Refuse an unexpected request for a disk-backed temporary file or
an additional attached database. No fallback may use the current directory, an environment-selected
temporary directory or the ordinary SQLite filesystem. Normal Stage 1 transactions, including rollback
and migrations, must work under this policy.

The filesystem implements the required database and shared-memory file methods. Time, sleep and
randomness may use the installed platform implementation because they do not choose database paths.
Optional database-page mapping is disabled initially; shared-memory mapping remains required. SQLite's
[file-method contract](https://www.sqlite.org/c3ref/io_methods.html) defines those separate interfaces.

### Locking and durability

Keep normal WAL locking. Database lock transitions, shared-memory lock slots, initialization and recovery
must interoperate with the bundled native SQLite implementation on the same files. Use macOS
descriptor-owned record locks with the compatible byte ranges. Each open file description owns its locks,
so closing another connection must not release them. Lock attempts are nonblocking; SQLite's bounded busy
handling controls retries. Unsupported locking fails closed.

Map shared memory through a file opened relative to the held directory. Preserve the synchronization
barrier and read/write protections required by the connection's policy. File control must honor
persistent WAL companions. Initialization, truncation and final-close behavior must follow the same
locking protocol as active readers and writers.

Honor SQLite's requested file synchronization and the necessary directory synchronization after file
creation or removal. Propagate failures. Never report a successful durable commit after suppressing a
write or synchronization error. Advertise only device capabilities the implementation guarantees.

The application-level `write.lock` remains separate: mutating commands acquire it, read-only commands do
not. It cannot substitute for SQLite's coordination with readers. SQLite documents both concurrent WAL
access and its contention exceptions in [write-ahead logging](https://www.sqlite.org/wal.html).

## Access policies

| Consumer             | Allowed effects                                                                                                                                             |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Writable ledger      | Create or repair private files, initialize WAL, migrate inside a transaction, write, checkpoint and recover.                                                |
| Observational ledger | Read existing database and companions; acquire coordination locks. No creation, permission repair, migration, checkpoint write or writable mapping.         |
| Refreshed title copy | Read the copied database and WAL. Initialize or update shared memory only inside the private copy directory; leave copied database and WAL bytes unchanged. |
| Existing title copy  | Read existing files and acquire coordination locks. No creation, truncation, permission repair or writable mapping.                                         |

Use a five-second busy timeout on every production connection. Tests may choose a shorter timeout. Keep
foreign-key enforcement on the ledger. Persist ledger companions across close so observational
connections can use them without creating files. An absent ledger, absent state directory or zero schema
still yields the existing empty-ledger outcome. Missing companions for a populated ledger are an error;
observation must not repair them.

Observation itself must not change file bytes, directory entries, modes, sizes, modification times,
change times or flags. Other writers may still change the ledger. Verify unchanged file snapshots with
those writers quiescent. Access-time changes made by the operating system as a consequence of reading are
outside this promise. A read-only connection may fail when recovery requires writes; it never promotes
itself to a writable connection. Do not mark the live ledger immutable, because other commands may update
it.

### Title-copy integration

Source files in the Voice Memos container are opened only through VPT's read-only file handles. SQLite
receives the private destination directory, never the source pathname. Preserve the fixed copied files,
their private permissions and the rule that a dry run does not refresh or open title databases.

Drop the previous title-copy connection before refreshing its files. Refresh must have exclusive access
to the private copy while truncating and copying; retained title-copy readers prevent refresh until they
close, with the same bounded wait. This coordination is confined to the private title copy and does not
block ledger readers. Copy failures leave titles unavailable. Reopen only after the complete copy
succeeds, including the existing handling of missing source companions.

Read committed titles that remain in the copied WAL. Ordinary copy, schema and title-query failures
continue to yield `Unavailable`. The shared opener and mutating refresh preserve typed containment errors
instead of discarding them through `.ok()` or a catch-all fallback. Existing-copy observation uses its
own read-only policy, not the refresh policy. Adapter-local signatures may change to retain these errors;
application repository contracts remain unchanged.

## Errors

An escaped or redirected path is a containment refusal, mapped to the existing `path_escape` rule and
exit 3 where the command reports the operation. Preserve busy, corruption, constraint and ordinary
input/output failures as distinct ledger outcomes. These remain exit 1 at the command boundary unless the
existing command contract says otherwise.

Track containment failures alongside SQLite's numeric error so a generic database-open failure cannot
erase their type. Keep that state local to the affected connection. Diagnostics may identify the
configured path and failed operation, but must not include database contents or Voice Memos titles. All
error and close paths retain the same filesystem authority as the successful path.

## Acceptance evidence

Each new behavior starts with a compiling test that fails for its intended reason. Use synthetic files
under owned temporary roots, never live Voice Memos, configuration or state. Every test finishes within
one second under the workspace suite. Blocking tests have an independent deadline and owned-process
cleanup; remove the implementation bound in a control mutation to prove the watchdog works.

| Scenario                                                      | Required observation                                                                                                                  |
| ------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| Root pathname replaced after acquisition                      | Database, companion and cleanup operations remain attached to the original root or refuse; replacement files remain unchanged.        |
| Companion redirected or wrong file type                       | Refusal before touching the target; cover database, rollback journal, WAL and shared memory.                                          |
| Unknown filename, attached database or disk temporary request | Fail closed with no filesystem fallback.                                                                                              |
| Concurrent reader and writer                                  | The reader sees its committed snapshot while a writer commits; a subsequent read sees the new commit.                                 |
| Contention and connection close                               | Busy waits remain bounded; closing one connection leaves another connection's locks effective.                                        |
| Native SQLite interoperability                                | Both implementations read ordinary files, respect each other's locks and coordinate WAL access across processes.                      |
| Failed write, synchronization or interrupted transaction      | No partial logical transaction is accepted; a writable reopen recovers committed state through the same boundary.                     |
| Observational open, query and close                           | File snapshots retain the properties above, including on missing companions or required recovery.                                     |
| Private title copy                                            | A WAL-only committed title is visible; source files remain unchanged; failed refresh and absent titles retain their defined outcomes. |
| Title copy retained during refresh                            | Refresh waits or returns busy without truncating files still in use; ledger reading remains available.                                |
| Failed open and final close                                   | Registrations, mappings, descriptors and locks are released without redirecting cleanup.                                              |

Test VPT's filesystem implementation through its production boundary. No third-party source patch or
SQLite system-call override is part of these tests. Mutation checks must prove the relevant guard or
route matters, with exact edited bytes and an unmodified control recorded in the execution report.

Run the clean-code Rust gates after implementation, including the shared contract suite against both the
in-memory and durable ledger implementations. Keep unsafe operations private, source files below the hard
500-line ceiling and the command entry point below 150 lines. Review locking, callback lifetime and
failure cleanup independently before calling the adapter complete.

## Delivery boundary

Implement this within the existing five-crate workspace. Task 5a supplies checked-root operations; Task
11 owns the shared database boundary and ledger integration; Task 16 uses it for the private title copy.
Repository consumers in Tasks 12 and 31 keep their application contracts. Command composition and
observation coverage in Tasks 27, 28 and 30 must exercise the strengthened route.

The written implementation plan must allocate the acceptance evidence above to bounded tasks before
product implementation resumes. This amendment changes no shipped data format and authorizes no
installation, live-data test, third-party modification, push or pull request.
