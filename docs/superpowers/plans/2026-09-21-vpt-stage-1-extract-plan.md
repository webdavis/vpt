# vpt Stage 1: Extract Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended)
> or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax
> for tracking.

**Goal:** Ship stage 1 of vpt, the Voice Processing Tool: a workspace that installs, sweeps Apple's Voice
Memos store read-only, archives each recording under a content-derived identity in a SQLite-backed home,
and offers `setup`, `doctor`, `ingest`, `show`, `list`, `storage`, `retention run`, `symlink` and
`--version`, with notifications through the producer API and a Swift helper for `notify` and `trash`.

**Architecture:** Five crates in one Cargo workspace with a one-way dependency direction enforced by each
manifest: `vpt-domain` (pure policy: identity, the MPEG-4 wholeness gate, sweep gates, retention
decision), `vpt-application` (one concrete use case per verb and the ports it owns), `vpt-protocol` (the
versioned JSON documents), `vpt-adapters` (filesystem, SQLite, process spawning, TOML) and the command
crate `vpt` (argument decoding, exit codes, composition). A Swift package `helper/vpt-macos` reaches the
macOS frameworks for notifications and the Trash. Every test runs in a sandbox and nothing reaches the
operator's real store, home, config or desktop.

**Tech Stack:** Rust stable (edition 2024), `rusqlite` 0.39 (bundled SQLite, WAL), `serde` 1.0.229 and
`serde_json` 1.0.151 for the protocol, `toml` 1.1.4 for configuration, `sha2` 0.11 for digests, `libc`
0.2.189 for the handful of macOS calls (`fclonefileat`, `renamex_np`, `flock`, `pread`, `killpg`,
`localtime_r`, `isatty`), `tempfile` 3.27 in tests only; Swift 6 with `swift build` and `swift test`.

**Spec:** `docs/superpowers/specs/2026-09-20-vpt-design.md` (the plan argues from it; read sections 3, 4,
5, 9 to 14 alongside each task).

## Global Constraints

Every task's requirements implicitly include this section. Values are copied from the spec.

- **Toolchain.** `rust-toolchain.toml` pinning `stable` (spec section 13). The spec names no edition;
  every manifest in this plan uses `edition = "2024"`. `Cargo.lock` committed, built `--locked`.
- **Five crates, one workspace** (spec section 3.1): `crates/vpt-domain`, `crates/vpt-application`,
  `crates/vpt-protocol`, `crates/vpt-adapters`, `crates/vpt`. Stage 1 creates all five in Task 1.
  Direction: `vpt-domain <- vpt-application <- vpt-adapters <- vpt`; `vpt-protocol` is used by
  `vpt-adapters` and `vpt` and depends on nothing in the workspace. The command crate is named `vpt`, its
  binary is `vpt`, and `main.rs` is under 150 lines and holds no policy.
- **The domain excludes** filesystem access, SQLite, TOML, JSON, HTTP, environment variables, process
  spawning, macOS APIs, vendor APIs and terminal output. Each domain function is a total function of its
  arguments.
- **Ports are traits** because each is an external capability with a fake in tests. Nothing else in the
  workspace is a trait. `dyn Trait` appears only at the composition root in `vpt`.
- **File size** (spec section 2 and the clean-code Rust skill): Rust files target 200 implementation and
  300 total lines, and never exceed 500 total with tests included, measured after `rustfmt` with the
  skill's awk command; no waiver. Swift production files never exceed 200 lines; Swift test files may run
  to 700.
- **Clean code and SOLID.** One responsibility per file, module, type and function; concrete use-case
  types; ports narrow but cohesive; typed outcomes, never a bare `Option` where failure classes differ;
  no `unwrap` or `expect` on untrusted input; no `Arc<Mutex<_>>` by default; private by default with
  curated `lib.rs` exports; no `#[cfg(test)]` item above production code. Implementation modules stay
  private. Named capability APIs (`config`, application `ports`, domain and protocol document modules)
  expose only their intended types and functions; tests stay in private children.
- **Test-first without exception** (spec section 12): the failing test first, seen to fail for the
  intended reason, then the code. Unit tests live beside their implementation under `#[cfg(test)]`, in a
  private `tests.rs` child when large. Every test finishes within one second under
  `cargo test --workspace`. Mutation verification is by hand per behavior, against an unmutated control,
  and its table goes in each pull request.
- **Sandboxed tests.** Every binary run in a test sets `HOME`, `XDG_CONFIG_HOME`, `XDG_DATA_HOME`,
  `XDG_STATE_HOME` and `VPT_CONFIG` to a temporary directory and `PATH` to a directory the test controls.
  No test reads the operator's config, ledger, vault or Voice Memos store; no test posts a notification
  or moves a file to the real Trash; no test reaches the network.
- **The Voice Memos store is read-only** (spec section 5.1): every file handle into it is opened
  read-only, vpt never creates, renames, rewrites or deletes anything inside it, a full sweep leaves
  every entry's size, mtime and flags unchanged, and the live database is never opened.
- **Trash, never delete** (spec section 11): nothing vpt does unlinks. Removal is the opt-in retention
  run moving files to the system Trash through the helper, and a failed staging moving its staged file to
  the Trash; with the helper absent such a file stays where it is.
- **Ledger schema versioning** (spec section 4.4): one SQLite database, mode 0600 in a 0700 directory,
  WAL mode, a 5 s busy timeout on every connection, versioned migrations applied at open inside a
  transaction with `user_version`, a future version refused.
- **Exit codes** (spec section 9): 0 did what was asked; 1 vpt failed (an engine, the helper, a spawned
  command, the filesystem or the ledger); 2 the input was wrong (unknown argument, a config value vpt
  refuses to read, an unknown id, a verb that needs an unset key); 3 vpt refused by one of its own rules,
  and the message names the rule.
- **Output contract** (spec section 9): `--json` output is withheld until the final status is known; on
  success one document on stdout; on failure one `vpt.error/1` document on stderr and stdout stays empty.
- **Copy rules.** No em-dashes anywhere (code, comments, documents, commit messages). No home path of the
  author's machine and no machine name in any file. Conventional Commits, one logical change per commit,
  `SKIP_AI_COMMIT=1` in the environment, never an AI co-author trailer. `trash`, never `rm`, for anything
  removed by hand, scratch directories included.
- **Protocol limits** (spec section 3.1): every incoming document other than `vpt.engine/1` and
  `vpt.proposal/1` is limited to 65,536 bytes, depth 8, 65,536 characters per text value and 256 entries
  per array, enforced while the document is read.

______________________________________________________________________

## File structure

Created by this plan (a task names the files it creates or modifies):

```
Cargo.toml                                   workspace: resolver 3, five members, default-members ["crates/vpt"]
rust-toolchain.toml                          channel = "stable"
Cargo.lock
.gitignore                                   (exists) target/, .superpowers/, audio extensions
README.md                                    the two install steps of spec section 13
justfile                                     fmt, clippy, test, doc, gitleaks, file-size, swift-build, swift-test, ship
.github/workflows/ci.yml                     the gates in justfile order on macos-latest
crates/vpt-domain/src/lib.rs                 curated exports
crates/vpt-domain/src/time.rs                UtcInstant, UtcOffset, FileTime, civil date arithmetic
crates/vpt-domain/src/digest.rs              Sha256Digest
crates/vpt-domain/src/identity.rs            RecordingId
crates/vpt-domain/src/container.rs           the MPEG-4 wholeness gate and mvhd reading
crates/vpt-domain/src/sweep.rs               the sweep gates and deferral reasons
crates/vpt-domain/src/duration.rs            the config duration syntax
crates/vpt-domain/src/retention.rs           Hold and the expiry decision
crates/vpt-domain/src/layout.rs              StoreKey and the root overlap rule
crates/vpt-domain/src/notification.rs        Notification, EventKind, EventState
crates/vpt-domain/src/fixtures.rs            (feature "fixtures") the assembled MPEG-4 test bytes
crates/vpt-application/src/lib.rs            curated exports
crates/vpt-application/src/settings.rs       validated Settings and its sub-structs
crates/vpt-application/src/ports/mod.rs      re-exports
crates/vpt-application/src/ports/recorder.rs RecorderStore, SourceHandle, TitleSource
crates/vpt-application/src/ports/archive.rs  Archive
crates/vpt-application/src/ports/ledger.rs   RecordingLedger, PublicationJournal, rows
crates/vpt-application/src/ports/retention.rs RetentionJournal and the intent rows
crates/vpt-application/src/ports/clock.rs    Clock
crates/vpt-application/src/ports/trash.rs    Trash
crates/vpt-application/src/ports/notifier.rs Notifier
crates/vpt-application/src/ports/stores.rs   Stores
crates/vpt-application/src/ports/prompt.rs   Prompt
crates/vpt-application/src/setup.rs          Setup
crates/vpt-application/src/publication.rs    repair_publications
crates/vpt-application/src/ingest/mod.rs     Ingest, IngestReport, IngestError
crates/vpt-application/src/ingest/candidate.rs one candidate through the gates
crates/vpt-application/src/ingest/publish.rs staging, publication, duplicate resolution
crates/vpt-application/src/retention/mod.rs  Retention, RetentionReport, RetentionError
crates/vpt-application/src/retention/report.rs partial progress and typed failures
crates/vpt-application/src/retention/reconcile.rs reconcile_intents
crates/vpt-application/src/inventory.rs      StoreInventory for vpt storage
crates/vpt-protocol/src/lib.rs               curated exports
crates/vpt-protocol/src/result.rs            vpt.result/1
crates/vpt-protocol/src/error.rs             vpt.error/1
crates/vpt-protocol/src/event.rs             vpt.event/1
crates/vpt-protocol/src/helper.rs            vpt.helper/1 and the notify and trash replies
crates/vpt-protocol/src/limits.rs            the bounded JSON reader
crates/vpt-protocol/src/limits/visitor.rs    bounded descent before retaining children
crates/vpt-protocol/src/limits/tests.rs      private bounded-reader regressions
crates/vpt-adapters/src/lib.rs               curated exports
crates/vpt-adapters/src/contained.rs         no-follow access through root descriptors
crates/vpt-adapters/src/contained/tests.rs   private checked-access regressions
crates/vpt-adapters/src/config/schema.rs     the one key table: name, kind, default, comment, secret
crates/vpt-adapters/src/config/render.rs     the setup template
crates/vpt-adapters/src/config/load.rs       read, parse, unknown keys, merge over defaults
crates/vpt-adapters/src/config/validate.rs   value rules by kind
crates/vpt-adapters/src/config/settings.rs   table to Settings, path expansion
crates/vpt-adapters/src/config/roots.rs      root resolution and overlap refusals
crates/vpt-adapters/src/config/roots/tests.rs private root-resolution regressions
crates/vpt-adapters/src/config/paths.rs      config path discovery (VPT_CONFIG, XDG, HOME)
crates/vpt-adapters/src/config/write.rs      setup's filesystem writes
crates/vpt-adapters/src/ledger/mod.rs        LedgerError
crates/vpt-adapters/src/ledger/sqlite/mod.rs SqliteLedger: open, permissions, WAL, busy timeout
crates/vpt-adapters/src/ledger/sqlite/migrations.rs the versioned schema
crates/vpt-adapters/src/ledger/sqlite/connection.rs retained-root checks and read-only WAL access
crates/vpt-adapters/src/ledger/sqlite/boundary_tests.rs connection-boundary regressions
crates/vpt-adapters/src/ledger/sqlite/recordings.rs RecordingLedger for SQLite
crates/vpt-adapters/src/ledger/sqlite/journal.rs PublicationJournal for SQLite
crates/vpt-adapters/src/ledger/sqlite/retention.rs RetentionJournal for SQLite
crates/vpt-adapters/src/ledger/memory.rs     MemoryLedger, the in-memory twin
crates/vpt-adapters/src/ledger/contract.rs   (cfg(test)) the contract suite both ledgers run
crates/vpt-adapters/src/ledger/contract/retention.rs private retention contract child
crates/vpt-adapters/src/lock.rs              WriteLock over flock
crates/vpt-adapters/src/clock.rs             SystemClock
crates/vpt-adapters/src/voice_memos/store.rs VoiceMemosStore: listing and read-only descriptors
crates/vpt-adapters/src/voice_memos/store/tests.rs private source-boundary tests
crates/vpt-adapters/src/voice_memos/titles.rs the private database copy and the title lookup
crates/vpt-adapters/src/voice_memos/titles/tests.rs private title-copy tests
crates/vpt-adapters/src/archive/mod.rs       ClonefileArchive: staging and digests
crates/vpt-adapters/src/archive/tests.rs     private staging and cleanup tests
crates/vpt-adapters/src/archive/publish.rs   exclusive publication and directory sync
crates/vpt-adapters/src/spawn.rs             bounded process execution
crates/vpt-adapters/src/spawn/tests.rs      controlled executor regressions
crates/vpt-adapters/src/spawn/tests/support.rs isolated process fixtures
crates/vpt-adapters/src/helper.rs            HelperClient: version, notify, trash
crates/vpt-adapters/src/helper/reply.rs      known fields and additive diagnostics
crates/vpt-adapters/src/helper/tests.rs      private helper protocol regressions
crates/vpt-adapters/src/notify/mod.rs        DesktopNotifier, CommandNotifier, OffNotifier
crates/vpt-adapters/src/notify/delivery.rs   delivery and fallback diagnostics
crates/vpt-adapters/src/notify/tests.rs      event and token regressions
crates/vpt-adapters/src/notify/delivery/tests.rs private delivery regressions
crates/vpt-adapters/src/stores.rs            FilesystemStores
crates/vpt-adapters/src/symlink.rs           the managed link
crates/vpt-adapters/src/git_tree.rs          the git working tree walk
crates/vpt-adapters/src/prompt.rs            TtyPrompt
crates/vpt/src/main.rs                       fn main() { vpt::run() }
crates/vpt/src/lib.rs                        run(): parse, dispatch, emit, exit
crates/vpt/src/cli/args.rs                   Verb, Invocation, parse, USAGE
crates/vpt/src/cli/output.rs                 Outcome and emission
crates/vpt/src/compose.rs                    Runtime: settings, roots, adapters
crates/vpt/src/compose/clock.rs             production and dev-tools clocks
crates/vpt/src/compose/errors.rs            typed failures to error documents
crates/vpt/src/compose/ledger.rs            read-only or writable ledger composition
crates/vpt/src/compose/runtime.rs           operation-specific startup and retained state
crates/vpt/src/compose/observations.rs      ledger-only and stores-only observation
crates/vpt/src/compose/recovery.rs          startup progress and retention events
crates/vpt/src/compose/tests.rs             private composition regressions
crates/vpt/src/documents/record.rs           RecordingRecord to JSON
crates/vpt/src/commands/*.rs                 one file per verb
crates/vpt/src/doctor/mod.rs                 vpt doctor: the verdict
crates/vpt/src/doctor/checks.rs              one function per check
crates/vpt/src/doctor/census.rs             stable check names and dependencies
crates/vpt/src/doctor/probes.rs             independent read-only checks
crates/vpt/src/bin/vpt-fake-engine.rs        (feature dev-tools) the fake helper and command
crates/vpt/tests/support/mod.rs              the sandbox harness
crates/vpt/tests/<behavior>.rs               acceptance tests, one file per behavior area
crates/vpt-adapters/tests/support/mod.rs     the ingest fixture, fixed clock, recording trash and notifier
crates/vpt-adapters/tests/<behavior>.rs      the ingest and retention suites over real adapters
helper/vpt-macos/Package.swift
helper/vpt-macos/Sources/VptMacos/*.swift    Arguments, Documents, Notify, Trash, Run
helper/vpt-macos/Sources/vpt-macos/main.swift
helper/vpt-macos/Tests/VptMacosTests/*.swift
```

______________________________________________________________________

### Task 1: Workspace scaffold and `vpt --version`

**Files:**

- Create: `Cargo.toml`, `rust-toolchain.toml`, `README.md`, `justfile`, `.github/workflows/ci.yml`
- Create: `crates/vpt-domain/Cargo.toml`, `crates/vpt-domain/src/lib.rs`
- Create: `crates/vpt-application/Cargo.toml`, `crates/vpt-application/src/lib.rs`
- Create: `crates/vpt-protocol/Cargo.toml`, `crates/vpt-protocol/src/lib.rs`
- Create: `crates/vpt-adapters/Cargo.toml`, `crates/vpt-adapters/src/lib.rs`
- Create: `crates/vpt/Cargo.toml`, `crates/vpt/src/main.rs`, `crates/vpt/src/lib.rs`,
  `crates/vpt/src/cli/mod.rs`, `crates/vpt/src/cli/args.rs`, `crates/vpt/src/commands/mod.rs`,
  `crates/vpt/src/commands/version.rs`
- Test: `crates/vpt/tests/support/mod.rs`, `crates/vpt/tests/version.rs`, `crates/vpt/tests/usage.rs`

**Interfaces:**

- Consumes: nothing.

- Produces: `vpt::run() -> !` (the crate's only public item; `cli` and `commands` are private modules);
  `cli::args::{Verb, Invocation { pub verb: Verb, pub json: bool,`
  `pub config: Option<PathBuf> }, UsageError(pub String),`
  `parse(args: impl IntoIterator<Item = OsString>) -> Result<Invocation, UsageError>,` `USAGE: &str}`;
  `commands::version::{VERSION: &str, document(helper_version: Option<&str>) ->`
  `serde_json::Value, human(helper_version: Option<&str>) -> String}`; the test harness
  `support::{VPT: &str, Sandbox::new(name: &str) -> Sandbox, Sandbox::path(&self) ->`
  `&Path, Sandbox::config_path(&self) -> PathBuf, Sandbox::vpt(&self) ->`
  `std::process::Command, run(command: &mut Command) -> Output, stdout(output: &Output) ->`
  `String, stderr(output: &Output) -> String}`.

- [ ] **Step 1: Write the failing acceptance test**

`crates/vpt/tests/support/mod.rs`:

```rust
//! The sandbox every binary test runs in: a private HOME and config, and a PATH
//! holding nothing but what the test puts there. Env rides on the Command, never
//! on the process, because the test binary is threaded.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub const VPT: &str = env!("CARGO_BIN_EXE_vpt");

pub struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    pub fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("vpt-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for leaf in ["home", "config", "data", "state", "bin", "voice-memos/Recordings"] {
            std::fs::create_dir_all(root.join(leaf)).expect("sandbox leaf");
        }
        let root = root.canonicalize().expect("canonical sandbox root");
        Sandbox { root }
    }

    pub fn path(&self) -> &Path {
        &self.root
    }

    pub fn config_path(&self) -> PathBuf {
        self.root.join("config/vpt/config.toml")
    }

    pub fn vpt(&self) -> Command {
        let mut command = Command::new(VPT);
        command
            .env_clear()
            .env("HOME", self.root.join("home"))
            .env("XDG_CONFIG_HOME", self.root.join("config"))
            .env("XDG_DATA_HOME", self.root.join("data"))
            .env("XDG_STATE_HOME", self.root.join("state"))
            .env("VPT_CONFIG", self.config_path())
            .env("PATH", self.root.join("bin"));
        command
    }
}

pub fn run(command: &mut Command) -> Output {
    command.output().expect("vpt spawned")
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
```

`crates/vpt/tests/version.rs`:

```rust
mod support;

use support::{Sandbox, run, stdout};

#[test]
fn version_reports_the_crate_version_and_no_helper_when_the_path_is_empty() {
    let sandbox = Sandbox::new("version-json");

    let output = run(sandbox.vpt().args(["--version", "--json"]));

    assert_eq!(output.status.code(), Some(0));
    let document: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("json");
    assert_eq!(document["schema"], "vpt.result/1");
    assert_eq!(document["command"], "version");
    assert_eq!(document["version"], env!("CARGO_PKG_VERSION"));
    assert!(document["helper_version"].is_null());
}

#[test]
fn version_without_json_prints_one_human_line() {
    let sandbox = Sandbox::new("version-human");

    let output = run(sandbox.vpt().arg("--version"));

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout(&output), format!("vpt {} (helper absent)\n", env!("CARGO_PKG_VERSION")));
}
```

`crates/vpt/tests/usage.rs`:

```rust
mod support;

use support::{Sandbox, run, stderr, stdout};

#[test]
fn an_unknown_verb_prints_usage_to_stderr_and_exits_2() {
    let sandbox = Sandbox::new("usage-unknown");

    let output = run(sandbox.vpt().arg("transcode"));

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout(&output), "");
    let err = stderr(&output);
    assert!(err.starts_with("vpt: unknown verb transcode\n"), "{err}");
    assert!(err.contains("usage: vpt"), "{err}");
}
```

`crates/vpt/src/cli/args.rs` starts as its test module alone; Step 3 adds the code above it:

```rust
//! Argument decoding. Hand rolled: nine verbs and four flags do not justify a parser crate.

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::path::PathBuf;

    fn parsed(line: &str) -> Result<Invocation, UsageError> {
        parse(std::iter::once(OsString::from("vpt")).chain(line.split(' ').map(OsString::from)))
    }

    #[test]
    fn json_and_config_are_global_flags_in_any_position() {
        let invocation = parsed("ingest --json --config /tmp/c.toml --dry-run").expect("parsed");
        assert!(invocation.json);
        assert_eq!(invocation.config, Some(PathBuf::from("/tmp/c.toml")));
        assert_eq!(invocation.verb, Verb::Ingest { dry_run: true, once: None });
    }

    #[test]
    fn an_unknown_verb_is_a_usage_error_naming_it() {
        assert_eq!(parsed("transcode"), Err(UsageError("unknown verb transcode".into())));
    }

    #[test]
    fn a_trailing_unknown_argument_is_refused() {
        assert_eq!(parsed("doctor --loud"), Err(UsageError("unexpected argument --loud".into())));
    }

    #[test]
    fn a_value_flag_without_a_value_is_refused() {
        assert_eq!(parsed("ingest --once"), Err(UsageError("--once needs a value".into())));
    }
}
```

- [ ] **Step 2: Create the workspace so the tests can compile, then run them to verify they fail**

`Cargo.toml`:

```toml
[workspace]
resolver = "3"
default-members = ["crates/vpt"]
members = [
  "crates/vpt-adapters",
  "crates/vpt-application",
  "crates/vpt",
  "crates/vpt-domain",
  "crates/vpt-protocol",
]
```

`rust-toolchain.toml`:

```toml
# Every crate builds on stable. rustup reads this file from the current
# directory upward, so a builder that runs cargo from this checkout gets stable
# whatever its default toolchain is.
[toolchain]
channel = "stable"
```

`crates/vpt-domain/Cargo.toml`:

```toml
# vpt-domain: vpt policy as total functions of their arguments.
#
# No workspace dependencies and no infrastructure dependency: nothing here reads
# a file, opens a database, parses TOML or JSON, spawns a process or asks the
# clock. A caller hands it an observation and it answers.

[package]
name = "vpt-domain"
version = "0.1.0"
edition = "2024"

[features]
fixtures = []
```

`crates/vpt-domain/src/lib.rs`:

```rust
//! Pure policy: identity, the wholeness gate, sweep gates, retention and value types.
```

`crates/vpt-application/Cargo.toml`:

```toml
# vpt-application: one concrete use case per verb, and the ports they own.
#
# Depends on vpt-domain and on nothing else in the workspace. A use case names
# the external capability it needs as a trait it declares itself, and an adapter
# implements that trait from the outside.

[package]
name = "vpt-application"
version = "0.1.0"
edition = "2024"

[dependencies]
vpt-domain = { path = "../vpt-domain" }
```

`crates/vpt-application/src/lib.rs`:

```rust
//! Use cases and the ports they own.
```

`crates/vpt-protocol/Cargo.toml`:

```toml
# vpt-protocol: the versioned documents that cross a process boundary.
#
# No workspace dependencies, including vpt-domain: this is what a command engine,
# an agent command or a notify command reads and writes, and the internal model
# must not leak into it. Translation lives in the adapters.

[package]
name = "vpt-protocol"
version = "0.1.0"
edition = "2024"

[dependencies]
serde = { version = "1.0.229", features = ["derive"] }
serde_json = "1.0.151"
```

`crates/vpt-protocol/src/lib.rs`:

```rust
//! The versioned JSON documents vpt reads and writes across a process boundary.
```

`crates/vpt-adapters/Cargo.toml`:

```toml
# vpt-adapters: everything concrete, organized by the capability it provides.
#
# Depends inward only: on the ports in vpt-application, the policy in
# vpt-domain and the documents in vpt-protocol. Only the command crate depends
# on this one.

[package]
name = "vpt-adapters"
version = "0.1.0"
edition = "2024"

[dependencies]
vpt-application = { path = "../vpt-application" }
vpt-domain = { path = "../vpt-domain" }
vpt-protocol = { path = "../vpt-protocol" }

libc = "0.2.189"
rusqlite = { version = "=0.39.0", default-features = false, features = [
  "bundled",
] }
serde_json = "1.0.151"
sha2 = "0.11.0"
toml = { version = "1.1.4", default-features = false, features = [
  "parse",
  "serde",
  "std",
] }

[dev-dependencies]
tempfile = "3.27.0"
vpt-domain = { path = "../vpt-domain", features = ["fixtures"] }
```

`crates/vpt-adapters/src/lib.rs`:

```rust
//! Concrete adapters: filesystem, SQLite, processes and configuration.
```

`crates/vpt/Cargo.toml`:

```toml
# vpt: argument decoding, exit codes and composition. The binary keeps the
# tool's name because every caller invokes it by that name.

[package]
name = "vpt"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "vpt"
path = "src/main.rs"

# The fake helper and fake command used by the tests are feature gated so that
# `cargo install --git <url> vpt` installs only vpt.
[features]
dev-tools = []

[[bin]]
name = "vpt-fake-engine"
path = "src/bin/vpt-fake-engine.rs"
required-features = ["dev-tools"]

[dependencies]
vpt-adapters = { path = "../vpt-adapters" }
vpt-application = { path = "../vpt-application" }
vpt-domain = { path = "../vpt-domain" }
vpt-protocol = { path = "../vpt-protocol" }

serde_json = "1.0.151"

[dev-dependencies]
rusqlite = { version = "=0.39.0", default-features = false, features = [
  "bundled",
] }
vpt-domain = { path = "../vpt-domain", features = ["fixtures"] }
```

`crates/vpt/src/bin/vpt-fake-engine.rs` (Task 24 fills it; the file must exist for the manifest):

```rust
fn main() {
    eprintln!("vpt-fake-engine: no subcommand yet");
    std::process::exit(2);
}
```

`crates/vpt/src/main.rs`:

```rust
fn main() {
    vpt::run();
}
```

`crates/vpt/src/lib.rs`:

```rust
//! The command crate: parse, dispatch, emit, exit. No policy lives here.

mod cli;
mod commands;

pub fn run() -> ! {
    std::process::exit(2);
}
```

`crates/vpt/src/cli/mod.rs`:

```rust
pub(crate) mod args;
```

`crates/vpt/src/commands/mod.rs`:

```rust
pub(crate) mod version;
```

`crates/vpt/src/commands/version.rs` starts as one line, `//! \`vpt --version\`.`, and `args.rs\` is the
test module written in Step 1. Every module is declared before the red run so the test modules are
compiled and selected; a run that selects zero tests, or that succeeds, does not satisfy this step.

Run: `cargo test -p vpt --test version --test usage`

Expected: all three acceptance tests FAIL, `assertion failed: left: Some(2), right: Some(0)` for the two
version tests (the binary exits 2 before parsing anything) and the usage test on its
`starts_with("vpt: unknown verb transcode")` assertion.

Run: `cargo test -p vpt --lib`

Expected: the build of the `args::tests` module fails with `cannot find` for `parse`, `Invocation`,
`UsageError` and `Verb`: the module is compiled and its four tests are what the code must satisfy.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt/src/cli/args.rs`, above the test module:

```rust
//! Argument decoding. Hand rolled: nine verbs and four flags do not justify a parser crate.

use std::ffi::OsString;
use std::path::PathBuf;

pub const USAGE: &str = "usage: vpt [--json] [--config <file>] <verb>\n\
verbs: setup [--force] | doctor | ingest [--dry-run] [--once <path>] | show <id> |\n\
       list [--stage <stage>] | storage | retention run [--dry-run] |\n\
       symlink deploy | symlink verify | --version\n";

#[derive(Debug, PartialEq, Eq)]
pub enum Verb {
    Version,
    Setup { force: bool },
    Doctor,
    Ingest { dry_run: bool, once: Option<PathBuf> },
    Show { id: String },
    List { stage: Option<String> },
    Storage,
    RetentionRun { dry_run: bool },
    SymlinkDeploy,
    SymlinkVerify,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Invocation {
    pub verb: Verb,
    pub json: bool,
    pub config: Option<PathBuf>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct UsageError(pub String);

pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Invocation, UsageError> {
    let mut words: Vec<String> = args
        .into_iter()
        .skip(1)
        .map(|word| word.to_string_lossy().into_owned())
        .collect();
    let json = take_flag(&mut words, "--json");
    let config = take_value(&mut words, "--config")?.map(PathBuf::from);
    let verb = parse_verb(&mut words)?;
    if let Some(extra) = words.first() {
        return Err(UsageError(format!("unexpected argument {extra}")));
    }
    Ok(Invocation { verb, json, config })
}

fn parse_verb(words: &mut Vec<String>) -> Result<Verb, UsageError> {
    if words.is_empty() {
        return Err(UsageError("a verb is required".into()));
    }
    let first = words.remove(0);
    match first.as_str() {
        "--version" => Ok(Verb::Version),
        "setup" => Ok(Verb::Setup { force: take_flag(words, "--force") }),
        "doctor" => Ok(Verb::Doctor),
        "ingest" => Ok(Verb::Ingest {
            dry_run: take_flag(words, "--dry-run"),
            once: take_value(words, "--once")?.map(PathBuf::from),
        }),
        "show" => Ok(Verb::Show { id: take_positional(words, "<id>")? }),
        "list" => Ok(Verb::List { stage: take_value(words, "--stage")? }),
        "storage" => Ok(Verb::Storage),
        "retention" => match take_positional(words, "run")?.as_str() {
            "run" => Ok(Verb::RetentionRun { dry_run: take_flag(words, "--dry-run") }),
            other => Err(UsageError(format!("unknown retention subcommand {other}"))),
        },
        "symlink" => match take_positional(words, "deploy|verify")?.as_str() {
            "deploy" => Ok(Verb::SymlinkDeploy),
            "verify" => Ok(Verb::SymlinkVerify),
            other => Err(UsageError(format!("unknown symlink subcommand {other}"))),
        },
        other => Err(UsageError(format!("unknown verb {other}"))),
    }
}

fn take_flag(words: &mut Vec<String>, flag: &str) -> bool {
    match words.iter().position(|word| word == flag) {
        Some(index) => {
            words.remove(index);
            true
        }
        None => false,
    }
}

fn take_value(words: &mut Vec<String>, flag: &str) -> Result<Option<String>, UsageError> {
    let Some(index) = words.iter().position(|word| word == flag) else {
        return Ok(None);
    };
    if index + 1 >= words.len() || words[index + 1].starts_with("--") {
        return Err(UsageError(format!("{flag} needs a value")));
    }
    words.remove(index);
    Ok(Some(words.remove(index)))
}

fn take_positional(words: &mut Vec<String>, name: &str) -> Result<String, UsageError> {
    if words.is_empty() || words[0].starts_with("--") {
        return Err(UsageError(format!("{name} is required")));
    }
    Ok(words.remove(0))
}
```

The test module of Step 1 stays below this code, unchanged. `crates/vpt/src/commands/version.rs`:

```rust
//! `vpt --version`: the crate version and the helper's, when one answers.

use serde_json::{Value, json};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn document(helper_version: Option<&str>) -> Value {
    json!({
        "schema": "vpt.result/1",
        "command": "version",
        "version": VERSION,
        "helper_version": helper_version,
    })
}

pub fn human(helper_version: Option<&str>) -> String {
    match helper_version {
        Some(helper) => format!("vpt {VERSION} (helper {helper})\n"),
        None => format!("vpt {VERSION} (helper absent)\n"),
    }
}
```

`crates/vpt/src/lib.rs`:

```rust
//! The command crate: parse, dispatch, emit, exit. No policy lives here.

mod cli;
mod commands;

use cli::args::{Invocation, Verb, parse};
use std::io::Write;

pub fn run() -> ! {
    let code = match parse(std::env::args_os()) {
        Ok(invocation) => dispatch(&invocation),
        Err(error) => {
            eprint!("vpt: {}\n{}", error.0, cli::args::USAGE);
            2
        }
    };
    std::process::exit(code);
}

fn dispatch(invocation: &Invocation) -> i32 {
    match &invocation.verb {
        Verb::Version => {
            let text = if invocation.json {
                format!("{}\n", commands::version::document(None))
            } else {
                commands::version::human(None)
            };
            let _ = std::io::stdout().write_all(text.as_bytes());
            0
        }
        _ => {
            eprint!("vpt: verb not implemented yet\n{}", cli::args::USAGE);
            2
        }
    }
}
```

`README.md`:

```markdown
# vpt

The Voice Processing Tool: it archives the voice memos Apple's Voice Memos syncs to a Mac, transcribes
them, and writes Markdown notes. The design is `docs/superpowers/specs/2026-09-20-vpt-design.md`.

## Install

Two steps, both required for the Apple speech engine, desktop notifications and the Trash:

1. `cargo install --git https://github.com/webdavis/vpt vpt` installs `vpt` into `~/.cargo/bin`.
2. `cd helper/vpt-macos && swift build -c release` builds `vpt-macos`; copy
   `.build/release/vpt-macos` beside `vpt` (into `~/.cargo/bin`).

Then `vpt setup` writes the configuration and `vpt doctor` reports what it found.
```

`justfile`:

```just
# The gates, in CI order. `just ship` runs them all.

fmt:
  cargo fmt --all -- --check

clippy:
  cargo clippy --locked --workspace --all-targets --features dev-tools -- -D warnings

test:
  cargo test --locked --workspace --no-fail-fast --features dev-tools

doc:
  RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps

gitleaks:
  gitleaks git --no-banner

# Physical lines after rustfmt, implementation lines before the first
# #[cfg(test)]. A file over 500 total fails; 200 implementation and 300 total
# are warnings.
file-size:
  #!/usr/bin/env bash
  set -euo pipefail
  status=0
  while IFS= read -r f; do
    read -r impl total < <(awk '
      /^[[:space:]]*#\[cfg\(test\)\]/ && !seen { seen = 1 }
      !seen { impl++ }
      { total++ }
      END {
        if (FILENAME ~ /(^|\/)tests(\.rs|\/)/) impl = 0
        printf "%d %d\n", impl, total
      }' "$f")
    if (( total > 500 )); then
      printf 'FAIL %5d total %s\n' "$total" "$f"
      status=1
    elif (( total > 300 || impl > 200 )); then
      printf 'WARN %5d impl %5d total %s\n' "$impl" "$total" "$f"
    fi
  done < <(git ls-files 'crates/*.rs' 'crates/**/*.rs')
  exit "$status"

ship: fmt clippy test doc gitleaks file-size
```

`.github/workflows/ci.yml`:

```yaml
name: CI
on:
  push:
    branches:
      - main
  pull_request:
permissions:
  contents: read
jobs:
  gates:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          persist-credentials: false
      - name: Install just and gitleaks
        run: brew install gitleaks just
      - name: Toolchain
        run: rustup show active-toolchain
      - name: Format
        run: just fmt
      - name: Clippy
        run: just clippy
      - name: Test
        run: just test
      - name: Docs
        run: just doc
      - name: Secrets scan
        run: just gitleaks
      - name: File size
        run: just file-size
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo fmt --all`

Run: `cargo test -p vpt`

Expected: `test result: ok.` for the four unit tests in `args.rs`, the two tests in `version.rs` and the
one in `usage.rs`.

Run: `cargo fmt --all -- --check &&`
`cargo clippy --locked --workspace --all-targets --features dev-tools -- -D warnings`

Expected: no output from fmt; clippy finishes with `Finished` and no warnings.

- [ ] **Step 5: Commit**

Run: `cargo fmt --all`

```bash
git add Cargo.toml Cargo.lock rust-toolchain.toml README.md justfile .github crates
SKIP_AI_COMMIT=1 git commit -m "feat: workspace scaffold and vpt --version"
```

______________________________________________________________________

### Task 2: The output contract, result and error envelopes

**Files:**

- Create: `crates/vpt-protocol/src/result.rs`, `crates/vpt-protocol/src/error.rs`
- Modify: `crates/vpt-protocol/src/lib.rs`
- Create: `crates/vpt/src/cli/output.rs`
- Modify: `crates/vpt/src/cli/mod.rs`, `crates/vpt/src/lib.rs`
- Test: `crates/vpt/tests/usage.rs` (gains one test)

**Interfaces:**

- Consumes: `cli::args::{parse, UsageError, USAGE}` (Task 1).

- Produces:
  `vpt_protocol::result::document(command: &str, body: serde_json::Value) -> serde_json::Value`;
  `vpt_protocol::error::ErrorKind::{Refused, Usage, Config, Engine, Helper, Command, Store, Ledger}` with
  `ErrorKind::exit_code(self) -> i32`; `Check { pub name: String, pub ok: bool, pub detail: String }`
  (serde `Serialize`);
  `ErrorDocument { pub kind: ErrorKind, pub rule: Option<String>, pub message: String,`
  `pub ids: Vec<String>, pub completed: Vec<String>, pub checks: Option<Vec<Check>>,`
  `pub diagnostics: Vec<String> }` with
  `ErrorDocument::new(kind: ErrorKind, message: impl Into<String>) -> ErrorDocument`, the builder methods
  `rule(self, rule: &str) -> Self`, `ids(self, ids: Vec<String>) -> Self`,
  `completed(self, completed: Vec<String>) -> Self`, `checks(self, checks: Vec<Check>) -> Self`,
  `diagnostics(self, diagnostics: Vec<String>) -> Self`, and `exit_code(&self) -> i32`,
  `to_json(&self) -> serde_json::Value`;
  `cli::output::Outcome::{Success { document: serde_json::Value, human: String },`
  `Failure(ErrorDocument)}` and `emit(outcome: Outcome, json: bool) -> i32`, which writes stdout or
  stderr and returns the exit code (a stdout that cannot be written is a `Store` failure, exit 1).

- [ ] **Step 1: Write the failing tests**

`crates/vpt-protocol/src/lib.rs` declares both modules before the red run, so the test modules are
compiled and selected:

```rust
//! The versioned JSON documents vpt reads and writes across a process boundary.

pub mod error;
pub mod result;
```

`crates/vpt-protocol/src/error.rs` starts as its test module alone (Step 3 adds the code above it):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refusal_carries_its_rule_and_exits_3() {
        let document = ErrorDocument::new(ErrorKind::Refused, "engines apple and whisply both report family whisper")
            .rule("same_family");

        let json = document.to_json();

        assert_eq!(json["schema"], "vpt.error/1");
        assert_eq!(json["error"]["kind"], "refused");
        assert_eq!(json["error"]["rule"], "same_family");
        assert_eq!(json["error"]["ids"], serde_json::json!([]));
        assert_eq!(json["error"]["completed"], serde_json::json!([]));
        assert!(json["error"].get("checks").is_none());
        assert_eq!(ErrorKind::Refused.exit_code(), 3);
    }

    #[test]
    fn every_kind_maps_to_the_spec_exit_code() {
        assert_eq!(ErrorKind::Usage.exit_code(), 2);
        assert_eq!(ErrorKind::Config.exit_code(), 2);
        for kind in [ErrorKind::Engine, ErrorKind::Helper, ErrorKind::Command, ErrorKind::Store, ErrorKind::Ledger] {
            assert_eq!(kind.exit_code(), 1);
        }
    }

    #[test]
    fn a_non_refusal_has_a_null_rule() {
        let json = ErrorDocument::new(ErrorKind::Ledger, "database locked").to_json();
        assert!(json["error"]["rule"].is_null());
    }

    #[test]
    fn checks_appear_only_when_given() {
        let json = ErrorDocument::new(ErrorKind::Refused, "2 checks failed")
            .rule("doctor_checks")
            .checks(vec![Check { name: "helper".into(), ok: false, detail: "absent".into() }])
            .to_json();
        assert_eq!(json["error"]["checks"][0]["name"], "helper");
        assert_eq!(json["error"]["checks"][0]["ok"], false);
    }

    #[test]
    fn diagnostics_are_retained_inside_the_one_error_document_when_present() {
        let plain = ErrorDocument::new(ErrorKind::Store, "read failed").to_json();
        assert!(plain["error"].get("diagnostics").is_none());
        let reported = ErrorDocument::new(ErrorKind::Store, "read failed")
            .diagnostics(vec!["desktop notifications disabled: helper absent".into()])
            .to_json();
        assert_eq!(reported["error"]["diagnostics"], serde_json::json!([
            "desktop notifications disabled: helper absent"
        ]));
        assert_eq!(reported["error"]["kind"], "store");
    }
}
```

`crates/vpt-protocol/src/result.rs`, likewise the test module alone:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_result_document_leads_with_schema_and_command() {
        let json = document("storage", serde_json::json!({"stores": {}}));
        let text = json.to_string();
        assert!(text.starts_with(r#"{"schema":"vpt.result/1","command":"storage","#), "{text}");
    }
}
```

Append to `crates/vpt/tests/usage.rs`. The existing human-output test of Task 1 remains a regression
test; this is the new red acceptance test:

```rust
#[test]
fn an_unknown_verb_under_json_is_an_error_document_on_stderr() {
    let sandbox = Sandbox::new("usage-json");

    let output = run(sandbox.vpt().args(["--json", "transcode"]));

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout(&output), "");
    let document: serde_json::Value = serde_json::from_str(&stderr(&output)).expect("json on stderr");
    assert_eq!(document["schema"], "vpt.error/1");
    assert_eq!(document["error"]["kind"], "usage");
    assert_eq!(document["error"]["message"], "unknown verb transcode");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-protocol`

Expected: the build of the two test modules fails with `cannot find` for `ErrorDocument`, `ErrorKind`,
`Check` and `document`. The modules are compiled and selected; a run that selects zero tests, or that
succeeds, does not satisfy this step.

Run: `cargo test -p vpt --test usage`

Expected: `an_unknown_verb_under_json_is_an_error_document_on_stderr` FAILS: stderr holds usage text, not
JSON. The regression test of Task 1 still passes.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-protocol/src/error.rs` (above the test section):

```rust
//! `vpt.error/1`: the one document a failing command writes to stderr.

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Refused,
    Usage,
    Config,
    Engine,
    Helper,
    Command,
    Store,
    Ledger,
}

impl ErrorKind {
    pub fn exit_code(self) -> i32 {
        match self {
            ErrorKind::Refused => 3,
            ErrorKind::Usage | ErrorKind::Config => 2,
            ErrorKind::Engine | ErrorKind::Helper | ErrorKind::Command | ErrorKind::Store | ErrorKind::Ledger => 1,
        }
    }

    fn name(self) -> &'static str {
        match self {
            ErrorKind::Refused => "refused",
            ErrorKind::Usage => "usage",
            ErrorKind::Config => "config",
            ErrorKind::Engine => "engine",
            ErrorKind::Helper => "helper",
            ErrorKind::Command => "command",
            ErrorKind::Store => "store",
            ErrorKind::Ledger => "ledger",
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Check {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorDocument {
    pub kind: ErrorKind,
    pub rule: Option<String>,
    pub message: String,
    pub ids: Vec<String>,
    pub completed: Vec<String>,
    pub checks: Option<Vec<Check>>,
    pub diagnostics: Vec<String>,
}

impl ErrorDocument {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        ErrorDocument { kind, rule: None, message: message.into(), ids: vec![], completed: vec![], checks: None, diagnostics: vec![] }
    }

    pub fn rule(mut self, rule: &str) -> Self {
        self.rule = Some(rule.to_owned());
        self
    }

    pub fn ids(mut self, ids: Vec<String>) -> Self {
        self.ids = ids;
        self
    }

    pub fn completed(mut self, completed: Vec<String>) -> Self {
        self.completed = completed;
        self
    }

    pub fn checks(mut self, checks: Vec<Check>) -> Self {
        self.checks = Some(checks);
        self
    }

    pub fn diagnostics(mut self, diagnostics: Vec<String>) -> Self {
        self.diagnostics = diagnostics;
        self
    }

    pub fn exit_code(&self) -> i32 {
        self.kind.exit_code()
    }

    pub fn to_json(&self) -> Value {
        let mut error = serde_json::Map::new();
        error.insert("kind".into(), Value::String(self.kind.name().into()));
        error.insert("rule".into(), self.rule.clone().map_or(Value::Null, Value::String));
        error.insert("message".into(), Value::String(self.message.clone()));
        error.insert("ids".into(), serde_json::to_value(&self.ids).unwrap_or(Value::Null));
        error.insert("completed".into(), serde_json::to_value(&self.completed).unwrap_or(Value::Null));
        if let Some(checks) = &self.checks {
            error.insert("checks".into(), serde_json::to_value(checks).unwrap_or(Value::Null));
        }
        if !self.diagnostics.is_empty() {
            error.insert("diagnostics".into(), serde_json::to_value(&self.diagnostics).unwrap_or(Value::Null));
        }
        let mut document = serde_json::Map::new();
        document.insert("schema".into(), Value::String("vpt.error/1".into()));
        document.insert("error".into(), Value::Object(error));
        Value::Object(document)
    }
}
```

`crates/vpt-protocol/src/result.rs`:

```rust
//! `vpt.result/1`: the envelope of a result with no schema of its own.

use serde_json::{Map, Value};

pub fn document(command: &str, body: Value) -> Value {
    let mut ordered = Map::new();
    ordered.insert("schema".into(), Value::String("vpt.result/1".into()));
    ordered.insert("command".into(), Value::String(command.to_owned()));
    if let Value::Object(fields) = body {
        for (key, value) in fields {
            ordered.insert(key, value);
        }
    }
    Value::Object(ordered)
}
```

Because key order matters for the assertion, add `preserve_order` to the `serde_json` dependency of
`vpt-protocol`, `vpt-adapters` and `vpt`:

```toml
serde_json = { version = "1.0.151", features = ["preserve_order"] }
```

`crates/vpt/src/cli/output.rs`:

```rust
//! Output is withheld until the final status is known: one document on stdout
//! on success, one `vpt.error/1` on stderr on failure, and nothing else.

use std::io::Write;
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub enum Outcome {
    Success { document: serde_json::Value, human: String },
    Failure(ErrorDocument),
}

pub fn emit(outcome: Outcome, json: bool) -> i32 {
    match outcome {
        Outcome::Success { document, human } => {
            let text = if json { format!("{document}\n") } else { human };
            if std::io::stdout().write_all(text.as_bytes()).is_err() {
                return emit(Outcome::Failure(ErrorDocument::new(ErrorKind::Store, "standard output write failed")), json);
            }
            0
        }
        Outcome::Failure(error) => {
            let text = if json {
                format!("{}\n", error.to_json())
            } else {
                let mut text = format!("vpt: {}\n", error.message);
                for diagnostic in &error.diagnostics {
                    text.push_str(&format!("vpt: {diagnostic}\n"));
                }
                text
            };
            let _ = std::io::stderr().write_all(text.as_bytes());
            error.exit_code()
        }
    }
}
```

`crates/vpt/src/cli/mod.rs`:

```rust
pub(crate) mod args;
pub(crate) mod output;
```

`crates/vpt/src/lib.rs` (replace `run` and `dispatch`):

```rust
//! The command crate: parse, dispatch, emit, exit. No policy lives here.

mod cli;
mod commands;

use cli::args::{Invocation, USAGE, Verb, parse};
use cli::output::{Outcome, emit};
use std::io::Write;
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub fn run() -> ! {
    let args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let json = args.iter().any(|word| word == "--json");
    let code = match parse(args) {
        Ok(invocation) => dispatch(&invocation),
        Err(error) => {
            let code = emit(Outcome::Failure(ErrorDocument::new(ErrorKind::Usage, error.0)), json);
            if !json {
                let _ = std::io::stderr().write_all(USAGE.as_bytes());
            }
            code
        }
    };
    std::process::exit(code);
}

fn dispatch(invocation: &Invocation) -> i32 {
    let outcome = match &invocation.verb {
        Verb::Version => Outcome::Success {
            document: commands::version::document(None),
            human: commands::version::human(None),
        },
        _ => Outcome::Failure(ErrorDocument::new(ErrorKind::Usage, "verb not implemented yet")),
    };
    emit(outcome, invocation.json)
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo fmt --all`

Run: `cargo test -p vpt-protocol -p vpt`

Expected: all tests in `error.rs`, `result.rs`, `args.rs`, `version.rs` and `usage.rs` PASS.

- [ ] **Step 5: Commit**

Run: `cargo fmt --all`

```bash
git add crates Cargo.lock
SKIP_AI_COMMIT=1 git commit -m "feat: result and error envelopes with the exit code contract"
```

______________________________________________________________________

### Task 3: The configuration key table, the setup template and the loader

One authoritative definition of every key (name, kind, default, comment, secret) drives the rendered
template, the unknown-key refusal and the validator. The loader parses the operator's file, refuses an
unknown key, an unknown table or a missing `config_version`, and merges the file over the rendered
defaults, so every key the application reads is present. A parse failure is reported by a fixed sentence,
never by the parser's own message, because that message can quote the rejected line.

**Files:**

- Create: `crates/vpt-adapters/src/config/mod.rs`, `crates/vpt-adapters/src/config/schema.rs`,
  `crates/vpt-adapters/src/config/schema/types.rs`, `crates/vpt-adapters/src/config/schema/dynamic.rs`,
  `crates/vpt-adapters/src/config/render.rs`, `crates/vpt-adapters/src/config/load.rs`
- Modify: `crates/vpt-adapters/src/lib.rs`

**Interfaces:**

- Consumes: nothing.

- Produces, re-exported from `vpt_adapters::config` (the files below `config/` are private modules):
  `Kind::{Bool, PositiveInt, NonNegativeInt, PositiveInt64, Ratio, RatioAboveZero, Duration,`
  `Enum(&'static [&'static str]), Text, Secret, TextList, Argv}` with
  `Kind::expected(self) -> &'static str` (the type name a wrong-type refusal reports);
  `KeySpec { pub path: &'static str, pub kind: Kind, pub default: &'static str,`
  `pub comment: &'static str }`; `KEYS: &[KeySpec]`; `STORE_KEYS: &[&str]`;
  `DynamicTable { pub prefix: &'static str, pub leaves: &'static [(&'static str, Kind)] }`;
  `DYNAMIC_TABLES: &[DynamicTable]`; `spec_for(path: &str) -> Option<Kind>`;
  `template(main_engine: &str, state_dir: &str) -> String`; `CONFIG_VERSION: i64`;
  `ConfigError::{Missing(String), Unreadable { path: String, detail: String }, Unparseable(String),`
  `MissingVersion, UnsupportedVersion(i64), UnknownKey(String), WrongType { key: String,`
  `expected: String },` `OutOfRange { key: String, rule: String }}`;
  `load_text(text: &str) -> Result<toml::Table, ConfigError>`;
  `load_file(path: &Path) -> Result<toml::Table, ConfigError>`;
  `get<'a>(table: &'a toml::Table, path: &str) -> Option<&'a toml::Value>`.

- [ ] **Step 1: Write the failing tests**

Declare every module first so the test modules are compiled and selected by the red run.
`crates/vpt-adapters/src/lib.rs`:

```rust
//! Concrete adapters: filesystem, SQLite, processes and configuration.

pub mod config;
```

`crates/vpt-adapters/src/config/mod.rs`:

```rust
//! Configuration: one key table, the template rendered from it, the loader
//! that refuses what the table does not name, and the validator.

mod load;
mod render;
mod schema;

pub use load::{CONFIG_VERSION, ConfigError, get, load_file, load_text};
pub use render::template;
pub use schema::{DYNAMIC_TABLES, DynamicTable, KEYS, KeySpec, Kind, STORE_KEYS, spec_for};
```

`crates/vpt-adapters/src/config/schema.rs` starts as its doc line, `//! The configuration key table.`,
and `crates/vpt-adapters/src/config/load.rs` starts as its test module alone:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{KEYS, template};

    #[test]
    fn the_rendered_template_loads_and_every_key_of_the_table_is_present() {
        let table = load_text(&template("apple", "~/.local/state/vpt")).expect("template loads");
        for key in KEYS {
            assert!(get(&table, key.path).is_some(), "{} missing after load", key.path);
        }
        assert_eq!(get(&table, "engines.main").and_then(|v| v.as_str()), Some("apple"));
    }

    #[test]
    fn an_operator_value_wins_over_the_default() {
        let table = load_text("config_version = 1\n[source]\nquiet_period_secs = 90\n").expect("loads");
        assert_eq!(get(&table, "source.quiet_period_secs").and_then(|v| v.as_integer()), Some(90));
        assert_eq!(get(&table, "source.deferral_page_threshold").and_then(|v| v.as_integer()), Some(4));
    }

    #[test]
    fn an_unknown_key_is_refused_by_its_full_path() {
        let error = load_text("config_version = 1\n[source]\nquiet_period = 90\n").unwrap_err();
        assert_eq!(error, ConfigError::UnknownKey("source.quiet_period".into()));
    }

    #[test]
    fn an_unknown_table_is_refused_even_when_empty() {
        assert_eq!(load_text("config_version = 1\n[unknown]\n").unwrap_err(), ConfigError::UnknownKey("unknown".into()));
        assert_eq!(load_text("config_version = 1\n[source.extra]\n").unwrap_err(), ConfigError::UnknownKey("source.extra".into()));
    }

    #[test]
    fn a_table_where_a_scalar_belongs_is_a_wrong_type() {
        let error = load_text("config_version = 1\n[source]\nquiet_period_secs = {}\n").unwrap_err();
        assert_eq!(error, ConfigError::WrongType { key: "source.quiet_period_secs".into(), expected: "integer".into() });
    }

    #[test]
    fn a_missing_config_version_is_refused() {
        assert_eq!(load_text("[source]\nread_titles = false\n").unwrap_err(), ConfigError::MissingVersion);
    }

    #[test]
    fn a_future_config_version_is_refused_with_its_value() {
        assert_eq!(load_text("config_version = 2\n").unwrap_err(), ConfigError::UnsupportedVersion(2));
    }

    #[test]
    fn an_engine_table_and_a_language_pair_are_dynamic_keys() {
        let text = "config_version = 1\n[engines.cloudy]\nkind = \"command\"\ncommand = [\"x\"]\n\
                    family = \"whisper\"\nlocal = false\n[engines.by_language.de-DE]\nmain = \"cloudy\"\n\
                    [retention.hold]\naudio = \"90d\"\n";
        let table = load_text(text).expect("dynamic keys load");
        assert_eq!(get(&table, "engines.cloudy.family").and_then(|v| v.as_str()), Some("whisper"));
        assert_eq!(get(&table, "engines.by_language.de-DE.main").and_then(|v| v.as_str()), Some("cloudy"));
    }

    #[test]
    fn a_dynamic_table_still_refuses_a_leaf_it_does_not_know() {
        let error = load_text("config_version = 1\n[engines.cloudy]\nkind = \"command\"\nspeed = 3\n").unwrap_err();
        assert_eq!(error, ConfigError::UnknownKey("engines.cloudy.speed".into()));
    }

    #[test]
    fn a_hold_for_an_unknown_store_is_refused() {
        let error = load_text("config_version = 1\n[retention.hold]\nvideos = \"1d\"\n").unwrap_err();
        assert_eq!(error, ConfigError::UnknownKey("retention.hold.videos".into()));
    }

    #[test]
    fn unparseable_toml_is_refused_without_quoting_the_text() {
        let text = "config_version = 1\n[context.google]\nclient_secret = \"canary-7Q9x-secret\n";
        let error = load_text(text).unwrap_err();
        assert_eq!(error, ConfigError::Unparseable("invalid TOML syntax in configuration".into()));
        assert!(!format!("{error:?}").contains("canary"));
    }

    #[test]
    fn a_state_path_with_a_quote_and_a_backslash_round_trips_through_the_template() {
        let table = load_text(&template("apple", "/tmp/say \"hi\"\\now")).expect("loads");
        assert_eq!(get(&table, "home.state_dir").and_then(|v| v.as_str()), Some("/tmp/say \"hi\"\\now"));
    }
}
```

`crates/vpt-adapters/src/config/render.rs`, likewise the test module alone:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_template_starts_with_the_version_and_names_the_chosen_engine() {
        let text = template("whisply", "/state/vpt");
        assert!(text.starts_with("config_version = 1"), "{text}");
        assert!(text.contains("\nmain = \"whisply\""), "{text}");
        assert!(text.contains("state_dir = \"/state/vpt\""), "{text}");
    }

    #[test]
    fn every_key_line_carries_a_comment() {
        for line in template("apple", "~/.local/state/vpt").lines() {
            if line.contains(" = ") {
                assert!(line.contains(" # "), "uncommented key line: {line}");
            }
        }
    }

    #[test]
    fn a_secret_key_is_marked_in_its_comment() {
        let text = template("apple", "~/.local/state/vpt");
        assert!(text.contains("client_secret = \"\" # OAuth client secret (secret)"), "{text}");
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-adapters config`

Expected: the build of the two test modules fails with `cannot find` for `KEYS`, `template`, `load_text`,
`get` and `ConfigError`. The modules are compiled and selected; a run that selects zero tests, or that
succeeds, does not satisfy this step.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-adapters/src/config/schema/types.rs`:

```rust
//! The kinds a key can have, and the shape of one key's definition.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Bool,
    /// A timeout, a threshold or a length: a positive 32-bit integer.
    PositiveInt,
    /// A count, a gap or a lookback: a non-negative 32-bit integer.
    NonNegativeInt,
    /// `source.max_audio_bytes` alone: a positive 64-bit integer.
    PositiveInt64,
    /// A finite number in `[0, 1]`.
    Ratio,
    /// A finite number in `(0, 1]`.
    RatioAboveZero,
    /// `0` or a positive decimal integer with one of `s`, `m`, `h`, `d`.
    Duration,
    Enum(&'static [&'static str]),
    Text,
    Secret,
    TextList,
    Argv,
}

impl Kind {
    /// The type name a wrong-type refusal reports.
    pub fn expected(self) -> &'static str {
        match self {
            Kind::Bool => "boolean",
            Kind::PositiveInt | Kind::NonNegativeInt | Kind::PositiveInt64 => "integer",
            Kind::Ratio | Kind::RatioAboveZero => "number",
            Kind::Duration => "duration",
            Kind::Enum(_) | Kind::Text | Kind::Secret => "string",
            Kind::TextList | Kind::Argv => "list of strings",
        }
    }
}

pub struct KeySpec {
    pub path: &'static str,
    pub kind: Kind,
    pub default: &'static str,
    pub comment: &'static str,
}

pub const STORE_KEYS: &[&str] =
    &["audio", "transcripts", "analysis", "briefs", "engine_outputs", "drafts", "released"];

pub(super) const fn key(path: &'static str, kind: Kind, default: &'static str, comment: &'static str) -> KeySpec {
    KeySpec { path, kind, default, comment }
}
```

`crates/vpt-adapters/src/config/schema/dynamic.rs`:

```rust
//! The tables whose middle segment the operator names, and the lookup that
//! answers what kind a dotted path has.

use super::{KEYS, Kind};

/// `engines.<name>.*` and `engines.by_language.<lang>.*`. The leaf must be one of `leaves`.
pub struct DynamicTable {
    pub prefix: &'static str,
    pub leaves: &'static [(&'static str, Kind)],
}

pub const DYNAMIC_TABLES: &[DynamicTable] = &[
    DynamicTable { prefix: "engines.by_language.", leaves: &[("main", Kind::Text), ("checker", Kind::Text)] },
    DynamicTable {
        prefix: "engines.",
        leaves: &[
            ("kind", Kind::Enum(&["apple", "whisply", "command"])),
            ("command", Kind::Argv),
            ("family", Kind::Text),
            ("local", Kind::Bool),
            ("device", Kind::Text),
            ("model", Kind::Text),
        ],
    },
];

pub fn spec_for(path: &str) -> Option<Kind> {
    if let Some(known) = KEYS.iter().find(|key| key.path == path) {
        return Some(known.kind);
    }
    for table in DYNAMIC_TABLES {
        if let Some(rest) = path.strip_prefix(table.prefix) {
            let mut parts = rest.splitn(2, '.');
            let (Some(name), Some(leaf)) = (parts.next(), parts.next()) else { continue };
            if name.is_empty() || leaf.contains('.') {
                continue;
            }
            if let Some((_, kind)) = table.leaves.iter().find(|(known, _)| *known == leaf) {
                return Some(*kind);
            }
        }
    }
    None
}

/// Whether a table at `path` is one the schema declares: a prefix of a static
/// key, a dynamic table's parent, or a dynamic table itself.
pub fn table_is_declared(path: &str) -> bool {
    let as_prefix = format!("{path}.");
    KEYS.iter().any(|key| key.path.starts_with(&as_prefix))
        || DYNAMIC_TABLES.iter().any(|table| {
            table.prefix.starts_with(&as_prefix)
                || path.strip_prefix(table.prefix).is_some_and(|name| !name.is_empty() && !name.contains('.'))
        })
}
```

`crates/vpt-adapters/src/config/schema.rs`:

```rust
//! The one authoritative definition of every configuration key. The renderer,
//! the loader, the validator and the settings reader all read this table.

mod dynamic;
mod types;

pub use dynamic::{DYNAMIC_TABLES, DynamicTable, spec_for, table_is_declared};
pub use types::{KeySpec, Kind, STORE_KEYS};
use types::key;

/// Every key of spec section 10, in the order the template renders them.
pub const KEYS: &[KeySpec] = &[
    key("config_version", Kind::PositiveInt, "1", "the configuration schema version"),
    key("home.path", Kind::Text, "\"~/.vpt\"", "the home; every store defaults under it"),
    key("home.state_dir", Kind::Text, "\"~/.local/state/vpt\"", "the ledger's directory"),
    key("home.symlink_target", Kind::Text, "\"\"", "when set, ~/.vpt is a managed symlink to this directory"),
    key("helper.path", Kind::Text, "\"vpt-macos\"", "the macOS helper, resolved on PATH when relative"),
    key("stores.audio", Kind::Text, "\"<home>/audio\"", "archive clones"),
    key("stores.transcripts", Kind::Text, "\"<home>/transcripts\"", "transcript notes"),
    key("stores.analysis", Kind::Text, "\"<home>/analysis\"", "synthesis notes"),
    key("stores.briefs", Kind::Text, "\"<home>/briefs\"", "briefs"),
    key("stores.engine_outputs", Kind::Text, "\"<home>/engine-outputs\"", "raw engine documents"),
    key("stores.drafts", Kind::Text, "\"<home>/drafts\"", "private redaction reports"),
    key("stores.released", Kind::Text, "\"<home>/released\"", "the shared folder"),
    key(
        "source.recordings_dir",
        Kind::Text,
        "\"~/Library/Group Containers/group.com.apple.VoiceMemos.shared/Recordings\"",
        "Apple's store, read read-only",
    ),
    key("source.read_titles", Kind::Bool, "true", "read titles from a private copy of the database"),
    key("source.quiet_period_secs", Kind::PositiveInt, "30", "seconds a file must be unchanged before it is at rest"),
    key("source.deferral_page_threshold", Kind::PositiveInt, "4", "consecutive deferrals of one recording before one event"),
    key("source.max_audio_bytes", Kind::PositiveInt64, "2147483648", "a larger source file defers before any content is read"),
    key("recording.language", Kind::Text, "\"en-US\"", "the language engines are asked for"),
    key("engines.main", Kind::Text, "\"apple\"", "the transcript of record"),
    key("engines.checker", Kind::Text, "\"\"", "the second slot; empty runs one engine"),
    key("engines.checker_runs", Kind::Enum(&["always", "on_failure"]), "\"always\"", "always or on_failure"),
    key("engines.allow_same_family", Kind::Bool, "false", "let two engines of one family run"),
    key("engines.timeout_secs", Kind::PositiveInt, "1800", "per-engine wall-clock deadline"),
    key("engines.apple.kind", Kind::Enum(&["apple"]), "\"apple\"", "the Apple Speech adapter through the helper"),
    key("engines.whisply.kind", Kind::Enum(&["whisply"]), "\"whisply\"", "the whisply adapter"),
    key("engines.whisply.command", Kind::Argv, "[\"whisply\"]", "how whisply is invoked"),
    key("engines.whisply.device", Kind::Text, "\"mlx\"", "mlx reports no confidence, cpu does"),
    key("engines.whisply.model", Kind::Text, "\"large-v3-turbo\"", "the model name"),
    key("reconcile.confidence_floor", Kind::Ratio, "0.35", "word confidence below this is flagged"),
    key("reconcile.max_divergence_ratio", Kind::RatioAboveZero, "0.35", "past this edit-distance ratio, one divergent flag"),
    key("reconcile.known_terms_path", Kind::Text, "\"~/.config/vpt/known-terms.txt\"", "the confirmed-terms list"),
    key("readability.enabled", Kind::Bool, "false", "the readability pass over the note body"),
    key("note.profile", Kind::Enum(&["portable", "obsidian"]), "\"portable\"", "portable or obsidian"),
    key("note.link_style", Kind::Enum(&["markdown", "wiki"]), "\"markdown\"", "markdown or wiki"),
    key("note.slug_max_chars", Kind::PositiveInt, "60", "slug length cap"),
    key("note.obsidian.hub_transcripts", Kind::Text, "\"transcripts\"", "folder note name for hub"),
    key("note.obsidian.hub_analysis", Kind::Text, "\"analysis\"", "folder note name for hub"),
    key("note.obsidian.hub_briefs", Kind::Text, "\"briefs\"", "folder note name for hub"),
    key("note.obsidian.vault_root", Kind::Text, "\"\"", "the vault's root directory, required by the obsidian profile"),
    key("tags.new_tags", Kind::Enum(&["write", "hold"]), "\"hold\"", "write (automatic) or hold (suggested until confirmed)"),
    key("tags.known_tags_path", Kind::Text, "\"~/.config/vpt/known-tags.txt\"", "the confirmed-tags list"),
    key("tags.max_per_note", Kind::NonNegativeInt, "5", "confirmed tags per note"),
    key("tags.max_suggested", Kind::NonNegativeInt, "10", "held tags per note"),
    key("relations.session_gap_minutes", Kind::NonNegativeInt, "60", "continues window"),
    key("relations.max_suggested", Kind::NonNegativeInt, "5", "agent-proposed related links kept per note"),
    key("synthesis.command", Kind::Argv, "[]", "the agent command; empty disables synthesis only"),
    key("synthesis.timeout_secs", Kind::PositiveInt, "600", "the command's deadline"),
    key("synthesis.grounding_window_secs", Kind::PositiveInt, "15", "verify-note's tolerance around a cited span"),
    key("brief.lookback_days", Kind::NonNegativeInt, "180", "how far back selection reaches"),
    key("brief.max_notes", Kind::NonNegativeInt, "12", "selection cap"),
    key("brief.max_spans_per_note", Kind::NonNegativeInt, "5", "quoted spans per selected note"),
    key("brief.trigger.enabled", Kind::Bool, "false", "the calendar trigger behind --upcoming"),
    key("brief.trigger.lead_time", Kind::Duration, "\"24h\"", "how far ahead --upcoming looks"),
    key("context.type", Kind::Enum(&["none", "dam", "google"]), "\"none\"", "none, dam or google"),
    key("context.calendars", Kind::TextList, "[]", "dam path prefixes or Google calendar ids; empty refuses"),
    key("context.projects", Kind::TextList, "[]", "dam task path prefixes; empty refuses"),
    key("context.dam.command", Kind::Argv, "[\"dam\"]", "how dam is invoked"),
    key("context.google.client_id", Kind::Secret, "\"\"", "OAuth client id"),
    key("context.google.client_secret", Kind::Secret, "\"\"", "OAuth client secret"),
    key("context.google.refresh_token", Kind::Secret, "\"\"", "a refresh token bearing only the read-only events scope"),
    key("share.automatic", Kind::Bool, "false", "redact every recording's notes during vpt run"),
    key(
        "share.source_line",
        Kind::Text,
        "\"redacted extract, not a verbatim record\"",
        "the one frontmatter line telling a recipient what they hold",
    ),
    key("share.include_date", Kind::Bool, "false", "put the capture date in the released frontmatter"),
    key("share.keep_timecodes", Kind::Bool, "false", "keep [mm:ss] references in released text"),
    key("share.deny_sections", Kind::TextList, "[]", "sections dropped from the content region before redaction"),
    key(
        "share.redact.classes",
        Kind::TextList,
        "[\"email\", \"url\", \"phone\", \"number\", \"money\"]",
        "pattern classes; terms and note names are always masked",
    ),
    key("share.redact.mask", Kind::Text, "\"[{class} {n}]\"", "rendered in place of a removed value"),
    key("share.redact.flagged_spans", Kind::Enum(&["omit", "mark"]), "\"omit\"", "omit or mark"),
    key("handoff.allow_private", Kind::Bool, "false", "let transcripts, analysis notes and briefs be handed off"),
    key("notify.mode", Kind::Enum(&["desktop", "command", "off"]), "\"desktop\"", "desktop, command or off"),
    key(
        "notify.command",
        Kind::Argv,
        "[]",
        "argv with {event}, {state}, {id}, {detail}, {count}, {path} tokens; receives vpt's JSON on stdin",
    ),
    key("notify.aggregate_after", Kind::PositiveInt, "3", "recordings needing review in one run before one aggregate event"),
    key("retention.enabled", Kind::Bool, "false", "the opt-in retention run"),
    key("retention.include_audio", Kind::Bool, "false", "let the audio store expire"),
    key("retention.hold.audio", Kind::Duration, "0", "files older than this move to the Trash; 0 never"),
    key("retention.hold.transcripts", Kind::Duration, "0", "as above"),
    key("retention.hold.analysis", Kind::Duration, "0", "as above"),
    key("retention.hold.briefs", Kind::Duration, "0", "as above"),
    key("retention.hold.engine_outputs", Kind::Duration, "0", "as above"),
    key("retention.hold.drafts", Kind::Duration, "0", "as above"),
    key("retention.hold.released", Kind::Duration, "0", "as above"),
];
```

The four counts `tags.max_per_note`, `tags.max_suggested`, `brief.max_notes` and
`brief.max_spans_per_note` are `NonNegativeInt`: spec section 10 makes a count non-negative, so zero is a
valid value for each. `config/schema.rs` formats to about 470 lines with rustfmt; the two child modules
keep it under the 500-line cap.

`crates/vpt-adapters/src/config/render.rs`, above its test module:

```rust
//! The file `vpt setup` writes: every key at its default, one comment each.

use super::schema::{KEYS, Kind};

pub fn template(main_engine: &str, state_dir: &str) -> String {
    let mut text = String::new();
    let mut current_table = String::new();
    for key in KEYS {
        let (table, leaf) = split_table(key.path);
        if table != current_table {
            if !table.is_empty() {
                text.push_str(&format!("\n[{table}]\n"));
            }
            current_table = table.to_owned();
        }
        let value = match key.path {
            "engines.main" => quoted(main_engine),
            "home.state_dir" => quoted(state_dir),
            _ => key.default.to_owned(),
        };
        let secret = if key.kind == Kind::Secret { " (secret)" } else { "" };
        text.push_str(&format!("{leaf} = {value} # {}{secret}\n", key.comment));
    }
    text
}

/// A TOML basic string. JSON string escaping is a subset of TOML's, except
/// that TOML refuses a raw DEL character, which JSON leaves unescaped.
fn quoted(value: &str) -> String {
    serde_json::to_string(value).expect("strings serialize").replace('\u{7f}', "\\u007f")
}

fn split_table(path: &str) -> (&str, &str) {
    match path.rfind('.') {
        Some(index) => (&path[..index], &path[index + 1..]),
        None => ("", path),
    }
}
```

`config_version` sits in the empty table, which renders no header, so the text starts with
`config_version = 1`.

`crates/vpt-adapters/src/config/load.rs`, above its test module:

```rust
//! Read the operator's file, refuse what the key table does not name, and merge
//! it over the rendered defaults so every key is present afterwards.

use super::render::template;
use super::schema::{STORE_KEYS, spec_for, table_is_declared};
use std::path::Path;
use toml::{Table, Value};

pub const CONFIG_VERSION: i64 = 1;
const UNPARSEABLE: &str = "invalid TOML syntax in configuration";

#[derive(Debug, PartialEq, Eq)]
pub enum ConfigError {
    Missing(String),
    Unreadable { path: String, detail: String },
    Unparseable(String),
    MissingVersion,
    UnsupportedVersion(i64),
    UnknownKey(String),
    WrongType { key: String, expected: String },
    OutOfRange { key: String, rule: String },
}

pub fn load_file(path: &Path) -> Result<Table, ConfigError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(ConfigError::Missing(path.display().to_string()));
        }
        Err(error) => {
            return Err(ConfigError::Unreadable { path: path.display().to_string(), detail: error.kind().to_string() });
        }
    };
    load_text(&text)
}

pub fn load_text(text: &str) -> Result<Table, ConfigError> {
    let operator: Table = text.parse().map_err(|_: toml::de::Error| ConfigError::Unparseable(UNPARSEABLE.into()))?;
    match operator.get("config_version") {
        None => return Err(ConfigError::MissingVersion),
        Some(Value::Integer(CONFIG_VERSION)) => {}
        Some(Value::Integer(other)) => return Err(ConfigError::UnsupportedVersion(*other)),
        Some(_) => return Err(ConfigError::WrongType { key: "config_version".into(), expected: "integer".into() }),
    }
    refuse_unknown_keys(&operator, "")?;
    let mut merged: Table = template("apple", "~/.local/state/vpt")
        .parse()
        .map_err(|_: toml::de::Error| ConfigError::Unparseable(UNPARSEABLE.into()))?;
    merge_into(&mut merged, operator);
    Ok(merged)
}

fn refuse_unknown_keys(table: &Table, prefix: &str) -> Result<(), ConfigError> {
    for (name, value) in table {
        let path = if prefix.is_empty() { name.clone() } else { format!("{prefix}.{name}") };
        match value {
            Value::Table(inner) => {
                if let Some(kind) = spec_for(&path) {
                    return Err(ConfigError::WrongType { key: path, expected: kind.expected().into() });
                }
                if !table_is_declared(&path) {
                    return Err(ConfigError::UnknownKey(path));
                }
                refuse_unknown_keys(inner, &path)?;
            }
            _ => {
                if let Some(store) = path.strip_prefix("retention.hold.") {
                    if !STORE_KEYS.contains(&store) {
                        return Err(ConfigError::UnknownKey(path));
                    }
                } else if spec_for(&path).is_none() {
                    return Err(ConfigError::UnknownKey(path));
                }
            }
        }
    }
    Ok(())
}

fn merge_into(base: &mut Table, over: Table) {
    for (name, value) in over {
        match (base.get_mut(&name), value) {
            (Some(Value::Table(existing)), Value::Table(incoming)) => merge_into(existing, incoming),
            (_, incoming) => {
                base.insert(name, incoming);
            }
        }
    }
}

/// Read a leaf by its dotted path.
pub fn get<'a>(table: &'a Table, path: &str) -> Option<&'a Value> {
    let mut segments = path.split('.');
    let first = segments.next()?;
    let mut current = table.get(first)?;
    for segment in segments {
        current = current.as_table()?.get(segment)?;
    }
    Some(current)
}
```

The unparseable message is the same fixed sentence on both parse sites, and `Unreadable` carries the
error kind rather than the operating system's text, so no refusal quotes the configuration.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo fmt --all`

Run: `cargo test -p vpt-adapters config`

Expected: 15 tests PASS.

- [ ] **Step 5: Commit**

Run: `cargo fmt --all`

```bash
git add crates/vpt-adapters
SKIP_AI_COMMIT=1 git commit -m "feat(config): the key table, the setup template and the loader"
```

______________________________________________________________________

### Task 4: Value rules and the duration syntax

**Files:**

- Create: `crates/vpt-domain/src/duration.rs`
- Modify: `crates/vpt-domain/src/lib.rs`
- Create: `crates/vpt-adapters/src/config/validate.rs`
- Modify: `crates/vpt-adapters/src/config/mod.rs`, `crates/vpt-adapters/src/config/load.rs`

**Interfaces:**

- Consumes: `config::{spec_for, Kind, Kind::expected, ConfigError, load_text}` (Task 3).

- Produces: `vpt_domain::duration::parse_duration(text: &str) -> Result<u64, DurationError>` (seconds)
  and `DurationError::{MissingUnit, UnknownUnit(char), NotAPositiveInteger, Overflow}`;
  `ConfigError::key(&self) -> Option<&str>` (the key a refusal names);
  `config::validate::validate(table: &toml::Table) -> Result<(), ConfigError>`, a private module of
  `config` called at the end of `load_text`.

- [ ] **Step 1: Write the failing tests**

Declare the modules first: `crates/vpt-domain/src/lib.rs` gains `pub mod duration;` and
`crates/vpt-adapters/src/config/mod.rs` gains `mod validate;`, so both test modules are compiled and
selected by the red run. `crates/vpt-domain/src/duration.rs` starts as its test module alone:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_means_never_and_needs_no_unit() {
        assert_eq!(parse_duration("0"), Ok(0));
    }

    #[test]
    fn each_unit_converts_to_seconds() {
        assert_eq!(parse_duration("45s"), Ok(45));
        assert_eq!(parse_duration("2m"), Ok(120));
        assert_eq!(parse_duration("24h"), Ok(86_400));
        assert_eq!(parse_duration("90d"), Ok(7_776_000));
    }

    #[test]
    fn a_positive_number_without_a_unit_is_refused() {
        assert_eq!(parse_duration("90"), Err(DurationError::MissingUnit));
    }

    #[test]
    fn an_unknown_unit_and_a_negative_number_are_refused() {
        assert_eq!(parse_duration("3w"), Err(DurationError::UnknownUnit('w')));
        assert_eq!(parse_duration("-3d"), Err(DurationError::NotAPositiveInteger));
        assert_eq!(parse_duration("0d"), Err(DurationError::NotAPositiveInteger));
    }

    #[test]
    fn overflow_is_refused_rather_than_wrapped() {
        assert_eq!(parse_duration("999999999999999999d"), Err(DurationError::Overflow));
    }
}
```

`crates/vpt-adapters/src/config/validate.rs`, likewise the test module alone:

```rust
#[cfg(test)]
mod tests {
    use crate::config::{ConfigError, load_text};

    fn with(body: &str) -> Result<toml::Table, ConfigError> {
        load_text(&format!("config_version = 1\n{body}"))
    }

    #[test]
    fn a_ratio_outside_its_range_is_refused_naming_the_key() {
        let error = with("[reconcile]\nconfidence_floor = 1.5\n").unwrap_err();
        assert_eq!(error, ConfigError::OutOfRange { key: "reconcile.confidence_floor".into(), rule: "a ratio in [0, 1]".into() });
    }

    #[test]
    fn a_ratio_accepts_integer_endpoints_and_refuses_a_non_finite_number() {
        assert!(with("[reconcile]\nconfidence_floor = 0\n").is_ok());
        assert!(with("[reconcile]\nconfidence_floor = 1\n").is_ok());
        assert!(with("[reconcile]\nmax_divergence_ratio = 1\n").is_ok());
        assert_eq!(with("[reconcile]\nconfidence_floor = inf\n").unwrap_err().key(), Some("reconcile.confidence_floor"));
    }

    #[test]
    fn the_divergence_ratio_must_be_above_zero() {
        let error = with("[reconcile]\nmax_divergence_ratio = 0.0\n").unwrap_err();
        assert_eq!(error.key(), Some("reconcile.max_divergence_ratio"));
        assert_eq!(with("[reconcile]\nmax_divergence_ratio = 0\n").unwrap_err().key(), Some("reconcile.max_divergence_ratio"));
    }

    #[test]
    fn a_positive_integer_refuses_zero_and_accepts_the_32_bit_maximum() {
        let error = with("[source]\nquiet_period_secs = 0\n").unwrap_err();
        assert_eq!(error, ConfigError::OutOfRange { key: "source.quiet_period_secs".into(), rule: "a positive 32-bit integer".into() });
        assert!(with("[source]\nquiet_period_secs = 4294967295\n").is_ok());
    }

    #[test]
    fn a_count_accepts_zero_and_the_32_bit_maximum_and_refuses_what_lies_outside() {
        for key in ["tags]\nmax_per_note", "tags]\nmax_suggested", "brief]\nmax_notes", "brief]\nmax_spans_per_note", "relations]\nsession_gap_minutes"] {
            assert!(with(&format!("[{key} = 0\n")).is_ok(), "{key} = 0");
            assert!(with(&format!("[{key} = 4294967295\n")).is_ok(), "{key} = u32::MAX");
            assert!(with(&format!("[{key} = -1\n")).is_err(), "{key} = -1");
            assert!(with(&format!("[{key} = 4294967296\n")).is_err(), "{key} = u32::MAX + 1");
        }
    }

    #[test]
    fn a_32_bit_key_refuses_a_value_above_32_bits() {
        assert_eq!(with("[engines]\ntimeout_secs = 4294967296\n").unwrap_err().key(), Some("engines.timeout_secs"));
        assert!(with("[source]\nmax_audio_bytes = 4294967296\n").is_ok());
    }

    #[test]
    fn a_wrong_type_is_refused_naming_the_expected_type() {
        let error = with("[source]\nread_titles = \"yes\"\n").unwrap_err();
        assert_eq!(error, ConfigError::WrongType { key: "source.read_titles".into(), expected: "boolean".into() });
    }

    #[test]
    fn an_enum_refuses_a_value_outside_its_set() {
        let error = with("[notify]\nmode = \"loud\"\n").unwrap_err();
        assert_eq!(error, ConfigError::OutOfRange { key: "notify.mode".into(), rule: "one of desktop, command, off".into() });
    }

    #[test]
    fn a_duration_accepts_the_integer_zero_and_the_unit_form() {
        assert!(with("[retention.hold]\naudio = 0\n").is_ok());
        assert!(with("[retention.hold]\naudio = \"90d\"\n").is_ok());
        assert_eq!(with("[retention.hold]\naudio = \"90\"\n").unwrap_err().key(), Some("retention.hold.audio"));
    }

    #[test]
    fn an_argv_must_be_a_list_of_strings() {
        assert_eq!(with("[notify]\ncommand = [1, 2]\n").unwrap_err().key(), Some("notify.command"));
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-domain duration`

Expected: the build of the `duration::tests` module fails with `cannot find` for `parse_duration` and
`DurationError`. The module is compiled and selected; a run that selects zero tests, or that succeeds,
does not satisfy this step.

Run: `cargo test -p vpt-adapters validate`

Expected: the build of the `validate::tests` module fails, `no method named key` on `ConfigError`. Once
Step 3 adds `key` and nothing else, every test but the `is_ok` assertions FAILS because `load_text`
accepts anything the table names; the whole of Step 3 turns them green.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-domain/src/duration.rs`:

```rust
//! The configuration duration syntax: `0`, or a positive decimal integer
//! followed by `s`, `m`, `h` or `d`, converted to seconds with overflow checked.

#[derive(Debug, PartialEq, Eq)]
pub enum DurationError {
    MissingUnit,
    UnknownUnit(char),
    NotAPositiveInteger,
    Overflow,
}

pub fn parse_duration(text: &str) -> Result<u64, DurationError> {
    if text == "0" {
        return Ok(0);
    }
    let Some(unit) = text.chars().last() else {
        return Err(DurationError::NotAPositiveInteger);
    };
    if unit.is_ascii_digit() {
        return Err(DurationError::MissingUnit);
    }
    let digits = &text[..text.len() - unit.len_utf8()];
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(DurationError::NotAPositiveInteger);
    }
    let count: u64 = digits.parse().map_err(|_| DurationError::Overflow)?;
    if count == 0 {
        return Err(DurationError::NotAPositiveInteger);
    }
    let multiplier = match unit {
        's' => 1,
        'm' => 60,
        'h' => 3_600,
        'd' => 86_400,
        other => return Err(DurationError::UnknownUnit(other)),
    };
    count.checked_mul(multiplier).ok_or(DurationError::Overflow)
}
```

`crates/vpt-adapters/src/config/validate.rs`, above its test module:

```rust
//! The value rules of spec section 10, applied by kind from the key table.

use super::load::ConfigError;
use super::schema::{Kind, spec_for};
use toml::{Table, Value};
use vpt_domain::duration::parse_duration;

pub fn validate(table: &Table) -> Result<(), ConfigError> {
    walk(table, "")
}

fn walk(table: &Table, prefix: &str) -> Result<(), ConfigError> {
    for (name, value) in table {
        let path = if prefix.is_empty() { name.clone() } else { format!("{prefix}.{name}") };
        match value {
            Value::Table(inner) => walk(inner, &path)?,
            leaf => {
                let kind = if path.starts_with("retention.hold.") { Kind::Duration } else { spec_for(&path).unwrap_or(Kind::Text) };
                check(&path, kind, leaf)?;
            }
        }
    }
    Ok(())
}

/// A TOML number as f64: a float as is, an integer widened, so `0` and `1`
/// are valid ratio endpoints.
fn number(value: &Value) -> Option<f64> {
    value.as_float().or_else(|| value.as_integer().map(|n| n as f64))
}

fn check(key: &str, kind: Kind, value: &Value) -> Result<(), ConfigError> {
    let wrong = || ConfigError::WrongType { key: key.into(), expected: kind.expected().into() };
    let range = |rule: &str| ConfigError::OutOfRange { key: key.into(), rule: rule.into() };
    match kind {
        Kind::Bool => value.as_bool().map(|_| ()).ok_or_else(wrong),
        Kind::PositiveInt => match value.as_integer() {
            Some(n) if n >= 1 && n <= i64::from(u32::MAX) => Ok(()),
            Some(_) => Err(range("a positive 32-bit integer")),
            None => Err(wrong()),
        },
        Kind::NonNegativeInt => match value.as_integer() {
            Some(n) if n >= 0 && n <= i64::from(u32::MAX) => Ok(()),
            Some(_) => Err(range("a non-negative 32-bit integer")),
            None => Err(wrong()),
        },
        Kind::PositiveInt64 => match value.as_integer() {
            Some(n) if n >= 1 => Ok(()),
            Some(_) => Err(range("a positive 64-bit integer")),
            None => Err(wrong()),
        },
        Kind::Ratio => match number(value) {
            Some(f) if f.is_finite() && (0.0..=1.0).contains(&f) => Ok(()),
            Some(_) => Err(range("a ratio in [0, 1]")),
            None => Err(wrong()),
        },
        Kind::RatioAboveZero => match number(value) {
            Some(f) if f.is_finite() && f > 0.0 && f <= 1.0 => Ok(()),
            Some(_) => Err(range("a ratio in (0, 1]")),
            None => Err(wrong()),
        },
        Kind::Duration => match value {
            Value::Integer(0) => Ok(()),
            Value::String(text) => parse_duration(text).map(|_| ()).map_err(|_| range("0 or <n>s|m|h|d")),
            _ => Err(range("0 or <n>s|m|h|d")),
        },
        Kind::Enum(options) => match value.as_str() {
            Some(text) if options.contains(&text) => Ok(()),
            Some(_) => Err(range(&format!("one of {}", options.join(", ")))),
            None => Err(wrong()),
        },
        Kind::Text | Kind::Secret => value.as_str().map(|_| ()).ok_or_else(wrong),
        Kind::TextList | Kind::Argv => match value.as_array() {
            Some(items) if items.iter().all(Value::is_str) => Ok(()),
            _ => Err(wrong()),
        },
    }
}
```

In `load.rs`, add `key()` to `ConfigError` and call `validate` before returning:

```rust
impl ConfigError {
    pub fn key(&self) -> Option<&str> {
        match self {
            ConfigError::UnknownKey(key)
            | ConfigError::WrongType { key, .. }
            | ConfigError::OutOfRange { key, .. } => Some(key),
            _ => None,
        }
    }
}
```

and in `load_text`, replace `Ok(merged)` with:

```rust
    super::validate::validate(&merged)?;
    Ok(merged)
```

`validate` stays a private module of `config`: nothing outside the loader calls it.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo fmt --all`

Run: `cargo test -p vpt-domain -p vpt-adapters`

Expected: all PASS, Task 3's fifteen tests included.

- [ ] **Step 5: Commit**

Run: `cargo fmt --all`

```bash
git add crates/vpt-domain crates/vpt-adapters
SKIP_AI_COMMIT=1 git commit -m "feat(config): value rules by kind and the duration syntax"
```

______________________________________________________________________

### Task 5: Settings and root resolution

The validated table becomes typed `Settings` the application reads; roots are expanded, made absolute,
resolved through the home's permitted symlink, and refused when they overlap in a way spec section 4.1
forbids. Resolution writes nothing: every root is computed and checked first, and only a separate, later
call creates the approved leaves. The home may contain its stores. The protected root is the Voice Memos
container, the parent of `recordings_dir`, so the live database beside `Recordings/` is covered by the
overlap rule too; the configuration directory is a root of its own.

**Files:**

- Create: `crates/vpt-domain/src/layout.rs`, `crates/vpt-domain/src/retention.rs`
- Modify: `crates/vpt-domain/src/lib.rs`
- Create: `crates/vpt-application/src/settings.rs`
- Modify: `crates/vpt-application/src/lib.rs`
- Create: `crates/vpt-adapters/src/config/settings.rs`, `crates/vpt-adapters/src/config/roots.rs`,
  `crates/vpt-adapters/src/config/roots/tests.rs`, `crates/vpt-adapters/src/config/paths.rs`
- Modify: `crates/vpt-adapters/src/config/mod.rs`

**Interfaces:**

- Consumes: `config::{get, ConfigError, STORE_KEYS, load_text}`, `parse_duration`.

- Produces:

  - `vpt_domain::layout::StoreKey::{Audio, Transcripts, Analysis, Briefs, EngineOutputs,`
    `Drafts, Released}` with `all() -> [StoreKey; 7]`, `key_name(self) -> &'static str`,
    `default_leaf(self) -> &'static str`, `from_key_name(name: &str) -> Option<StoreKey>`,
    `is_private(self) -> bool` (everything but `Released`);
    `RootName::{Home, Store(StoreKey), State, Config, VoiceMemos}` with `key_name(self) -> String`;
    `RootConflict { pub first: RootName, pub second: RootName }`;
    `check_overlaps(roots: &[(RootName, &Path)]) -> Result<(), RootConflict>`.
  - `vpt_domain::retention::Hold` with `never() -> Hold`, `of_seconds(seconds: u64) -> Hold`,
    `seconds(self) -> u64` (the expiry decision arrives in Task 31).
  - `vpt_application::Settings { pub config_version: u32, pub home: PathBuf, pub state_dir: PathBuf,`
    `pub symlink_target: Option<PathBuf>, pub helper_path: PathBuf, pub stores: StorePaths,`
    `pub source: SourceSettings, pub notify: NotifySettings, pub retention: RetentionSettings }`;
    `StorePaths { pub audio, pub transcripts, pub analysis, pub briefs, pub engine_outputs, pub drafts,`
    `pub released: PathBuf }` with `get(&self, key: StoreKey) -> &Path` and
    `set(&mut self, key: StoreKey, path: PathBuf)`;
    `SourceSettings { pub recordings_dir: PathBuf, pub read_titles: bool, pub quiet_period_secs: u64,`
    `pub deferral_page_threshold: u32, pub max_audio_bytes: u64 }`;
    `NotifyMode::{Desktop, Command(Vec<String>), Off}`;
    `NotifySettings { pub mode: NotifyMode, pub aggregate_after: u32 }`;
    `RetentionSettings { pub enabled: bool, pub include_audio: bool, pub holds: Vec<(StoreKey, Hold)> }`
    with `hold(&self, key: StoreKey) -> Hold`.
  - `vpt_adapters::config::{DEFAULT_HOME: &str, from_table(table: &toml::Table, home_dir: &Path) ->`
    `Result<Settings, ConfigError>}`.
  - `vpt_adapters::config::RootError::{NotAbsolute { key: String }, ParentMissing { key: String },`
    `NotADirectory { key: String }, PathEscape { key: String }, Overlap { first: String, second: String },`
    `Io { key: String, detail: String }}`;
    `Roots { pub home: PathBuf, pub state_dir: PathBuf, pub stores: StorePaths,`
    `pub recordings_dir: PathBuf,` `pub container: PathBuf, pub config_dir: PathBuf }` (every path
    absolute, canonical where it exists);
    `planned_directory(path: &Path, key: &str) -> Result<PathBuf, RootError>` (resolve existing ancestors
    without creation); `resolve_stores(settings: &Settings) -> Result<StorePaths, RootError>` (home and
    stores only); `resolve(settings: &Settings, config_dir: &Path) -> Result<Roots, RootError>` (writes
    nothing); `Roots::create_state_dir(&self) -> Result<(), RootError>` and
    `Roots::create_leaves(&self) -> Result<(), RootError>` (the home and every missing store leaf, mode
    0700, only after a resolution succeeded).
  - `vpt_adapters::config::{config_path(env: impl Fn(&str) -> Option<String>) -> PathBuf,`
    `default_state_dir(env: impl Fn(&str) -> Option<String>) -> String}`.

- [ ] **Step 1: Write the failing tests**

Declare the modules first: `crates/vpt-domain/src/lib.rs` gains `pub mod layout;` and
`pub mod retention;`; `crates/vpt-application/src/lib.rs` becomes

```rust
//! Use cases and the ports they own.

mod settings;

pub use settings::{NotifyMode, NotifySettings, RetentionSettings, Settings, SourceSettings, StorePaths};
```

and `crates/vpt-adapters/src/config/mod.rs` gains `mod paths; mod roots; mod settings;` with

```rust
pub use paths::{config_path, default_state_dir};
pub use roots::{RootError, Roots, planned_directory, resolve, resolve_stores};
pub use settings::{DEFAULT_HOME, from_table};
```

`crates/vpt-adapters/src/lib.rs` also gains `pub use config::{RootError, Roots};`.

Each new file starts as its test module alone. `crates/vpt-domain/src/layout.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn the_home_may_contain_its_stores() {
        let roots = [
            (RootName::Home, Path::new("/h")),
            (RootName::Store(StoreKey::Audio), Path::new("/h/audio")),
            (RootName::Store(StoreKey::Released), Path::new("/h/released")),
            (RootName::State, Path::new("/s")),
            (RootName::Config, Path::new("/c")),
            (RootName::VoiceMemos, Path::new("/vm")),
        ];
        assert_eq!(check_overlaps(&roots), Ok(()));
    }

    #[test]
    fn two_stores_may_not_nest_or_coincide() {
        let roots = [
            (RootName::Store(StoreKey::Audio), Path::new("/h/audio")),
            (RootName::Store(StoreKey::Drafts), Path::new("/h/audio/drafts")),
        ];
        assert_eq!(
            check_overlaps(&roots),
            Err(RootConflict { first: RootName::Store(StoreKey::Audio), second: RootName::Store(StoreKey::Drafts) })
        );
    }

    #[test]
    fn a_writable_root_may_not_overlap_the_voice_memos_container() {
        let roots = [(RootName::State, Path::new("/vm/state")), (RootName::VoiceMemos, Path::new("/vm"))];
        assert!(check_overlaps(&roots).is_err());
        let roots = [(RootName::Store(StoreKey::Audio), Path::new("/x")), (RootName::VoiceMemos, Path::new("/x/Recordings"))];
        assert!(check_overlaps(&roots).is_err());
    }

    #[test]
    fn the_release_destination_may_not_overlap_a_private_root() {
        let roots = [(RootName::Store(StoreKey::Released), Path::new("/c/out")), (RootName::Config, Path::new("/c"))];
        assert!(check_overlaps(&roots).is_err());
        let roots = [(RootName::State, Path::new("/s")), (RootName::Store(StoreKey::Released), Path::new("/s/out"))];
        assert!(check_overlaps(&roots).is_err());
    }
}
```

`crates/vpt-adapters/src/config/roots.rs` starts with `#[cfg(test)] mod tests;`.
`crates/vpt-adapters/src/config/roots/tests.rs`:

```rust
use super::*;
use crate::config::{from_table, load_text};
use std::os::unix::fs::PermissionsExt;
use vpt_application::Settings;

/// A fixture whose Voice Memos container is `<temp>/voice-memos` and whose
/// writable roots all sit outside it.
fn fixture() -> tempfile::TempDir {
    let temp = tempfile::tempdir().expect("temp");
    std::fs::create_dir_all(temp.path().join("h")).expect("home");
    std::fs::create_dir_all(temp.path().join("voice-memos/Recordings")).expect("voice memos");
    std::fs::write(temp.path().join("voice-memos/CloudRecordings.db"), b"live").expect("live db");
    std::fs::create_dir_all(temp.path().join("cfg")).expect("config dir");
    temp
}

fn settings_in(root: &Path, extra: &str) -> Settings {
    let text = format!(
        "config_version = 1\n[home]\npath = \"{}\"\nstate_dir = \"{}\"\n[source]\nrecordings_dir = \"{}\"\n{extra}",
        root.join("h").display(),
        root.join("s").display(),
        root.join("voice-memos/Recordings").display()
    );
    from_table(&load_text(&text).expect("loads"), root).expect("settings")
}

fn entries(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .expect("readable")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

#[test]
fn resolution_writes_nothing_and_create_leaves_makes_the_store_leaves_0700() {
    let temp = fixture();
    let roots = resolve(&settings_in(temp.path(), ""), &temp.path().join("cfg")).expect("resolves");
    assert!(
        !temp.path().join("h/audio").exists(),
        "resolve created a leaf"
    );
    assert!(
        !temp.path().join("s").exists(),
        "resolve created the state directory"
    );
    roots.create_leaves().expect("leaves");
    assert!(roots.stores.audio.is_dir());
    assert_eq!(
        roots.stores.audio,
        temp.path()
            .join("h/audio")
            .canonicalize()
            .expect("canonical")
    );
    assert_eq!(
        std::fs::metadata(&roots.stores.audio)
            .expect("meta")
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    roots.create_state_dir().expect("state");
    assert_eq!(
        std::fs::metadata(&roots.state_dir)
            .expect("meta")
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
}

#[test]
fn a_missing_default_home_is_a_prospective_leaf_with_its_stores_below_it() {
    let temp = fixture();
    std::fs::remove_dir(temp.path().join("h")).expect("no home yet");
    let roots = resolve(&settings_in(temp.path(), ""), &temp.path().join("cfg")).expect("resolves");
    assert_eq!(
        roots.stores.drafts,
        temp.path()
            .canonicalize()
            .expect("canonical")
            .join("h/drafts")
    );
    roots.create_leaves().expect("leaves");
    assert!(temp.path().join("h/drafts").is_dir());
}

#[test]
fn a_store_pointed_elsewhere_needs_an_existing_parent() {
    let temp = fixture();
    let extra = format!(
        "[stores]\naudio = \"{}\"\n",
        temp.path().join("missing/audio").display()
    );
    let error = resolve(&settings_in(temp.path(), &extra), &temp.path().join("cfg")).unwrap_err();
    assert_eq!(
        error,
        RootError::ParentMissing {
            key: "stores.audio".into()
        }
    );
}

#[test]
fn a_store_inside_the_voice_memos_container_is_refused_and_the_container_is_untouched() {
    let temp = fixture();
    let container = temp.path().join("voice-memos");
    let before = entries(&container);
    let live = std::fs::metadata(container.join("CloudRecordings.db")).expect("live");
    let extra = format!(
        "[stores]\ndrafts = \"{}\"\n",
        container.join("drafts").display()
    );

    let error = resolve(&settings_in(temp.path(), &extra), &temp.path().join("cfg")).unwrap_err();

    assert_eq!(
        error,
        RootError::Overlap {
            first: "source.recordings_dir".into(),
            second: "stores.drafts".into()
        }
    );
    assert_eq!(entries(&container), before);
    assert!(!container.join("drafts").exists());
    let after = std::fs::metadata(container.join("CloudRecordings.db")).expect("live");
    assert_eq!(
        (after.len(), after.modified().expect("mtime")),
        (live.len(), live.modified().expect("mtime"))
    );
    assert!(
        !temp.path().join("h/audio").exists(),
        "a refusal created a leaf elsewhere"
    );
}

#[test]
fn the_configuration_directory_is_a_root_the_released_store_may_not_overlap() {
    let temp = fixture();
    let extra = format!(
        "[stores]\nreleased = \"{}\"\n",
        temp.path().join("cfg/out").display()
    );
    let error = resolve(&settings_in(temp.path(), &extra), &temp.path().join("cfg")).unwrap_err();
    assert_eq!(
        error,
        RootError::Overlap {
            first: "config".into(),
            second: "stores.released".into()
        }
    );
}

#[test]
fn a_relative_root_after_expansion_is_refused() {
    let temp = fixture();
    let error = resolve(
        &settings_in(temp.path(), "[stores]\nbriefs = \"briefs\"\n"),
        &temp.path().join("cfg"),
    )
    .unwrap_err();
    assert_eq!(
        error,
        RootError::NotAbsolute {
            key: "stores.briefs".into()
        }
    );
}

#[test]
fn a_file_where_a_root_belongs_is_refused() {
    let temp = fixture();
    std::fs::write(temp.path().join("h/audio"), b"not a directory").expect("file");
    let error = resolve(&settings_in(temp.path(), ""), &temp.path().join("cfg")).unwrap_err();
    assert_eq!(
        error,
        RootError::NotADirectory {
            key: "stores.audio".into()
        }
    );
}

#[test]
fn a_symlinked_home_is_followed_when_no_target_is_configured() {
    let temp = fixture();
    std::fs::remove_dir(temp.path().join("h")).expect("replace the home");
    std::fs::create_dir_all(temp.path().join("real")).expect("real home");
    std::os::unix::fs::symlink(temp.path().join("real"), temp.path().join("h")).expect("link");
    let roots = resolve(&settings_in(temp.path(), ""), &temp.path().join("cfg")).expect("resolves");
    assert_eq!(
        roots.home,
        temp.path().join("real").canonicalize().expect("canonical")
    );
    assert_eq!(
        roots.container,
        temp.path()
            .join("voice-memos")
            .canonicalize()
            .expect("canonical")
    );
}

#[test]
fn a_missing_home_does_not_authorize_parent_traversal_into_the_source() {
    let temp = fixture();
    std::fs::remove_dir(temp.path().join("h")).expect("missing home");
    let before = entries(&temp.path().join("voice-memos"));
    let settings = settings_in(
        temp.path(),
        "[stores]\naudio = \"<home>/../voice-memos/evil\"\n",
    );

    assert_eq!(
        resolve(&settings, &temp.path().join("cfg")).unwrap_err(),
        RootError::ParentMissing {
            key: "stores.audio".into(),
        }
    );
    assert_eq!(entries(&temp.path().join("voice-memos")), before);
    assert!(!temp.path().join("h").exists());
}
#[test]
fn a_dangling_configuration_directory_link_is_refused() {
    let temp = fixture();
    let selected = temp.path().join("config-link");
    std::os::unix::fs::symlink(temp.path().join("absent"), &selected).expect("link");
    assert_eq!(
        resolve(&settings_in(temp.path(), ""), &selected).unwrap_err(),
        RootError::PathEscape {
            key: "config".into()
        }
    );
}

#[test]
fn resolution_can_describe_absent_source_and_deep_default_directories_without_writes() {
    let temp = fixture();
    let mut settings = settings_in(temp.path(), "");
    settings.state_dir = temp.path().join("new/local/state/vpt");
    settings.source.recordings_dir = temp.path().join("absent-source/Recordings");
    let selected = temp.path().join("new/config/vpt");
    let roots = resolve(&settings, &selected).expect("prospective roots");
    let base = temp.path().canonicalize().expect("canonical");
    assert_eq!(roots.config_dir, base.join("new/config/vpt"));
    assert_eq!(roots.state_dir, base.join("new/local/state/vpt"));
    assert_eq!(roots.container, base.join("absent-source"));
    assert!(!temp.path().join("new").exists());
    assert!(!temp.path().join("absent-source").exists());
}
```

`crates/vpt-adapters/src/config/settings.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::load_text;
    use std::path::Path;
    use vpt_application::NotifyMode;
    use vpt_domain::layout::StoreKey;

    #[test]
    fn tilde_and_home_placeholders_expand() {
        let settings = from_table(&load_text("config_version = 1\n").expect("loads"), Path::new("/h/someone")).expect("settings");
        assert_eq!(settings.home, Path::new("/h/someone/.vpt"));
        assert_eq!(settings.stores.get(StoreKey::EngineOutputs), Path::new("/h/someone/.vpt/engine-outputs"));
        assert_eq!(settings.state_dir, Path::new("/h/someone/.local/state/vpt"));
    }

    #[test]
    fn notify_command_mode_carries_its_argv_and_off_carries_nothing() {
        let table = load_text("config_version = 1\n[notify]\nmode = \"command\"\ncommand = [\"notify-command\", \"{event}\"]\n").expect("loads");
        let settings = from_table(&table, Path::new("/u")).expect("settings");
        assert_eq!(settings.notify.mode, NotifyMode::Command(vec!["notify-command".into(), "{event}".into()]));
        let table = load_text("config_version = 1\n[notify]\nmode = \"off\"\n").expect("loads");
        assert_eq!(from_table(&table, Path::new("/u")).expect("settings").notify.mode, NotifyMode::Off);
    }

    #[test]
    fn command_mode_with_an_empty_command_is_refused() {
        let table = load_text("config_version = 1\n[notify]\nmode = \"command\"\n").expect("loads");
        let error = from_table(&table, Path::new("/u")).unwrap_err();
        assert_eq!(error.key(), Some("notify.command"));
    }

    #[test]
    fn a_symlink_target_with_a_non_default_home_is_refused() {
        let table = load_text("config_version = 1\n[home]\npath = \"/elsewhere\"\nsymlink_target = \"/vol/vpt\"\n").expect("loads");
        assert_eq!(from_table(&table, Path::new("/u")).unwrap_err().key(), Some("home.symlink_target"));
    }

    #[test]
    fn retention_holds_are_parsed_per_store() {
        let table = load_text("config_version = 1\n[retention]\nenabled = true\n[retention.hold]\naudio = \"2d\"\n").expect("loads");
        let settings = from_table(&table, Path::new("/u")).expect("settings");
        assert!(settings.retention.enabled);
        assert_eq!(settings.retention.hold(StoreKey::Audio).seconds(), 172_800);
        assert_eq!(settings.retention.hold(StoreKey::Drafts).seconds(), 0);
    }
}
```

`crates/vpt-adapters/src/config/paths.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn env_of(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> + '_ {
        move |name| pairs.iter().find(|(key, _)| *key == name).map(|(_, value)| (*value).to_owned())
    }

    #[test]
    fn vpt_config_wins_over_xdg_which_wins_over_home() {
        assert_eq!(config_path(env_of(&[("VPT_CONFIG", "/c.toml"), ("XDG_CONFIG_HOME", "/x"), ("HOME", "/h")])), PathBuf::from("/c.toml"));
        assert_eq!(config_path(env_of(&[("XDG_CONFIG_HOME", "/x"), ("HOME", "/h")])), PathBuf::from("/x/vpt/config.toml"));
        assert_eq!(config_path(env_of(&[("HOME", "/h")])), PathBuf::from("/h/.config/vpt/config.toml"));
    }

    #[test]
    fn the_state_dir_default_follows_xdg_state_home_when_set() {
        assert_eq!(default_state_dir(env_of(&[("XDG_STATE_HOME", "/s")])), "/s/vpt");
        assert_eq!(default_state_dir(env_of(&[])), "~/.local/state/vpt");
    }
}
```

`crates/vpt-application/src/settings.rs` and `crates/vpt-domain/src/retention.rs` start as their doc
lines; the re-exports in `lib.rs` name items that do not exist yet, which is part of the red build.

- [ ] **Step 2: Run the tests to verify they fail**

Run independently, including the second command after the first expected failure:

Run: `cargo test -p vpt-domain layout`

Run: `cargo test -p vpt-adapters config`

Expected: the builds fail with `cannot find` for `check_overlaps`, `RootName`, `from_table`, `resolve`,
`config_path` and the `Settings` re-export. Every new test module is compiled and selected; a run that
selects zero tests, or that succeeds, does not satisfy this step.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-domain/src/layout.rs`, above its test module:

```rust
//! The store keys and the root overlap rule of spec section 4.1.

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StoreKey {
    Audio,
    Transcripts,
    Analysis,
    Briefs,
    EngineOutputs,
    Drafts,
    Released,
}

impl StoreKey {
    pub fn all() -> [StoreKey; 7] {
        [
            StoreKey::Audio,
            StoreKey::Transcripts,
            StoreKey::Analysis,
            StoreKey::Briefs,
            StoreKey::EngineOutputs,
            StoreKey::Drafts,
            StoreKey::Released,
        ]
    }

    pub fn key_name(self) -> &'static str {
        match self {
            StoreKey::Audio => "audio",
            StoreKey::Transcripts => "transcripts",
            StoreKey::Analysis => "analysis",
            StoreKey::Briefs => "briefs",
            StoreKey::EngineOutputs => "engine_outputs",
            StoreKey::Drafts => "drafts",
            StoreKey::Released => "released",
        }
    }

    pub fn default_leaf(self) -> &'static str {
        match self {
            StoreKey::EngineOutputs => "engine-outputs",
            other => other.key_name(),
        }
    }

    pub fn from_key_name(name: &str) -> Option<StoreKey> {
        StoreKey::all().into_iter().find(|key| key.key_name() == name)
    }

    pub fn is_private(self) -> bool {
        self != StoreKey::Released
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootName {
    Home,
    Store(StoreKey),
    State,
    Config,
    VoiceMemos,
}

impl RootName {
    pub fn key_name(self) -> String {
        match self {
            RootName::Home => "home.path".into(),
            RootName::Store(store) => format!("stores.{}", store.key_name()),
            RootName::State => "home.state_dir".into(),
            RootName::Config => "config".into(),
            RootName::VoiceMemos => "source.recordings_dir".into(),
        }
    }

    fn writable(self) -> bool {
        matches!(self, RootName::Store(_) | RootName::State)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct RootConflict {
    pub first: RootName,
    pub second: RootName,
}

fn overlap(a: &Path, b: &Path) -> bool {
    a.starts_with(b) || b.starts_with(a)
}

fn forbidden(a: RootName, b: RootName) -> bool {
    match (a, b) {
        (RootName::Store(_), RootName::Store(_)) => true,
        (RootName::VoiceMemos, other) | (other, RootName::VoiceMemos) => other.writable(),
        (RootName::Store(StoreKey::Released), other) | (other, RootName::Store(StoreKey::Released)) => {
            matches!(other, RootName::State | RootName::Config)
        }
        _ => false,
    }
}

pub fn check_overlaps(roots: &[(RootName, &Path)]) -> Result<(), RootConflict> {
    for (index, (first, first_path)) in roots.iter().enumerate() {
        for (second, second_path) in &roots[index + 1..] {
            if forbidden(*first, *second) && overlap(first_path, second_path) {
                return Err(RootConflict { first: *first, second: *second });
            }
        }
    }
    Ok(())
}
```

A released store overlapping a private store is covered by the store-versus-store arm.

`crates/vpt-domain/src/retention.rs` (the type only; Task 31 adds the decision):

```rust
//! Retention: a hold per store and the expiry decision.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hold {
    seconds: u64,
}

impl Hold {
    pub fn never() -> Hold {
        Hold { seconds: 0 }
    }

    pub fn of_seconds(seconds: u64) -> Hold {
        Hold { seconds }
    }

    pub fn seconds(self) -> u64 {
        self.seconds
    }
}
```

`crates/vpt-application/src/settings.rs`:

```rust
//! Validated settings. Built by the configuration adapter, read by use cases;
//! nothing here knows TOML.

use std::path::{Path, PathBuf};
use vpt_domain::layout::StoreKey;
use vpt_domain::retention::Hold;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub config_version: u32,
    pub home: PathBuf,
    pub state_dir: PathBuf,
    pub symlink_target: Option<PathBuf>,
    pub helper_path: PathBuf,
    pub stores: StorePaths,
    pub source: SourceSettings,
    pub notify: NotifySettings,
    pub retention: RetentionSettings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorePaths {
    pub audio: PathBuf,
    pub transcripts: PathBuf,
    pub analysis: PathBuf,
    pub briefs: PathBuf,
    pub engine_outputs: PathBuf,
    pub drafts: PathBuf,
    pub released: PathBuf,
}

impl StorePaths {
    pub fn get(&self, key: StoreKey) -> &Path {
        match key {
            StoreKey::Audio => &self.audio,
            StoreKey::Transcripts => &self.transcripts,
            StoreKey::Analysis => &self.analysis,
            StoreKey::Briefs => &self.briefs,
            StoreKey::EngineOutputs => &self.engine_outputs,
            StoreKey::Drafts => &self.drafts,
            StoreKey::Released => &self.released,
        }
    }

    pub fn set(&mut self, key: StoreKey, path: PathBuf) {
        match key {
            StoreKey::Audio => self.audio = path,
            StoreKey::Transcripts => self.transcripts = path,
            StoreKey::Analysis => self.analysis = path,
            StoreKey::Briefs => self.briefs = path,
            StoreKey::EngineOutputs => self.engine_outputs = path,
            StoreKey::Drafts => self.drafts = path,
            StoreKey::Released => self.released = path,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSettings {
    pub recordings_dir: PathBuf,
    pub read_titles: bool,
    pub quiet_period_secs: u64,
    pub deferral_page_threshold: u32,
    pub max_audio_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotifyMode {
    Desktop,
    Command(Vec<String>),
    Off,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotifySettings {
    pub mode: NotifyMode,
    pub aggregate_after: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionSettings {
    pub enabled: bool,
    pub include_audio: bool,
    pub holds: Vec<(StoreKey, Hold)>,
}

impl RetentionSettings {
    pub fn hold(&self, key: StoreKey) -> Hold {
        self.holds.iter().find(|(store, _)| *store == key).map_or(Hold::never(), |(_, hold)| *hold)
    }
}
```

`crates/vpt-adapters/src/config/settings.rs`, above its test module:

```rust
//! From the validated table to typed settings, with `~` and `<home>/` expanded.

use super::load::{ConfigError, get};
use std::path::{Path, PathBuf};
use toml::{Table, Value};
use vpt_application::{NotifyMode, NotifySettings, RetentionSettings, Settings, SourceSettings, StorePaths};
use vpt_domain::duration::parse_duration;
use vpt_domain::layout::StoreKey;
use vpt_domain::retention::Hold;

pub const DEFAULT_HOME: &str = "~/.vpt";

pub fn from_table(table: &Table, home_dir: &Path) -> Result<Settings, ConfigError> {
    let home_text = text(table, "home.path")?;
    let home = expand(&home_text, home_dir, None);
    let symlink_text = text(table, "home.symlink_target")?;
    let symlink_target = if symlink_text.is_empty() {
        None
    } else {
        if home_text != DEFAULT_HOME {
            return Err(ConfigError::OutOfRange {
                key: "home.symlink_target".into(),
                rule: "home.path must stay at its default when a symlink target is set".into(),
            });
        }
        Some(expand(&symlink_text, home_dir, None))
    };
    let mut stores = StorePaths {
        audio: PathBuf::new(),
        transcripts: PathBuf::new(),
        analysis: PathBuf::new(),
        briefs: PathBuf::new(),
        engine_outputs: PathBuf::new(),
        drafts: PathBuf::new(),
        released: PathBuf::new(),
    };
    for key in StoreKey::all() {
        let raw = text(table, &format!("stores.{}", key.key_name()))?;
        stores.set(key, expand(&raw, home_dir, Some(&home)));
    }
    let mode = match text(table, "notify.mode")?.as_str() {
        "desktop" => NotifyMode::Desktop,
        "off" => NotifyMode::Off,
        _ => {
            let argv = strings(table, "notify.command")?;
            if argv.is_empty() {
                return Err(ConfigError::OutOfRange { key: "notify.command".into(), rule: "command mode needs a command".into() });
            }
            NotifyMode::Command(argv)
        }
    };
    let mut holds = Vec::new();
    for key in StoreKey::all() {
        holds.push((key, hold(table, &format!("retention.hold.{}", key.key_name()))?));
    }
    Ok(Settings {
        config_version: small(table, "config_version")?,
        home,
        state_dir: expand(&text(table, "home.state_dir")?, home_dir, None),
        symlink_target,
        helper_path: PathBuf::from(text(table, "helper.path")?),
        stores,
        source: SourceSettings {
            recordings_dir: expand(&text(table, "source.recordings_dir")?, home_dir, None),
            read_titles: boolean(table, "source.read_titles")?,
            quiet_period_secs: u64::from(small(table, "source.quiet_period_secs")?),
            deferral_page_threshold: small(table, "source.deferral_page_threshold")?,
            max_audio_bytes: wide(table, "source.max_audio_bytes")?,
        },
        notify: NotifySettings { mode, aggregate_after: small(table, "notify.aggregate_after")? },
        retention: RetentionSettings {
            enabled: boolean(table, "retention.enabled")?,
            include_audio: boolean(table, "retention.include_audio")?,
            holds,
        },
    })
}

fn expand(raw: &str, home_dir: &Path, home: Option<&Path>) -> PathBuf {
    if let Some(rest) = raw.strip_prefix("~/") {
        return home_dir.join(rest);
    }
    if raw == "~" {
        return home_dir.to_path_buf();
    }
    if let (Some(rest), Some(home)) = (raw.strip_prefix("<home>/"), home) {
        return home.join(rest);
    }
    PathBuf::from(raw)
}

fn leaf<'a>(table: &'a Table, path: &str) -> Result<&'a Value, ConfigError> {
    get(table, path).ok_or_else(|| ConfigError::UnknownKey(path.into()))
}

fn text(table: &Table, path: &str) -> Result<String, ConfigError> {
    leaf(table, path)?.as_str().map(str::to_owned).ok_or_else(|| wrong(path, "string"))
}

fn boolean(table: &Table, path: &str) -> Result<bool, ConfigError> {
    leaf(table, path)?.as_bool().ok_or_else(|| wrong(path, "boolean"))
}

/// A validated 32-bit integer; the validator already refused anything outside `u32`.
fn small(table: &Table, path: &str) -> Result<u32, ConfigError> {
    let value = leaf(table, path)?.as_integer().ok_or_else(|| wrong(path, "integer"))?;
    u32::try_from(value).map_err(|_| wrong(path, "integer"))
}

fn wide(table: &Table, path: &str) -> Result<u64, ConfigError> {
    let value = leaf(table, path)?.as_integer().ok_or_else(|| wrong(path, "integer"))?;
    u64::try_from(value).map_err(|_| wrong(path, "integer"))
}

fn strings(table: &Table, path: &str) -> Result<Vec<String>, ConfigError> {
    let items = leaf(table, path)?.as_array().ok_or_else(|| wrong(path, "list of strings"))?;
    items.iter().map(|item| item.as_str().map(str::to_owned).ok_or_else(|| wrong(path, "list of strings"))).collect()
}

fn hold(table: &Table, path: &str) -> Result<Hold, ConfigError> {
    match leaf(table, path)? {
        Value::Integer(0) => Ok(Hold::never()),
        Value::String(duration) => parse_duration(duration)
            .map(Hold::of_seconds)
            .map_err(|_| ConfigError::OutOfRange { key: path.into(), rule: "0 or <n>s|m|h|d".into() }),
        _ => Err(wrong(path, "duration")),
    }
}

fn wrong(path: &str, expected: &str) -> ConfigError {
    ConfigError::WrongType { key: path.into(), expected: expected.into() }
}
```

`crates/vpt-adapters/src/config/roots.rs`, above its test module:

```rust
//! Resolve every configured root once, at startup, without writing, and
//! refuse the overlaps of spec section 4.1. Creation is a separate, later act.

use std::fs::DirBuilder;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use vpt_application::{Settings, StorePaths};
use vpt_domain::layout::{RootName, StoreKey, check_overlaps};

#[derive(Debug, PartialEq, Eq)]
pub enum RootError {
    NotAbsolute { key: String },
    ParentMissing { key: String },
    NotADirectory { key: String },
    PathEscape { key: String },
    Overlap { first: String, second: String },
    Io { key: String, detail: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Roots {
    pub home: PathBuf,
    pub state_dir: PathBuf,
    pub stores: StorePaths,
    pub recordings_dir: PathBuf,
    pub container: PathBuf,
    pub config_dir: PathBuf,
}

pub fn resolve(settings: &Settings, config_dir: &Path) -> Result<Roots, RootError> {
    let home = planned_directory(&settings.home, "home.path")?;
    let state_dir = planned_directory(&settings.state_dir, "home.state_dir")?;
    let recordings_dir =
        planned_directory(&settings.source.recordings_dir, "source.recordings_dir")?;
    let container = recordings_dir
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| RootError::NotAbsolute {
            key: "source.recordings_dir".into(),
        })?;
    let config_path = if config_dir.is_absolute() {
        config_dir.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| RootError::Io {
                key: "config".into(),
                detail: error.kind().to_string(),
            })?
            .join(config_dir)
    };
    let config_dir = planned_directory(&config_path, "config")?;
    let mut stores = settings.stores.clone();
    for key in StoreKey::all() {
        let below_home = settings
            .stores
            .get(key)
            .strip_prefix(&settings.home)
            .ok()
            .and_then(|rest| {
                let mut components = rest.components();
                match (components.next(), components.next()) {
                    (Some(std::path::Component::Normal(name)), None) => Some(home.join(name)),
                    _ => None,
                }
            });
        let path = prospective(
            settings.stores.get(key),
            &format!("stores.{}", key.key_name()),
            below_home,
        )?;
        stores.set(key, path);
    }
    let mut roots: Vec<(RootName, &Path)> = vec![
        (RootName::Home, &home),
        (RootName::State, &state_dir),
        (RootName::Config, &config_dir),
        (RootName::VoiceMemos, &container),
    ];
    for key in StoreKey::all() {
        roots.push((RootName::Store(key), stores.get(key)));
    }
    check_overlaps(&roots).map_err(|conflict| RootError::Overlap {
        first: conflict.first.key_name(),
        second: conflict.second.key_name(),
    })?;
    Ok(Roots {
        home,
        state_dir,
        stores,
        recordings_dir,
        container,
        config_dir,
    })
}

impl Roots {
    /// The private state directory, created before the write lock is taken.
    pub fn create_state_dir(&self) -> Result<(), RootError> {
        private_dir(&self.state_dir, "home.state_dir")
    }

    /// The home and every missing store leaf, mode 0700, in that order.
    pub fn create_leaves(&self) -> Result<(), RootError> {
        private_dir(&self.home, "home.path")?;
        for key in StoreKey::all() {
            private_dir(self.stores.get(key), &format!("stores.{}", key.key_name()))?;
        }
        Ok(())
    }
}

pub fn planned_directory(path: &Path, key: &str) -> Result<PathBuf, RootError> {
    if !path.is_absolute() {
        return Err(RootError::NotAbsolute { key: key.into() });
    }
    let mut cursor = path;
    let mut missing = Vec::new();
    loop {
        match cursor.canonicalize() {
            Ok(mut parent) => {
                if !parent.is_dir() {
                    return Err(RootError::NotADirectory { key: key.into() });
                }
                for leaf in missing.iter().rev() {
                    parent.push(leaf);
                }
                return Ok(parent);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                match cursor.symlink_metadata() {
                    Ok(_) => return Err(RootError::PathEscape { key: key.into() }),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => {
                        return Err(RootError::Io {
                            key: key.into(),
                            detail: error.kind().to_string(),
                        });
                    }
                }
                let leaf = cursor
                    .file_name()
                    .ok_or_else(|| RootError::ParentMissing { key: key.into() })?;
                missing.push(leaf.to_os_string());
                cursor = cursor
                    .parent()
                    .ok_or_else(|| RootError::ParentMissing { key: key.into() })?;
            }
            Err(error) => {
                return Err(RootError::Io {
                    key: key.into(),
                    detail: error.kind().to_string(),
                });
            }
        }
    }
}

pub fn resolve_stores(settings: &Settings) -> Result<StorePaths, RootError> {
    let home = planned_directory(&settings.home, "home.path")?;
    let mut stores = settings.stores.clone();
    for key in StoreKey::all() {
        let configured = settings.stores.get(key);
        let below_home = configured
            .strip_prefix(&settings.home)
            .ok()
            .and_then(|rest| {
                let mut components = rest.components();
                match (components.next(), components.next()) {
                    (Some(std::path::Component::Normal(name)), None) => Some(home.join(name)),
                    _ => None,
                }
            });
        stores.set(
            key,
            prospective(
                configured,
                &format!("stores.{}", key.key_name()),
                below_home,
            )?,
        );
    }
    let mut roots = vec![(RootName::Home, home.as_path())];
    for key in StoreKey::all() {
        roots.push((RootName::Store(key), stores.get(key)));
    }
    check_overlaps(&roots).map_err(|conflict| RootError::Overlap {
        first: conflict.first.key_name(),
        second: conflict.second.key_name(),
    })?;
    Ok(stores)
}

/// An absolute path, canonical where it exists. A missing leaf under an
/// existing parent resolves to `<canonical parent>/<leaf>`; a missing parent is
/// a refusal, unless `below_home` names where the leaf will sit once the home
/// itself, a prospective leaf, exists.
fn prospective(path: &Path, key: &str, below_home: Option<PathBuf>) -> Result<PathBuf, RootError> {
    if !path.is_absolute() {
        return Err(RootError::NotAbsolute { key: key.into() });
    }
    match path.canonicalize() {
        Ok(canonical) => {
            if !canonical.is_dir() {
                return Err(RootError::NotADirectory { key: key.into() });
            }
            return Ok(canonical);
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(RootError::Io {
                key: key.into(),
                detail: error.kind().to_string(),
            });
        }
    }
    match path.symlink_metadata() {
        Ok(_) => return Err(RootError::PathEscape { key: key.into() }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(RootError::Io {
                key: key.into(),
                detail: error.kind().to_string(),
            });
        }
    }
    let parent = path
        .parent()
        .ok_or_else(|| RootError::NotAbsolute { key: key.into() })?;
    let leaf = path
        .file_name()
        .ok_or_else(|| RootError::NotAbsolute { key: key.into() })?;
    match parent.canonicalize() {
        Ok(parent) => {
            if !parent.is_dir() {
                return Err(RootError::NotADirectory { key: key.into() });
            }
            Ok(parent.join(leaf))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            below_home.ok_or_else(|| RootError::ParentMissing { key: key.into() })
        }
        Err(error) => Err(RootError::Io {
            key: key.into(),
            detail: error.kind().to_string(),
        }),
    }
}

fn private_dir(path: &Path, key: &str) -> Result<(), RootError> {
    match DirBuilder::new().mode(0o700).create(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(RootError::Io {
            key: key.into(),
            detail: error.kind().to_string(),
        }),
    }
}
```

`Roots.container` is what the overlap rule protects, so a store beside `Recordings/` inside Apple's group
container is refused as well as one inside it, and the refusal names `source.recordings_dir` first
because the container precedes every store in the list `check_overlaps` walks. The unresolved
`settings.home` prefix permits exactly one normal store component beneath a missing home. A parent
traversal or deeper missing store parent is refused. `create_leaves` creates the home first.

`crates/vpt-adapters/src/config/paths.rs`, above its test module:

```rust
//! Where the configuration file is, from the environment alone.

use std::path::PathBuf;

pub fn config_path(env: impl Fn(&str) -> Option<String>) -> PathBuf {
    if let Some(explicit) = env("VPT_CONFIG") {
        return PathBuf::from(explicit);
    }
    if let Some(xdg) = env("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg).join("vpt/config.toml");
    }
    PathBuf::from(env("HOME").unwrap_or_default()).join(".config/vpt/config.toml")
}

/// The `home.state_dir` value `vpt setup` writes.
pub fn default_state_dir(env: impl Fn(&str) -> Option<String>) -> String {
    match env("XDG_STATE_HOME") {
        Some(xdg) => format!("{xdg}/vpt"),
        None => "~/.local/state/vpt".into(),
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo fmt --all`

Run: `cargo test --workspace`

Expected: all PASS.

- [ ] **Step 5: Commit**

Run: `cargo fmt --all`

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(config): typed settings and root resolution without writes"
```

______________________________________________________________________

### Task 5a: Checked access below a root

Spec section 4.1: below a resolved root, a path that traverses a symbolic link in any component, or that
escapes the root, is refused when it is opened or published, exit 3 `path_escape`. Stage 1 owns that rule
from its first file operation, and it owns it through directory descriptors: a resolved root is opened
once as a directory handle, and every leaf below it is judged, opened, created, renamed or synced
relative to that handle with no-follow semantics, so a component swapped for a link after resolution
changes nothing. Every root is canonical when Task 5 resolves it, and this module admits exactly one
normal component below it. The ledger, the lock, the title copy, the archive, publication and retention
all go through it; a ledger row or a journal entry is validated with `leaf_below` before anything reads,
writes or hands its path to the helper.

**Files:**

- Create: `crates/vpt-adapters/src/contained.rs`, `crates/vpt-adapters/src/contained/tests.rs`
- Modify: `crates/vpt-adapters/src/lib.rs`, `crates/vpt-adapters/src/config/roots.rs`,
  `crates/vpt-adapters/src/config/roots/tests.rs`, `crates/vpt-adapters/src/config/load.rs`

**Interfaces:**

- Consumes: `config::{RootError, Roots}` and its private
  `private_dir(path: &Path, key: &str) -> Result<(), RootError>` creator from Task 5.

- Produces: descriptor-relative creation behind `Roots::create_state_dir` and `Roots::create_leaves`;
  links introduced after resolution return `RootError::PathEscape { key }`.

- Produces, re-exported from `vpt_adapters`:
  `ContainedError::{Escape { root: PathBuf, path: PathBuf }, NotRegular(PathBuf),`
  `NotADirectory(PathBuf),` `Io { path: PathBuf, kind: std::io::ErrorKind }}`;
  `Access::{Read, Write, ReadWrite}`; `Kind::{File, Directory, Link, Other}`;
  `Stat { pub kind: Kind, pub size: u64, pub mtime_secs: i64, pub mtime_nanos: u32, pub flags: u32 }`
  (the leaf's own attributes, a link judged as a link);
  `leaf_below(root: &Path, path: &Path) -> Result<PathBuf, ContainedError>` (a bare file name or
  `<root>/<name>`, one normal component, never `.`, `..` or a nested path); `RootDir` (`Debug`), the open
  directory descriptor of one canonical root, with
  `RootDir::open(path: &Path) -> Result<RootDir, ContainedError>` (a link in any component or a
  non-directory is `NotADirectory`), `fn path(&self) -> &Path`,
  `fn try_clone(&self) -> Result<RootDir, ContainedError>` (duplicate the retained descriptor),
  `fn identity(&self) -> Result<(u64, u64), ContainedError>` (device and inode of that descriptor),
  `fn revalidate(&self) -> Result<(), ContainedError>` (reopen without following links and require the
  same device and inode before handing an absolute path to another process),
  `fn leaf(&self, path: &Path) -> Result<PathBuf, ContainedError>` (`leaf_below` against this root),
  `fn stat(&self, path: &Path) -> Result<Stat, ContainedError>` (`fstatat` without following),
  `fn regular(&self, path: &Path) -> Result<PathBuf, ContainedError>` (an existing regular file, a link
  refused as `NotRegular`),
  `fn open_file(&self, path: &Path, access: Access) -> Result<File, ContainedError>` (`openat` with
  `O_NOFOLLOW`, `O_CLOEXEC` and `O_NONBLOCK`; a link or a non-file at the leaf is `NotRegular`),
  `fn create_file(&self, path: &Path, mode: u32) -> Result<File, ContainedError>` (exclusive creation at
  that mode, an existing name is `Io` with kind `AlreadyExists`),
  `fn subdirectory(&self, name: &str) -> Result<RootDir, ContainedError>` (a mode-0700 directory, created
  when missing, opened as its own handle; a link or a file there is `NotADirectory`),
  `fn open_subdirectory(&self, name: &str) -> Result<RootDir, ContainedError>` (descriptor-relative,
  read-only traversal; never creates a missing directory),
  `fn rename_over(&self, from: &Path, to: &Path) -> Result<(), ContainedError>` (replaces the target),
  `fn rename_exclusive(&self, from: &Path, to: &Path) -> Result<bool, ContainedError>` (`false` when the
  target exists, nothing moved), `fn sync(&self) -> Result<(), ContainedError>` (the directory itself),
  `fn names(&self) -> Result<Vec<String>, ContainedError>` (every entry name in sorted order; a name that
  is not UTF-8 is skipped, and every name is judged through `stat` or `open_file` before use),
  `fn c_name(&self, path: &Path) -> Result<CString, ContainedError>` (the validated leaf's file name as a
  C string); `RootDir` implements `std::os::fd::AsFd`, so a caller that clones into the root with
  `fclonefileat` names it by descriptor.

- Produces: `ConfigError::PathEscape(String)`;
  `load_file(path: &Path) -> Result<toml::Table, ConfigError>` opens the selected leaf read-only below a
  checked configuration directory.

- [ ] **Step 1: Write the failing tests**

`crates/vpt-adapters/src/lib.rs` gains `mod contained;` and
`pub use contained::{Access, ContainedError, Kind, RootDir, Stat, leaf_below};`.
`crates/vpt-adapters/src/contained.rs` starts with:

```rust
#[cfg(test)]
mod tests;
```

`crates/vpt-adapters/src/contained/tests.rs`:

```rust
use super::*;
use std::os::unix::fs::PermissionsExt;

fn root() -> (tempfile::TempDir, RootDir) {
    let temp = tempfile::tempdir().expect("temp");
    let path = temp.path().canonicalize().expect("canonical");
    std::fs::write(path.join("plain.m4a"), b"bytes").expect("plain");
    std::fs::create_dir(path.join("sub")).expect("sub");
    std::os::unix::fs::symlink(path.join("plain.m4a"), path.join("link.m4a")).expect("link");
    std::os::unix::fs::symlink(path.join("sub"), path.join("dirlink")).expect("dir link");
    let root = RootDir::open(&path).expect("root");
    (temp, root)
}

#[test]
fn a_bare_name_or_the_root_joined_with_it_is_the_one_accepted_shape() {
    let (_temp, root) = root();
    let expected = root.path().join("plain.m4a");
    assert_eq!(root.leaf(Path::new("plain.m4a")), Ok(expected.clone()));
    assert_eq!(root.leaf(&expected), Ok(expected.clone()));
    assert_eq!(leaf_below(root.path(), Path::new("plain.m4a")), Ok(expected));
}

#[test]
fn escapes_and_nested_paths_are_refused_naming_root_and_path() {
    let (_temp, root) = root();
    for path in ["..", "../x", "sub/x", ".", "", "/etc/passwd"] {
        let outcome = root.leaf(Path::new(path));
        assert!(matches!(outcome, Err(ContainedError::Escape { .. })), "{path}: {outcome:?}");
    }
    let elsewhere = root.path().parent().expect("parent").join("plain.m4a");
    assert_eq!(
        root.leaf(&elsewhere),
        Err(ContainedError::Escape { root: root.path().to_path_buf(), path: elsewhere })
    );
}

#[test]
fn a_root_must_be_a_directory_reached_through_no_link() {
    let (_temp, root) = root();
    let through_link = root.path().join("dirlink");
    assert_eq!(RootDir::open(&through_link).err(), Some(ContainedError::NotADirectory(through_link)));
    let file = root.path().join("plain.m4a");
    assert_eq!(RootDir::open(&file).err(), Some(ContainedError::NotADirectory(file)));
    assert!(RootDir::open(&root.path().join("sub")).is_ok());
}

#[test]
fn stat_judges_the_leaf_itself_and_regular_refuses_a_link() {
    let (_temp, root) = root();
    let plain = root.stat(Path::new("plain.m4a")).expect("stat");
    assert_eq!((plain.kind, plain.size), (Kind::File, 5));
    assert!(plain.mtime_secs > 0 && plain.mtime_nanos < 1_000_000_000);
    assert_eq!(root.stat(Path::new("link.m4a")).expect("stat").kind, Kind::Link);
    assert_eq!(root.stat(Path::new("sub")).expect("stat").kind, Kind::Directory);
    assert_eq!(root.regular(Path::new("plain.m4a")), Ok(root.path().join("plain.m4a")));
    assert_eq!(root.regular(Path::new("link.m4a")), Err(ContainedError::NotRegular(root.path().join("link.m4a"))));
    let absent = root.stat(Path::new("absent.m4a"));
    assert!(matches!(absent, Err(ContainedError::Io { kind: std::io::ErrorKind::NotFound, .. })), "{absent:?}");
}

#[test]
fn open_file_refuses_a_link_and_a_directory_at_the_leaf() {
    let (_temp, root) = root();
    assert!(root.open_file(Path::new("plain.m4a"), Access::Read).is_ok());
    let link = root.open_file(Path::new("link.m4a"), Access::Read).err();
    assert_eq!(link, Some(ContainedError::NotRegular(root.path().join("link.m4a"))));
    let directory = root.open_file(Path::new("sub"), Access::Read).err();
    assert_eq!(directory, Some(ContainedError::NotRegular(root.path().join("sub"))));
}

#[test]
fn create_file_is_exclusive_and_sets_the_mode() {
    let (_temp, root) = root();
    let file = root.create_file(Path::new("new.m4a"), 0o600).expect("created");
    assert_eq!(file.metadata().expect("meta").permissions().mode() & 0o777, 0o600);
    let again = root.create_file(Path::new("new.m4a"), 0o600).err();
    assert!(matches!(again, Some(ContainedError::Io { kind: std::io::ErrorKind::AlreadyExists, .. })), "{again:?}");
    assert!(root.create_file(Path::new("link.m4a"), 0o600).is_err());
    assert_eq!(std::fs::read(root.path().join("plain.m4a")).expect("untouched"), b"bytes");
}

#[test]
fn subdirectory_creates_0700_once_and_refuses_a_link_or_a_file() {
    let (_temp, root) = root();
    let made = root.subdirectory("title-copy").expect("made");
    assert_eq!(made.path(), root.path().join("title-copy"));
    assert_eq!(std::fs::metadata(made.path()).expect("meta").permissions().mode() & 0o777, 0o700);
    assert_eq!(root.subdirectory("title-copy").expect("again").path(), made.path());
    assert_eq!(root.subdirectory("dirlink").err(), Some(ContainedError::NotADirectory(root.path().join("dirlink"))));
    assert_eq!(root.subdirectory("plain.m4a").err(), Some(ContainedError::NotADirectory(root.path().join("plain.m4a"))));
}

#[test]
fn rename_exclusive_keeps_an_existing_target_and_rename_over_replaces_it() {
    let (_temp, root) = root();
    std::fs::write(root.path().join("a"), b"A").expect("a");
    std::fs::write(root.path().join("b"), b"B").expect("b");
    assert_eq!(root.rename_exclusive(Path::new("a"), Path::new("b")), Ok(false));
    assert_eq!(std::fs::read(root.path().join("b")).expect("kept"), b"B");
    assert!(root.path().join("a").exists());
    assert_eq!(root.rename_exclusive(Path::new("a"), Path::new("c")), Ok(true));
    assert!(!root.path().join("a").exists());
    assert_eq!(root.rename_over(Path::new("c"), Path::new("b")), Ok(()));
    assert_eq!(std::fs::read(root.path().join("b")).expect("replaced"), b"A");
    assert!(!root.path().join("c").exists());
    assert!(matches!(root.rename_over(Path::new("../x"), Path::new("b")), Err(ContainedError::Escape { .. })));
}

#[test]
fn names_lists_every_entry_sorted_and_sync_succeeds() {
    let (_temp, root) = root();
    assert_eq!(root.names().expect("names"), vec!["dirlink", "link.m4a", "plain.m4a", "sub"]);
    assert_eq!(root.sync(), Ok(()));
    assert_eq!(root.c_name(Path::new("plain.m4a")).expect("name").as_bytes(), b"plain.m4a");
    assert!(matches!(root.c_name(Path::new("sub/x")), Err(ContainedError::Escape { .. })));
}

#[test]
fn revalidation_refuses_a_replaced_root_directory() {
    let temp = tempfile::tempdir().expect("temp");
    let base = temp.path().canonicalize().expect("canonical");
    let audio = base.join("audio");
    std::fs::create_dir(&audio).expect("audio");
    let root = RootDir::open(&audio).expect("root");
    assert_eq!(root.revalidate(), Ok(()));
    std::fs::rename(&audio, base.join("original")).expect("move");
    std::fs::create_dir(&audio).expect("replacement");
    assert!(matches!(root.names(), Err(ContainedError::Escape { .. })));
    assert!(matches!(root.revalidate(), Err(ContainedError::Escape { .. })));
}

#[test]
fn reading_a_subdirectory_creates_nothing_and_refuses_a_link() {
    let (_temp, root) = root();
    assert!(root.open_subdirectory("sub").is_ok());
    assert!(root.open_subdirectory("dirlink").is_err());
    assert!(root.open_subdirectory("missing").is_err());
    assert!(!root.path().join("missing").exists());
}

#[test]
fn opening_a_fifo_returns_not_regular_without_a_writer() {
    let temp = tempfile::tempdir().expect("temp");
    let path = temp.path().canonicalize().expect("canonical");
    let root = RootDir::open(&path).expect("root");
    let fifo = path.join("fifo");
    let name = c_string(&fifo).expect("name");
    // SAFETY: the temporary path is NUL-terminated and outlives the call.
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    assert!(matches!(
        root.open_file(&fifo, Access::Read),
        Err(ContainedError::NotRegular(_))
    ));
}

#[test]
fn a_cloned_root_retains_its_identity_and_refuses_replacement() {
    let temp = tempfile::tempdir().expect("temp");
    let base = temp.path().canonicalize().expect("canonical");
    let path = base.join("root");
    std::fs::create_dir(&path).expect("root");
    let root = RootDir::open(&path).expect("open");
    let clone = root.try_clone().expect("clone");
    assert_eq!(
        root.identity().expect("identity"),
        clone.identity().expect("identity")
    );
    std::fs::rename(&path, base.join("held")).expect("move");
    std::fs::create_dir(&path).expect("replacement");
    assert!(clone.revalidate().is_err());
}
```

Append to `config/roots/tests.rs`:

```rust
#[test]
fn creating_roots_refuses_a_link_added_after_resolution() {
    let temp = fixture();
    let roots = resolve(&settings_in(temp.path(), ""), &temp.path().join("cfg")).expect("roots");
    let before = entries(&roots.container);
    std::os::unix::fs::symlink(&roots.container, &roots.state_dir).expect("state link");
    assert_eq!(roots.create_state_dir(), Err(RootError::PathEscape { key: "home.state_dir".into() }));
    std::fs::rename(&roots.home, temp.path().join("original-home")).expect("move home");
    std::os::unix::fs::symlink(&roots.container, &roots.home).expect("home link");
    assert_eq!(roots.create_leaves(), Err(RootError::PathEscape { key: "home.path".into() }));
    assert_eq!(entries(&roots.container), before);
}
```

Add `PathEscape(String)` to `ConfigError` before this red run. Add this test inside the existing private
test module in `config/load.rs`:

```rust
#[test]
fn a_configuration_leaf_symlink_is_refused() {
    let temp = tempfile::tempdir().expect("temp");
    let target = temp.path().join("target.toml");
    let selected = temp.path().join("config.toml");
    std::fs::write(&target, "config_version = 1\n").expect("target");
    std::os::unix::fs::symlink(&target, &selected).expect("link");

    assert_eq!(
        load_file(&selected),
        Err(ConfigError::PathEscape(selected.display().to_string()))
    );
    assert_eq!(
        std::fs::read_to_string(target).expect("unchanged"),
        "config_version = 1\n"
    );
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-adapters contained`

Expected: the build of the `contained::tests` module fails with `cannot find` for `RootDir`,
`leaf_below`, `ContainedError`, `Access` and `Kind`. The module is compiled and selected; a run that
selects zero tests, or that succeeds, does not satisfy this step. Also run
`cargo test -p vpt-adapters creating_roots_refuses_a_link_added_after_resolution`; once the new module
compiles, it must fail because the old creator accepts the state link. Run
`cargo test -p vpt-adapters a_configuration_leaf_symlink_is_refused` independently and require the
current raw loader to fail this regression.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-adapters/src/contained.rs`, above the existing private test-module declaration:

```rust
//! Checked access below a resolved root through its directory descriptor:
//! exactly one normal component, and a leaf that is never followed through a
//! symbolic link.

use std::ffi::CString;
use std::fs::File;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainedError {
    Escape {
        root: PathBuf,
        path: PathBuf,
    },
    NotRegular(PathBuf),
    NotADirectory(PathBuf),
    Io {
        path: PathBuf,
        kind: std::io::ErrorKind,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Read,
    Write,
    ReadWrite,
}

impl Access {
    fn flags(self) -> libc::c_int {
        match self {
            Access::Read => libc::O_RDONLY,
            Access::Write => libc::O_WRONLY,
            Access::ReadWrite => libc::O_RDWR,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    File,
    Directory,
    Link,
    Other,
}

/// The leaf's own attributes: a link is reported as a link, never followed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stat {
    pub kind: Kind,
    pub size: u64,
    pub mtime_secs: i64,
    pub mtime_nanos: u32,
    pub flags: u32,
}

/// The open directory descriptor of one canonical root.
#[derive(Debug)]
pub struct RootDir {
    fd: OwnedFd,
    path: PathBuf,
}

const NO_FOLLOW: libc::c_int = libc::O_NOFOLLOW | libc::O_CLOEXEC;

fn escape(root: &Path, path: &Path) -> ContainedError {
    ContainedError::Escape {
        root: root.to_path_buf(),
        path: path.to_path_buf(),
    }
}

fn io(path: &Path, error: &std::io::Error) -> ContainedError {
    ContainedError::Io {
        path: path.to_path_buf(),
        kind: error.kind(),
    }
}

fn c_string(path: &Path) -> Result<CString, ContainedError> {
    CString::new(path.as_os_str().as_bytes()).map_err(|_| {
        io(
            path,
            &std::io::Error::from(std::io::ErrorKind::InvalidInput),
        )
    })
}

/// The file name of a validated leaf, as the C string `openat` and friends take.
fn c_name(leaf: &Path) -> Result<CString, ContainedError> {
    let name = leaf.file_name().ok_or_else(|| {
        io(
            leaf,
            &std::io::Error::from(std::io::ErrorKind::InvalidInput),
        )
    })?;
    c_string(Path::new(name))
}

/// `<root>/<name>` for a bare name, or the path itself when it already is exactly that.
pub fn leaf_below(root: &Path, path: &Path) -> Result<PathBuf, ContainedError> {
    let relative = if path.is_absolute() {
        path.strip_prefix(root).map_err(|_| escape(root, path))?
    } else {
        path
    };
    let mut components = relative.components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(name)), None) if !name.is_empty() => Ok(root.join(name)),
        _ => Err(escape(root, path)),
    }
}

impl RootDir {
    /// Open a canonical root; a link in any component or a non-directory is refused.
    pub fn open(path: &Path) -> Result<RootDir, ContainedError> {
        let c_path = c_string(path)?;
        let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW_ANY | libc::O_CLOEXEC;
        // SAFETY: `c_path` is NUL-terminated and outlives the call; no other pointer is passed.
        let fd = unsafe { libc::open(c_path.as_ptr(), flags) };
        if fd < 0 {
            let error = std::io::Error::last_os_error();
            return Err(match error.raw_os_error() {
                Some(libc::ELOOP) | Some(libc::ENOTDIR) => {
                    ContainedError::NotADirectory(path.to_path_buf())
                }
                _ => io(path, &error),
            });
        }
        // SAFETY: `fd` is a fresh descriptor this value now owns.
        Ok(RootDir {
            fd: unsafe { OwnedFd::from_raw_fd(fd) },
            path: path.to_path_buf(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn try_clone(&self) -> Result<Self, ContainedError> {
        Ok(Self {
            fd: self
                .fd
                .try_clone()
                .map_err(|error| io(&self.path, &error))?,
            path: self.path.clone(),
        })
    }

    pub fn identity(&self) -> Result<(u64, u64), ContainedError> {
        use std::os::unix::fs::MetadataExt;
        let file = File::from(
            self.fd
                .try_clone()
                .map_err(|error| io(&self.path, &error))?,
        );
        let metadata = file.metadata().map_err(|error| io(&self.path, &error))?;
        Ok((metadata.dev(), metadata.ino()))
    }

    pub fn revalidate(&self) -> Result<(), ContainedError> {
        use std::os::unix::fs::MetadataExt;
        let current = Self::open(&self.path)?;
        let held = File::from(
            self.fd
                .try_clone()
                .map_err(|error| io(&self.path, &error))?,
        )
        .metadata()
        .map_err(|error| io(&self.path, &error))?;
        let observed = File::from(current.fd)
            .metadata()
            .map_err(|error| io(&self.path, &error))?;
        if (held.dev(), held.ino()) != (observed.dev(), observed.ino()) {
            return Err(escape(&self.path, &self.path));
        }
        Ok(())
    }

    pub fn leaf(&self, path: &Path) -> Result<PathBuf, ContainedError> {
        leaf_below(&self.path, path)
    }

    /// The validated leaf's file name, as the C string `openat` and friends take.
    pub fn c_name(&self, path: &Path) -> Result<CString, ContainedError> {
        c_name(&self.leaf(path)?)
    }

    pub fn open_subdirectory(&self, name: &str) -> Result<RootDir, ContainedError> {
        let path = self.leaf(Path::new(name))?;
        let file = self.open_at(&path, libc::O_RDONLY | libc::O_DIRECTORY | NO_FOLLOW, 0)?;
        Ok(RootDir {
            fd: file.into(),
            path,
        })
    }

    /// `openat` relative to the root; ELOOP at the leaf is a link and is refused.
    fn open_at(&self, leaf: &Path, flags: libc::c_int, mode: u32) -> Result<File, ContainedError> {
        let name = c_name(leaf)?;
        // SAFETY: `name` is NUL-terminated and outlives the call; the directory
        // descriptor stays open for the lifetime of `self`.
        let fd = unsafe {
            libc::openat(
                self.fd.as_raw_fd(),
                name.as_ptr(),
                flags,
                mode as libc::c_uint,
            )
        };
        if fd < 0 {
            let error = std::io::Error::last_os_error();
            return Err(match error.raw_os_error() {
                Some(libc::ELOOP) => ContainedError::NotRegular(leaf.to_path_buf()),
                _ => io(leaf, &error),
            });
        }
        // SAFETY: `fd` is a fresh descriptor the `File` now owns.
        Ok(unsafe { File::from_raw_fd(fd) })
    }

    pub fn stat(&self, path: &Path) -> Result<Stat, ContainedError> {
        let leaf = self.leaf(path)?;
        let name = c_name(&leaf)?;
        // SAFETY: `raw` is a plain-data struct the call fills in; `name` is
        // NUL-terminated and outlives the call.
        let mut raw: libc::stat = unsafe { std::mem::zeroed() };
        let outcome = unsafe {
            libc::fstatat(
                self.fd.as_raw_fd(),
                name.as_ptr(),
                &mut raw,
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        if outcome != 0 {
            return Err(io(&leaf, &std::io::Error::last_os_error()));
        }
        let kind = match raw.st_mode & libc::S_IFMT {
            libc::S_IFREG => Kind::File,
            libc::S_IFDIR => Kind::Directory,
            libc::S_IFLNK => Kind::Link,
            _ => Kind::Other,
        };
        Ok(Stat {
            kind,
            size: u64::try_from(raw.st_size).unwrap_or(0),
            mtime_secs: raw.st_mtime,
            mtime_nanos: u32::try_from(raw.st_mtime_nsec).unwrap_or(0),
            flags: raw.st_flags,
        })
    }

    /// An existing regular file at the leaf, judged without following a link.
    pub fn regular(&self, path: &Path) -> Result<PathBuf, ContainedError> {
        let leaf = self.leaf(path)?;
        match self.stat(&leaf)?.kind {
            Kind::File => Ok(leaf),
            _ => Err(ContainedError::NotRegular(leaf)),
        }
    }

    /// Open the leaf without following a link; a non-file at the leaf is refused.
    pub fn open_file(&self, path: &Path, access: Access) -> Result<File, ContainedError> {
        let leaf = self.leaf(path)?;
        let file = self.open_at(&leaf, access.flags() | NO_FOLLOW | libc::O_NONBLOCK, 0)?;
        let metadata = file.metadata().map_err(|error| io(&leaf, &error))?;
        if !metadata.is_file() {
            return Err(ContainedError::NotRegular(leaf));
        }
        Ok(file)
    }

    /// Create the leaf exclusively at `mode`; nothing is ever replaced.
    pub fn create_file(&self, path: &Path, mode: u32) -> Result<File, ContainedError> {
        let leaf = self.leaf(path)?;
        let file = self.open_at(
            &leaf,
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | NO_FOLLOW,
            mode,
        )?;
        file.set_permissions(std::fs::Permissions::from_mode(mode))
            .map_err(|error| io(&leaf, &error))?;
        Ok(file)
    }

    /// A private directory at `<root>/<name>`, created when missing, as its own handle.
    pub fn subdirectory(&self, name: &str) -> Result<RootDir, ContainedError> {
        let leaf = self.leaf(Path::new(name))?;
        let c_leaf = c_name(&leaf)?;
        // SAFETY: `c_leaf` is NUL-terminated and outlives both calls; the
        // directory descriptor stays open for the lifetime of `self`.
        let made = unsafe { libc::mkdirat(self.fd.as_raw_fd(), c_leaf.as_ptr(), 0o700) };
        if made != 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(io(&leaf, &error));
            }
        }
        let flags = libc::O_RDONLY | libc::O_DIRECTORY | NO_FOLLOW;
        let fd = unsafe { libc::openat(self.fd.as_raw_fd(), c_leaf.as_ptr(), flags) };
        if fd < 0 {
            let error = std::io::Error::last_os_error();
            return Err(match error.raw_os_error() {
                Some(libc::ELOOP) | Some(libc::ENOTDIR) => ContainedError::NotADirectory(leaf),
                _ => io(&leaf, &error),
            });
        }
        // SAFETY: `fd` is a fresh descriptor this value now owns.
        let directory = RootDir {
            fd: unsafe { OwnedFd::from_raw_fd(fd) },
            path: leaf,
        };
        if made == 0 {
            let handle = File::from(
                directory
                    .fd
                    .try_clone()
                    .map_err(|error| io(&directory.path, &error))?,
            );
            handle
                .set_permissions(std::fs::Permissions::from_mode(0o700))
                .map_err(|error| io(&directory.path, &error))?;
        }
        Ok(directory)
    }

    fn rename(&self, from: &Path, to: &Path, flags: libc::c_uint) -> Result<(), std::io::Error> {
        let from =
            c_name(from).map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
        let to = c_name(to).map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
        // SAFETY: both names are NUL-terminated and outlive the call; the
        // directory descriptor stays open for the lifetime of `self`.
        let outcome = unsafe {
            libc::renameatx_np(
                self.fd.as_raw_fd(),
                from.as_ptr(),
                self.fd.as_raw_fd(),
                to.as_ptr(),
                flags,
            )
        };
        if outcome == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }

    /// Move `from` onto `to`, replacing whatever `to` held.
    pub fn rename_over(&self, from: &Path, to: &Path) -> Result<(), ContainedError> {
        let (from, to) = (self.leaf(from)?, self.leaf(to)?);
        self.rename(&from, &to, 0).map_err(|error| io(&to, &error))
    }

    /// Move `from` to `to` only when `to` is absent; `false` leaves both untouched.
    pub fn rename_exclusive(&self, from: &Path, to: &Path) -> Result<bool, ContainedError> {
        let (from, to) = (self.leaf(from)?, self.leaf(to)?);
        match self.rename(&from, &to, libc::RENAME_EXCL) {
            Ok(()) => Ok(true),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
            Err(error) => Err(io(&to, &error)),
        }
    }

    /// Flush the directory itself, so a rename or a creation is durable.
    pub fn sync(&self) -> Result<(), ContainedError> {
        let handle = File::from(
            self.fd
                .try_clone()
                .map_err(|error| io(&self.path, &error))?,
        );
        handle.sync_all().map_err(|error| io(&self.path, &error))
    }

    /// Every entry name in sorted order; each is judged through `stat` or `open_file` before use.
    pub fn names(&self) -> Result<Vec<String>, ContainedError> {
        self.revalidate()?;
        let entries = std::fs::read_dir(&self.path).map_err(|error| io(&self.path, &error))?;
        let mut names = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| io(&self.path, &error))?;
            if let Ok(name) = entry.file_name().into_string() {
                names.push(name);
            }
        }
        names.sort();
        Ok(names)
    }
}

impl AsFd for RootDir {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}
```

`O_NOFOLLOW_ANY` refuses a link in any component when the root itself is opened, and `O_NOFOLLOW` on
every `openat` refuses one at the leaf, which is the only component below a root this module admits;
together they cover every symbolic link a path below a root can traverse, whether it was there at
resolution or arrived later. `renameatx_np` with `RENAME_EXCL` is the platform's exclusive rename, the
primitive Task 18's probe verifies. A canonical root has no link in it, so the tests canonicalize their
temporary directories before opening them. Every later task maps a `ContainedError` to exit 3, rule
`path_escape`, at the composition root.

Replace `private_dir` in `config/roots.rs` with the body below. Remove its `DirBuilder` and
`DirBuilderExt` imports and add `use crate::{ContainedError, RootDir};`.

```rust
fn private_dir(path: &Path, key: &str) -> Result<(), RootError> {
    let parent = path.parent().ok_or_else(|| RootError::NotAbsolute { key: key.into() })?;
    let name = path.file_name().and_then(|name| name.to_str())
        .ok_or_else(|| RootError::NotAbsolute { key: key.into() })?;
    let map = |error| match error {
        ContainedError::Io { kind, .. } => RootError::Io { key: key.into(), detail: kind.to_string() },
        _ => RootError::PathEscape { key: key.into() },
    };
    let directory = RootDir::open(parent).map_err(map)?;
    directory.subdirectory(name).map(|_| ()).map_err(map)
}
```

In `config/load.rs`, add these imports and replace `load_file` with the checked implementation and its
error helpers below. Keep `load_text` and its tests unchanged.

```rust
use crate::contained::{Access, ContainedError, RootDir};
use std::io::Read;
```

```rust
pub fn load_file(path: &Path) -> Result<Table, ConfigError> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = parent
        .canonicalize()
        .map_err(|error| config_io(path, error))?;
    let root = RootDir::open(&parent)
        .map_err(|error| config_contained(path, error))?;
    let name = path.file_name().ok_or_else(|| ConfigError::Unreadable {
        path: path.display().to_string(),
        detail: std::io::ErrorKind::InvalidInput.to_string(),
    })?;
    let mut file = root
        .open_file(Path::new(name), Access::Read)
        .map_err(|error| config_contained(path, error))?;
    let mut text = String::new();
    file.read_to_string(&mut text)
        .map_err(|error| config_io(path, error))?;
    load_text(&text)
}

fn config_io(path: &Path, error: std::io::Error) -> ConfigError {
    if error.kind() == std::io::ErrorKind::NotFound {
        ConfigError::Missing(path.display().to_string())
    } else {
        ConfigError::Unreadable {
            path: path.display().to_string(),
            detail: error.kind().to_string(),
        }
    }
}

fn config_contained(path: &Path, error: ContainedError) -> ConfigError {
    match error {
        ContainedError::Io { kind, .. } => {
            config_io(path, std::io::Error::from(kind))
        }
        _ => ConfigError::PathEscape(path.display().to_string()),
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo fmt --all`

Run: `cargo test -p vpt-adapters contained`

Expected: all 13 contained-access tests PASS. Also run `cargo test -p vpt-adapters config::roots` and
require the root-creation regression to pass. Run
`cargo test -p vpt-adapters a_configuration_leaf_symlink_is_refused` and require it to pass. Run
`cargo clippy -p vpt-adapters --all-targets -- -D warnings` and expect no warnings.

- [ ] **Step 5: Commit**

Run: `cargo fmt --all`

```bash
git add crates/vpt-adapters
SKIP_AI_COMMIT=1 git commit -m "feat(adapters): checked access below a resolved root through its descriptor"
```

______________________________________________________________________

### Task 6: `vpt setup`

**Files:**

- Create: `crates/vpt-application/src/ports/mod.rs`, `crates/vpt-application/src/ports/prompt.rs`,
  `crates/vpt-application/src/setup.rs`
- Modify: `crates/vpt-application/src/lib.rs`
- Create: `crates/vpt-adapters/src/prompt.rs`, `crates/vpt-adapters/src/config/write.rs`
- Modify: `crates/vpt-adapters/src/lib.rs`, `crates/vpt-adapters/src/config/mod.rs`
- Create: `crates/vpt/src/commands/setup.rs`, `crates/vpt/src/compose.rs`
- Modify: `crates/vpt/src/lib.rs`, `crates/vpt/src/commands/mod.rs`, `crates/vpt/Cargo.toml`
- Test: `crates/vpt/tests/setup.rs`; `crates/vpt/tests/support/mod.rs` (`vpt()` detaches from the
  terminal)

**Interfaces:**

- Consumes: `config::{template, config_path, default_state_dir, from_table, load_text, resolve}`,
  `RootDir::{try_clone, revalidate, open_file, create_file, subdirectory, sync}`.

- Produces:

  - `vpt_application::ports::{Prompt, PromptError}` (the files below `ports/` are private modules):
    `PromptError::{NoTerminal, Closed}`; `trait Prompt { fn choose(&mut self, question: &str,`
    `options: &[&str], preselected: usize) -> Result<usize, PromptError>; }`.
  - `vpt_application::{Setup, SetupWriter, SetupError, SetupOutcome}`:
    `SetupError::{NoTerminal, Exists(PathBuf), PathEscape(PathBuf), Io(String)}`;
    `trait SetupWriter { fn config_path(&self) -> PathBuf; fn config_exists(&self) -> bool;`
    `fn write_config(&self, text: &str, force: bool) -> Result<PathBuf, SetupError>;`
    `fn create_private_dir(&self, path: &Path) -> Result<(), SetupError>; }`;
    `SetupOutcome { pub written: PathBuf, pub main_engine: String }` (`Debug`);
    `Setup<R: Fn(&str) -> String> { pub engines: Vec<String>, pub render: R,`
    `pub directories: Vec<PathBuf> }` with
    `run<P: Prompt, W: SetupWriter>(&self, prompt: &mut P, writer: &W, force: bool) ->`
    `Result<SetupOutcome, SetupError>`.
  - `vpt_adapters::TtyPrompt::open() -> Result<TtyPrompt, PromptError>` (implements `Prompt`; end of file
    on the terminal is `Closed`, never a default choice).
  - `vpt_adapters::config::FilesystemSetupWriter::new(config_path: PathBuf, directories: &[PathBuf]) ->`
    `Result<FilesystemSetupWriter, SetupError>` (retains checked existing ancestors without writing;
    implements `SetupWriter`, file mode 0600 and prepared directory mode 0700, existing modes repaired).
  - `compose::Environment::from_process() -> Environment` with
    `var(&self, name: &str) -> Option<String>`, `config_path(&self) -> PathBuf`,
    `home_dir(&self) -> PathBuf`, `state_dir_default(&self) -> String`.
  - `commands::setup::run(environment: &Environment, config: Option<&Path>, force: bool) -> Outcome`.
  - Test support: `Sandbox::vpt()` now starts the binary in a new session, so no test can reach the
    operator's terminal; the interactive test attaches to a pseudoterminal it creates itself.

- [ ] **Step 1: Write the failing tests**

Declare the modules first. `crates/vpt-application/src/lib.rs`:

```rust
//! Use cases and the ports they own.

pub mod ports;
mod settings;
mod setup;

pub use settings::{NotifyMode, NotifySettings, RetentionSettings, Settings, SourceSettings, StorePaths};
pub use setup::{Setup, SetupError, SetupOutcome, SetupWriter};
```

`crates/vpt-application/src/ports/mod.rs`:

```rust
//! The external capabilities the use cases consume, one trait each. The files
//! below are private; every port is re-exported here.

mod prompt;

pub use prompt::{Prompt, PromptError};
```

`crates/vpt-adapters/src/lib.rs` gains `mod prompt;` and `pub use prompt::{TtyPrompt};`;
`crates/vpt-adapters/src/config/mod.rs` gains `mod write;` and `pub use write::FilesystemSetupWriter;`;
`crates/vpt/src/lib.rs` gains `mod compose;`; `crates/vpt/src/commands/mod.rs` gains
`pub(crate) mod setup;`. `ports/prompt.rs`, `prompt.rs`, `compose.rs` and `commands/setup.rs` start as
their doc lines. `crates/vpt/Cargo.toml` gains `libc = "0.2.189"` under `[dev-dependencies]`.

`crates/vpt-application/src/setup.rs` starts as its test module alone:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct FakePrompt(Option<usize>);
    impl Prompt for FakePrompt {
        fn choose(&mut self, _question: &str, _options: &[&str], _preselected: usize) -> Result<usize, PromptError> {
            self.0.ok_or(PromptError::NoTerminal)
        }
    }

    struct FakeWriter {
        exists: bool,
        written: RefCell<Vec<(String, bool)>>,
        dirs: RefCell<Vec<PathBuf>>,
    }
    impl SetupWriter for FakeWriter {
        fn config_path(&self) -> PathBuf {
            PathBuf::from("/c/config.toml")
        }
        fn config_exists(&self) -> bool {
            self.exists
        }
        fn write_config(&self, text: &str, force: bool) -> Result<PathBuf, SetupError> {
            self.written.borrow_mut().push((text.to_owned(), force));
            Ok(PathBuf::from("/c/config.toml"))
        }
        fn create_private_dir(&self, path: &Path) -> Result<(), SetupError> {
            self.dirs.borrow_mut().push(path.to_owned());
            Ok(())
        }
    }

    fn setup() -> Setup<impl Fn(&str) -> String> {
        Setup {
            engines: vec!["apple".into(), "whisply".into()],
            render: |engine: &str| format!("main = \"{engine}\"\n"),
            directories: vec![PathBuf::from("/h"), PathBuf::from("/s"), PathBuf::from("/h/audio")],
        }
    }

    fn writer(exists: bool) -> FakeWriter {
        FakeWriter { exists, written: RefCell::new(vec![]), dirs: RefCell::new(vec![]) }
    }

    #[test]
    fn the_chosen_engine_is_rendered_and_every_directory_is_created() {
        let writer = writer(false);
        let outcome = setup().run(&mut FakePrompt(Some(1)), &writer, false).expect("setup");
        assert_eq!(outcome.main_engine, "whisply");
        assert_eq!(writer.written.borrow().as_slice(), [("main = \"whisply\"\n".to_owned(), false)]);
        assert_eq!(writer.dirs.borrow().len(), 3);
    }

    #[test]
    fn apple_is_preselected_so_an_empty_answer_picks_it() {
        let writer = writer(false);
        let outcome = setup().run(&mut FakePrompt(Some(0)), &writer, false).expect("setup");
        assert_eq!(outcome.main_engine, "apple");
    }

    #[test]
    fn an_existing_config_is_refused_without_force_and_nothing_is_written() {
        let writer = writer(true);
        let error = setup().run(&mut FakePrompt(Some(0)), &writer, false).unwrap_err();
        assert_eq!(error, SetupError::Exists(PathBuf::from("/c/config.toml")));
        assert!(writer.written.borrow().is_empty());
        assert!(writer.dirs.borrow().is_empty());
    }

    #[test]
    fn force_overwrites_an_existing_config_and_says_so_to_the_writer() {
        let writer = writer(true);
        assert!(setup().run(&mut FakePrompt(Some(0)), &writer, true).is_ok());
        assert_eq!(writer.written.borrow().as_slice(), [("main = \"apple\"\n".to_owned(), true)]);
    }

    #[test]
    fn no_terminal_writes_nothing() {
        let writer = writer(false);
        assert_eq!(setup().run(&mut FakePrompt(None), &writer, false).unwrap_err(), SetupError::NoTerminal);
        assert!(writer.written.borrow().is_empty());
    }
}
```

`crates/vpt-adapters/src/config/write.rs` starts as its test module alone:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn mode(path: &Path) -> u32 {
        std::fs::metadata(path).expect("meta").permissions().mode() & 0o777
    }

    #[test]
    fn a_replaced_config_parent_is_refused_without_writing_either_directory() {
        let temp = tempfile::tempdir().expect("temp");
        let base = temp.path().canonicalize().expect("canonical");
        let parent = base.join("config");
        let outside = base.join("outside");
        std::fs::create_dir(&parent).expect("parent");
        std::fs::create_dir(&outside).expect("outside");
        let writer = FilesystemSetupWriter::new(parent.join("config.toml"), &[]).expect("prepared");
        std::fs::rename(&parent, base.join("held")).expect("move");
        std::os::unix::fs::symlink(&outside, &parent).expect("replacement");
        assert!(matches!(
            writer.write_config("replace", true),
            Err(SetupError::PathEscape(_))
        ));
        assert!(!outside.join("config.toml").exists());
        assert!(!base.join("held/config.toml").exists());
    }

    #[test]
    fn prepared_missing_ancestors_are_created_only_when_writing() {
        let temp = tempfile::tempdir().expect("temp");
        let base = temp.path().canonicalize().expect("canonical");
        let directories = vec![base.join("home/.vpt"), base.join("state/local/vpt")];
        let writer = FilesystemSetupWriter::new(base.join("config/vpt/config.toml"), &directories)
            .expect("prepared");
        assert!(!base.join("config").exists());
        assert!(!base.join("home").exists());
        assert!(!base.join("state").exists());
        writer
            .write_config("config_version = 1", false)
            .expect("config");
        for directory in &directories {
            writer.create_private_dir(directory).expect("directory");
            assert_eq!(mode(directory), 0o700);
        }
    }

    #[test]
    fn a_new_config_is_0600_inside_a_0700_directory() {
        let temp = tempfile::tempdir().expect("temp");
        let base = temp.path().canonicalize().expect("canonical");
        let writer =
            FilesystemSetupWriter::new(base.join("vpt/config.toml"), &[]).expect("prepared");
        let written = writer
            .write_config("config_version = 1\n", false)
            .expect("written");
        assert_eq!(mode(&written), 0o600);
        assert_eq!(mode(&base.join("vpt")), 0o700);
    }

    #[test]
    fn without_force_an_existing_file_is_exists_even_when_created_after_the_check() {
        let temp = tempfile::tempdir().expect("temp");
        let base = temp.path().canonicalize().expect("canonical");
        let path = base.join("config.toml");
        std::fs::write(&path, b"old").expect("existing");
        let writer = FilesystemSetupWriter::new(path.clone(), &[]).expect("prepared");
        assert_eq!(
            writer.write_config("new", false).unwrap_err(),
            SetupError::Exists(path.clone())
        );
        assert_eq!(std::fs::read(&path).expect("kept"), b"old");
    }

    #[test]
    fn force_replaces_the_text_and_repairs_a_permissive_mode() {
        let temp = tempfile::tempdir().expect("temp");
        let base = temp.path().canonicalize().expect("canonical");
        let path = base.join("config.toml");
        std::fs::write(&path, b"old").expect("existing");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
            .expect("permissive");
        let writer = FilesystemSetupWriter::new(path.clone(), &[]).expect("prepared");
        writer.write_config("new", true).expect("replaced");
        assert_eq!(std::fs::read(&path).expect("read"), b"new");
        assert_eq!(mode(&path), 0o600);
    }

    #[test]
    fn an_existing_permissive_directory_is_repaired_to_0700() {
        let temp = tempfile::tempdir().expect("temp");
        let base = temp.path().canonicalize().expect("canonical");
        let dir = base.join("home");
        std::fs::create_dir(&dir).expect("dir");
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).expect("permissive");
        FilesystemSetupWriter::new(base.join("c.toml"), std::slice::from_ref(&dir))
            .expect("prepared")
            .create_private_dir(&dir)
            .expect("repaired");
        assert_eq!(mode(&dir), 0o700);
    }

    #[test]
    fn force_refuses_a_configuration_symlink_without_changing_its_target() {
        let temp = tempfile::tempdir().expect("temp");
        let base = temp.path().canonicalize().expect("canonical");
        let target = base.join("target");
        let selected = base.join("config.toml");
        std::fs::write(&target, b"keep").expect("target");
        std::os::unix::fs::symlink(&target, &selected).expect("link");
        let writer = FilesystemSetupWriter::new(selected.clone(), &[]).expect("prepared");

        assert_eq!(
            writer.write_config("replace", true),
            Err(SetupError::PathEscape(selected))
        );
        assert_eq!(std::fs::read(target).expect("target"), b"keep");
    }
}
```

`crates/vpt/tests/support/mod.rs`: `vpt()` gains a session detach so the child has no controlling
terminal, whatever the test runner's stdin is:

```rust
    pub fn vpt(&self) -> Command {
        use std::os::unix::process::CommandExt;
        let mut command = Command::new(VPT);
        command
            .env_clear()
            .env("HOME", self.root.join("home"))
            .env("XDG_CONFIG_HOME", self.root.join("config"))
            .env("XDG_DATA_HOME", self.root.join("data"))
            .env("XDG_STATE_HOME", self.root.join("state"))
            .env("VPT_CONFIG", self.config_path())
            .env("PATH", self.root.join("bin"));
        // SAFETY: setsid is async-signal-safe and the forked child has no other threads.
        unsafe {
            command.pre_exec(|| if libc::setsid() == -1 { Err(std::io::Error::last_os_error()) } else { Ok(()) });
        }
        command
    }
```

`crates/vpt/tests/setup.rs`:

```rust
mod support;

use std::os::unix::fs::PermissionsExt;
use support::{Sandbox, run, stderr, stdout};

#[test]
fn setup_without_a_terminal_exits_2_and_writes_nothing() {
    let sandbox = Sandbox::new("setup-no-tty");

    let output = run(sandbox.vpt().args(["setup", "--json"]));

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout(&output), "");
    let document: serde_json::Value = serde_json::from_str(&stderr(&output)).expect("error document");
    assert_eq!(document["error"]["kind"], "usage");
    assert_eq!(document["error"]["message"], "vpt setup needs a controlling terminal");
    assert!(!sandbox.config_path().exists());
    assert!(!sandbox.path().join("home/.vpt").exists());
}

#[test]
fn setup_under_a_pty_writes_the_config_home_state_and_store_leaves() {
    let sandbox = Sandbox::new("setup-pty");

    let mut command = std::process::Command::new("/usr/bin/script");
    command.args(["-q", "/dev/null", support::VPT, "setup"]);
    let vpt = sandbox.vpt();
    let envs: Vec<(String, String)> = vpt
        .get_envs()
        .filter_map(|(key, value)| Some((key.to_string_lossy().into_owned(), value?.to_string_lossy().into_owned())))
        .collect();
    command.env_clear().envs(envs).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped());
    let mut child = command.spawn().expect("script spawned");
    use std::io::Write;
    child.stdin.take().expect("stdin").write_all(b"\n").expect("answer");
    let output = child.wait_with_output().expect("script finished");

    assert_eq!(output.status.code(), Some(0), "{}", stdout(&output));
    let config = std::fs::read_to_string(sandbox.config_path()).expect("config written");
    assert!(config.starts_with("config_version = 1"), "{config}");
    assert!(config.contains("main = \"apple\""), "{config}");
    let mode = std::fs::metadata(sandbox.config_path()).expect("meta").permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
    let home = sandbox.path().join("home/.vpt");
    assert_eq!(std::fs::metadata(&home).expect("home").permissions().mode() & 0o777, 0o700);
    for leaf in ["audio", "transcripts", "analysis", "briefs", "engine-outputs", "drafts", "released"] {
        assert!(home.join(leaf).is_dir(), "{leaf} missing");
    }
    assert!(sandbox.path().join("state/vpt").is_dir());
}

#[test]
fn setup_refuses_an_existing_config_without_force_and_before_any_terminal() {
    let sandbox = Sandbox::new("setup-exists");
    std::fs::create_dir_all(sandbox.config_path().parent().expect("dir")).expect("config dir");
    std::fs::write(sandbox.config_path(), "config_version = 1\n").expect("existing");

    let output = run(sandbox.vpt().args(["setup"]));

    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("exists; pass --force"), "{}", stderr(&output));
}

#[test]
fn setup_honours_an_explicit_config_path() {
    let sandbox = Sandbox::new("setup-config-flag");
    let elsewhere = sandbox.path().join("elsewhere/config.toml");
    std::fs::create_dir_all(elsewhere.parent().expect("dir")).expect("dir");
    std::fs::write(&elsewhere, "config_version = 1\n").expect("existing");

    let output = run(sandbox.vpt().args(["setup", "--config", elsewhere.to_str().expect("utf8")]));

    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("elsewhere/config.toml exists"), "{}", stderr(&output));
}
```

The `script` process is started outside `Sandbox::vpt()` on purpose: it allocates the pseudoterminal the
interactive test attaches to, so it is the one child that keeps a terminal.

- [ ] **Step 2: Run the tests to verify they fail**

Run independently, including the second command after the first expected failure:

Run: `cargo test -p vpt-application setup`

Run: `cargo test -p vpt-adapters write`

Expected: the builds fail with `cannot find` for `Setup`, `SetupWriter`, `Prompt`,
`FilesystemSetupWriter`. Both test modules are compiled and selected; a run that selects zero tests, or
that succeeds, does not satisfy this step.

Run: `cargo test -p vpt --test setup`

Expected: `setup_without_a_terminal_exits_2_and_writes_nothing` FAILS on the message (the verb is
unimplemented and says so); the pty test FAILS on the missing config; the two refusal tests FAIL on their
stderr text.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-application/src/ports/prompt.rs`:

```rust
//! An interactive choice on the controlling terminal.

#[derive(Debug, PartialEq, Eq)]
pub enum PromptError {
    NoTerminal,
    Closed,
}

pub trait Prompt {
    fn choose(&mut self, question: &str, options: &[&str], preselected: usize) -> Result<usize, PromptError>;
}
```

`crates/vpt-application/src/setup.rs`, above its test module:

```rust
//! `vpt setup`: one prompt, one rendered file, the directories the spec names.

use crate::ports::{Prompt, PromptError};
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq)]
pub enum SetupError {
    NoTerminal,
    Exists(PathBuf),
    PathEscape(PathBuf),
    Io(String),
}

pub trait SetupWriter {
    fn config_path(&self) -> PathBuf;
    fn config_exists(&self) -> bool;
    /// Exclusive creation unless `force`; an existing file is `Exists`.
    fn write_config(&self, text: &str, force: bool) -> Result<PathBuf, SetupError>;
    fn create_private_dir(&self, path: &Path) -> Result<(), SetupError>;
}

#[derive(Debug)]
pub struct SetupOutcome {
    pub written: PathBuf,
    pub main_engine: String,
}

pub struct Setup<R: Fn(&str) -> String> {
    pub engines: Vec<String>,
    pub render: R,
    pub directories: Vec<PathBuf>,
}

impl<R: Fn(&str) -> String> Setup<R> {
    pub fn run<P: Prompt, W: SetupWriter>(&self, prompt: &mut P, writer: &W, force: bool) -> Result<SetupOutcome, SetupError> {
        if writer.config_exists() && !force {
            return Err(SetupError::Exists(writer.config_path()));
        }
        let options: Vec<&str> = self.engines.iter().map(String::as_str).collect();
        let chosen = prompt.choose("Main transcription engine", &options, 0).map_err(|error| match error {
            PromptError::NoTerminal => SetupError::NoTerminal,
            PromptError::Closed => SetupError::Io("the terminal closed before an answer".into()),
        })?;
        let main_engine = self.engines.get(chosen).cloned().unwrap_or_else(|| self.engines[0].clone());
        let written = writer.write_config(&(self.render)(&main_engine), force)?;
        for directory in &self.directories {
            writer.create_private_dir(directory)?;
        }
        Ok(SetupOutcome { written, main_engine })
    }
}
```

`crates/vpt-adapters/src/prompt.rs`:

```rust
//! The controlling terminal, opened by name so `--json` keeps stdout clean.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use vpt_application::ports::{Prompt, PromptError};

pub struct TtyPrompt {
    tty: File,
}

impl TtyPrompt {
    pub fn open() -> Result<TtyPrompt, PromptError> {
        OpenOptions::new().read(true).write(true).open("/dev/tty").map(|tty| TtyPrompt { tty }).map_err(|_| PromptError::NoTerminal)
    }
}

impl Prompt for TtyPrompt {
    fn choose(&mut self, question: &str, options: &[&str], preselected: usize) -> Result<usize, PromptError> {
        let mut text = format!("{question}:\n");
        for (index, option) in options.iter().enumerate() {
            let marker = if index == preselected { "*" } else { " " };
            text.push_str(&format!("  {marker} {} {option}\n", index + 1));
        }
        text.push_str(&format!("choice [{}]: ", preselected + 1));
        self.tty.write_all(text.as_bytes()).map_err(|_| PromptError::Closed)?;
        let mut answer = String::new();
        let mut reader = BufReader::new(self.tty.try_clone().map_err(|_| PromptError::Closed)?);
        if reader.read_line(&mut answer).map_err(|_| PromptError::Closed)? == 0 {
            return Err(PromptError::Closed);
        }
        let answer = answer.trim();
        if answer.is_empty() {
            return Ok(preselected);
        }
        match answer.parse::<usize>() {
            Ok(number) if (1..=options.len()).contains(&number) => Ok(number - 1),
            _ => Ok(preselected),
        }
    }
}
```

A zero-length read is the terminal closing, so setup stops rather than writing the default engine after
the operator has gone.

`crates/vpt-adapters/src/config/write.rs`, above its test module:

```rust
use crate::contained::{Access, ContainedError, RootDir};
use std::ffi::OsString;
use std::fs::{File, Permissions};
use std::io::Write;
use std::os::fd::AsFd;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use vpt_application::{SetupError, SetupWriter};

struct PlannedDirectory {
    path: PathBuf,
    anchor: RootDir,
    missing: Vec<OsString>,
}

impl PlannedDirectory {
    fn capture(path: &Path) -> Result<Self, SetupError> {
        if !path.is_absolute() {
            return Err(SetupError::PathEscape(path.to_path_buf()));
        }
        let mut cursor = path.to_path_buf();
        let mut missing = Vec::new();
        loop {
            match RootDir::open(&cursor) {
                Ok(anchor) => {
                    missing.reverse();
                    return Ok(Self {
                        path: path.to_path_buf(),
                        anchor,
                        missing,
                    });
                }
                Err(ContainedError::Io {
                    kind: std::io::ErrorKind::NotFound,
                    ..
                }) => {
                    let leaf = cursor
                        .file_name()
                        .ok_or_else(|| SetupError::PathEscape(path.to_path_buf()))?
                        .to_os_string();
                    missing.push(leaf);
                    cursor = cursor
                        .parent()
                        .ok_or_else(|| SetupError::PathEscape(path.to_path_buf()))?
                        .to_path_buf();
                }
                Err(error) => return Err(contained(error)),
            }
        }
    }

    fn create(&self) -> Result<RootDir, SetupError> {
        self.anchor.revalidate().map_err(contained)?;
        let mut directory = self.anchor.try_clone().map_err(contained)?;
        for component in &self.missing {
            directory.revalidate().map_err(contained)?;
            let component = component
                .to_str()
                .ok_or_else(|| SetupError::Io("directory name is not UTF-8".into()))?;
            directory = directory.subdirectory(component).map_err(contained)?;
        }
        directory.revalidate().map_err(contained)?;
        let file = File::from(directory.as_fd().try_clone_to_owned().map_err(io)?);
        file.set_permissions(Permissions::from_mode(0o700))
            .map_err(io)?;
        Ok(directory)
    }
}

pub struct FilesystemSetupWriter {
    config_path: PathBuf,
    config_parent: PlannedDirectory,
    directories: Vec<PlannedDirectory>,
}

impl FilesystemSetupWriter {
    pub fn new(config_path: PathBuf, directories: &[PathBuf]) -> Result<Self, SetupError> {
        let parent = config_path
            .parent()
            .ok_or_else(|| SetupError::PathEscape(config_path.clone()))?;
        Ok(Self {
            config_parent: PlannedDirectory::capture(parent)?,
            config_path,
            directories: directories
                .iter()
                .map(|path| PlannedDirectory::capture(path))
                .collect::<Result<_, _>>()?,
        })
    }
}

fn io(error: std::io::Error) -> SetupError {
    SetupError::Io(error.kind().to_string())
}

fn contained(error: ContainedError) -> SetupError {
    match error {
        ContainedError::Io {
            path,
            kind: std::io::ErrorKind::AlreadyExists,
        } => SetupError::Exists(path),
        ContainedError::Io { kind, .. } => SetupError::Io(kind.to_string()),
        ContainedError::Escape { path, .. }
        | ContainedError::NotRegular(path)
        | ContainedError::NotADirectory(path) => SetupError::PathEscape(path),
    }
}

impl SetupWriter for FilesystemSetupWriter {
    fn config_path(&self) -> PathBuf {
        self.config_path.clone()
    }

    fn config_exists(&self) -> bool {
        self.config_path.symlink_metadata().is_ok()
    }

    fn write_config(&self, text: &str, force: bool) -> Result<PathBuf, SetupError> {
        let root = self.config_parent.create()?;
        root.revalidate().map_err(contained)?;
        let name = self
            .config_path
            .file_name()
            .ok_or_else(|| SetupError::PathEscape(self.config_path.clone()))?;
        let name = Path::new(name);
        let mut file = if force {
            match root.open_file(name, Access::Write) {
                Ok(file) => file,
                Err(ContainedError::Io {
                    kind: std::io::ErrorKind::NotFound,
                    ..
                }) => root.create_file(name, 0o600).map_err(contained)?,
                Err(error) => return Err(contained(error)),
            }
        } else {
            root.create_file(name, 0o600).map_err(contained)?
        };
        file.set_permissions(Permissions::from_mode(0o600))
            .map_err(io)?;
        if force {
            file.set_len(0).map_err(io)?;
        }
        file.write_all(text.as_bytes()).map_err(io)?;
        file.sync_all().map_err(io)?;
        root.revalidate().map_err(contained)?;
        root.sync().map_err(contained)?;
        Ok(self.config_path.clone())
    }

    fn create_private_dir(&self, path: &Path) -> Result<(), SetupError> {
        let directory = self
            .directories
            .iter()
            .find(|directory| directory.path == path)
            .ok_or_else(|| SetupError::PathEscape(path.to_path_buf()))?;
        directory.create().map(|_| ())
    }
}
```

`crates/vpt/src/compose.rs`:

```rust
//! The composition root: the environment, the settings and the adapters every
//! command shares. Nothing here decides policy.

use std::path::PathBuf;
use vpt_adapters::config::{config_path, default_state_dir};

pub struct Environment {
    vars: Vec<(String, String)>,
}

impl Environment {
    pub fn from_process() -> Environment {
        Environment { vars: std::env::vars().collect() }
    }

    pub fn var(&self, name: &str) -> Option<String> {
        self.vars.iter().find(|(key, _)| key == name).map(|(_, value)| value.clone())
    }

    pub fn config_path(&self) -> PathBuf {
        config_path(|name| self.var(name))
    }

    pub fn home_dir(&self) -> PathBuf {
        PathBuf::from(self.var("HOME").unwrap_or_default())
    }

    pub fn state_dir_default(&self) -> String {
        default_state_dir(|name| self.var(name))
    }
}
```

`crates/vpt/src/commands/setup.rs`:

```rust
//! `vpt setup`.

use crate::cli::output::Outcome;
use crate::compose::Environment;
use serde_json::json;
use std::path::Path;
use vpt_adapters::config::{FilesystemSetupWriter, from_table, load_text, resolve, template};
use vpt_adapters::TtyPrompt;
use vpt_application::ports::PromptError;
use vpt_application::{Setup, SetupError, SetupWriter};
use vpt_domain::layout::StoreKey;
use vpt_protocol::error::{ErrorDocument, ErrorKind};
use vpt_protocol::result::document;

pub fn run(environment: &Environment, config: Option<&Path>, force: bool) -> Outcome {
    let selected = config
        .map(Path::to_path_buf)
        .unwrap_or_else(|| environment.config_path());
    let state_dir = environment.state_dir_default();
    let home_dir = environment.home_dir();
    let rendered_defaults = template("apple", &state_dir);
    let settings = match load_text(&rendered_defaults)
        .and_then(|table| from_table(&table, &home_dir))
    {
        Ok(settings) => settings,
        Err(error) => {
            return Outcome::Failure(ErrorDocument::new(
                ErrorKind::Config,
                format!("{error:?}"),
            ));
        }
    };
    let parent = selected
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let roots = match resolve(&settings, parent) {
        Ok(roots) => roots,
        Err(error) => {
            return Outcome::Failure(ErrorDocument::new(
                ErrorKind::Config,
                format!("{error:?}"),
            ));
        }
    };
    let Some(name) = selected.file_name() else {
        return Outcome::Failure(ErrorDocument::new(
            ErrorKind::Usage,
            "the configuration path needs a file name",
        ));
    };
    let mut directories = vec![roots.home.clone(), roots.state_dir.clone()];
    directories.extend(
        StoreKey::all()
            .iter()
            .map(|key| roots.stores.get(*key).to_path_buf()),
    );
    let writer = match FilesystemSetupWriter::new(
        roots.config_dir.join(name),
        &directories,
    ) {
        Ok(writer) => writer,
        Err(error) => return setup_failure(error),
    };
    if writer.config_exists() && !force {
        return setup_failure(SetupError::Exists(writer.config_path()));
    }
    let setup = Setup {
        engines: vec!["apple".into(), "whisply".into()],
        render: move |engine: &str| template(engine, &state_dir),
        directories,
    };
    let mut prompt = match TtyPrompt::open() {
        Ok(prompt) => prompt,
        Err(PromptError::NoTerminal) | Err(PromptError::Closed) => {
            return Outcome::Failure(ErrorDocument::new(ErrorKind::Usage, "vpt setup needs a controlling terminal"));
        }
    };
    match setup.run(&mut prompt, &writer, force) {
        Ok(outcome) => Outcome::Success {
            document: document("setup", json!({"written": outcome.written, "main_engine": outcome.main_engine})),
            human: format!("wrote {} (main engine {})\n", outcome.written.display(), outcome.main_engine),
        },
        Err(error) => setup_failure(error),
    }
}

fn setup_failure(error: SetupError) -> Outcome {
    let document = match error {
        SetupError::NoTerminal => ErrorDocument::new(
            ErrorKind::Usage,
            "vpt setup needs a controlling terminal",
        ),
        SetupError::Exists(path) => ErrorDocument::new(
            ErrorKind::Usage,
            format!("{} exists; pass --force to overwrite it", path.display()),
        ),
        SetupError::PathEscape(path) => ErrorDocument::new(
            ErrorKind::Refused,
            format!("{} escapes its prepared root", path.display()),
        )
        .rule("path_escape"),
        SetupError::Io(detail) => ErrorDocument::new(ErrorKind::Store, detail),
    };
    Outcome::Failure(document)
}
```

Resolution and overlap checks run before the terminal is opened or anything is created. The writer
retains the checked ancestors for the configuration, home, state and stores. Existing configuration
without `--force` is refused before prompting. The selected path includes `--config` and `VPT_CONFIG`
included. The `ConfigError` Debug form names keys and kinds, never values, so formatting it here quotes
nothing from the file. In `crates/vpt/src/lib.rs` the dispatch arm is

```rust
        Verb::Setup { force } => commands::setup::run(&Environment::from_process(), invocation.config.as_deref(), *force),
```

with `use compose::Environment;` at the top.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo fmt --all`

Run: `cargo test --workspace --features dev-tools`

Expected: all PASS, the pty test included.

- [ ] **Step 5: Commit**

Run: `cargo fmt --all`

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(setup): vpt setup writes the config, the home and the store leaves"
```

______________________________________________________________________

### Task 7: Time and digest value types

**Files:**

- Create: `crates/vpt-domain/src/time.rs`, `crates/vpt-domain/src/digest.rs`
- Modify: `crates/vpt-domain/src/lib.rs`

**Interfaces:**

- Consumes: nothing.

- Produces: `vpt_domain::time::{UtcInstant { pub secs: i64 }, UtcOffset { pub secs: i32 },`
  `FileTime { pub secs: i64, pub nanos: u32 }, Civil { pub year: i64, pub month: u32, pub day: u32,`
  `pub hour: u32, pub minute: u32, pub second: u32 }}` with `UtcInstant::rfc3339(self) -> String`,
  `UtcInstant::rfc3339_with(self, offset: UtcOffset) -> String`,
  `UtcInstant::civil(self, offset: UtcOffset) -> Civil`, `UtcOffset::label(self) -> String`,
  `Civil::instant(self, offset: UtcOffset) -> Option<UtcInstant>` (None when the instant leaves `i64`),
  `Civil::is_valid(self) -> bool` (Gregorian month, day and leap-year rules and the hour, minute and
  second bounds), `FileTime::age_secs(self, now: UtcInstant) -> i64` (whole seconds, nanoseconds counted,
  saturating at the `i64` bounds); `vpt_domain::digest::Sha256Digest(pub [u8; 32])` with
  `hex(&self) -> String`, `hash12(&self) -> String`, `hash8(&self) -> String`,
  `from_hex(text: &str) -> Option<Sha256Digest>`. Civil arithmetic uses `i128` intermediates, so every
  `i64` instant plus any offset converts.

- [ ] **Step 1: Write the failing tests**

Declare the modules first: `crates/vpt-domain/src/lib.rs` gains `pub mod digest;` and `pub mod time;`
beside `duration`, `layout` and `retention`. Each new file starts as its test module alone.

`crates/vpt-domain/src/time.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_epoch_renders_as_utc_midnight() {
        assert_eq!(UtcInstant { secs: 0 }.rfc3339(), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn an_offset_shifts_the_civil_time_and_is_printed_with_its_sign() {
        let captured = UtcInstant { secs: 1_787_604_456 };
        assert_eq!(captured.rfc3339(), "2026-08-24T20:47:36Z");
        assert_eq!(captured.rfc3339_with(UtcOffset { secs: -21_600 }), "2026-08-24T14:47:36-06:00");
        assert_eq!(captured.rfc3339_with(UtcOffset { secs: 19_800 }), "2026-08-25T02:17:36+05:30");
    }

    #[test]
    fn a_leap_day_is_a_real_date() {
        let civil = UtcInstant { secs: 1_709_164_800 }.civil(UtcOffset { secs: 0 });
        assert_eq!((civil.year, civil.month, civil.day), (2024, 2, 29));
    }

    #[test]
    fn a_civil_time_names_the_instant_it_came_from() {
        let captured = UtcInstant { secs: 1_787_604_456 };
        for offset in [UtcOffset { secs: -21_600 }, UtcOffset { secs: 19_800 }, UtcOffset { secs: 0 }] {
            assert_eq!(captured.civil(offset).instant(offset), Some(captured), "{}", offset.label());
        }
    }

    #[test]
    fn the_extremes_of_an_i64_instant_convert_without_overflow() {
        let late = UtcInstant { secs: i64::MAX }.civil(UtcOffset { secs: 50_400 });
        let early = UtcInstant { secs: i64::MIN }.civil(UtcOffset { secs: -50_400 });
        assert!(late.year > early.year);
        assert_eq!(late.instant(UtcOffset { secs: 50_400 }), Some(UtcInstant { secs: i64::MAX }));
        assert_eq!(late.instant(UtcOffset { secs: -50_400 }), None);
    }

    #[test]
    fn the_calendar_decides_which_civil_times_exist() {
        let base = Civil { year: 2024, month: 2, day: 29, hour: 23, minute: 59, second: 59 };
        assert!(base.is_valid());
        assert!(!Civil { year: 2023, ..base }.is_valid());
        assert!(Civil { year: 2000, ..base }.is_valid());
        assert!(!Civil { year: 1900, ..base }.is_valid());
        assert!(!Civil { month: 4, day: 31, ..base }.is_valid());
        assert!(!Civil { month: 0, ..base }.is_valid());
        assert!(!Civil { month: 13, ..base }.is_valid());
        assert!(!Civil { day: 0, ..base }.is_valid());
        assert!(!Civil { hour: 24, ..base }.is_valid());
        assert!(!Civil { minute: 60, ..base }.is_valid());
        assert!(!Civil { second: 60, ..base }.is_valid());
    }

    #[test]
    fn a_file_time_ages_by_whole_seconds_counting_its_nanoseconds() {
        let mtime = FileTime { secs: 100, nanos: 999_999_999 };
        assert_eq!(mtime.age_secs(UtcInstant { secs: 130 }), 29);
        assert_eq!(FileTime { secs: 100, nanos: 0 }.age_secs(UtcInstant { secs: 130 }), 30);
        assert_eq!(FileTime { secs: i64::MIN, nanos: 1 }.age_secs(UtcInstant { secs: i64::MAX }), i64::MAX);
    }
}
```

`crates/vpt-domain/src/digest.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trips_and_the_prefixes_are_its_leading_characters() {
        let mut bytes = [0u8; 32];
        bytes[..6].copy_from_slice(&[0x4f, 0x3a, 0xb1, 0x9c, 0x02, 0xde]);
        let digest = Sha256Digest(bytes);
        assert!(digest.hex().starts_with("4f3ab19c02de"));
        assert_eq!(digest.hex().len(), 64);
        assert_eq!(digest.hash12(), "4f3ab19c02de");
        assert_eq!(digest.hash8(), "4f3ab19c");
        assert_eq!(Sha256Digest::from_hex(&digest.hex()), Some(digest));
    }

    #[test]
    fn malformed_hex_is_refused() {
        assert_eq!(Sha256Digest::from_hex("zz"), None);
        assert_eq!(Sha256Digest::from_hex(&"ab".repeat(31)), None);
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-domain`

Expected: the build of the crate's tests fails with `cannot find` for `UtcInstant`, `UtcOffset`,
`FileTime`, `Civil` and `Sha256Digest`. A run that succeeds does not satisfy this step.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-domain/src/time.rs`, above its test module:

```rust
//! Instants, offsets and file times, with the civil-date arithmetic the
//! identity and the notes need. No clock lives here.

const NANOS_PER_SECOND: i128 = 1_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UtcInstant {
    pub secs: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UtcOffset {
    pub secs: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileTime {
    pub secs: i64,
    pub nanos: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Civil {
    pub year: i64,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

impl UtcInstant {
    pub fn civil(self, offset: UtcOffset) -> Civil {
        let local = i128::from(self.secs) + i128::from(offset.secs);
        let days = local.div_euclid(86_400);
        let seconds = local.rem_euclid(86_400);
        let (year, month, day) = civil_from_days(days);
        Civil {
            year,
            month,
            day,
            hour: (seconds / 3_600) as u32,
            minute: ((seconds % 3_600) / 60) as u32,
            second: (seconds % 60) as u32,
        }
    }

    pub fn rfc3339(self) -> String {
        let c = self.civil(UtcOffset { secs: 0 });
        format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", c.year, c.month, c.day, c.hour, c.minute, c.second)
    }

    pub fn rfc3339_with(self, offset: UtcOffset) -> String {
        let c = self.civil(offset);
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}{}",
            c.year, c.month, c.day, c.hour, c.minute, c.second, offset.label()
        )
    }
}

impl UtcOffset {
    pub fn label(self) -> String {
        let sign = if self.secs < 0 { '-' } else { '+' };
        let magnitude = self.secs.unsigned_abs();
        format!("{sign}{:02}:{:02}", magnitude / 3_600, (magnitude % 3_600) / 60)
    }
}

impl Civil {
    /// The instant this civil time names when read at `offset`.
    pub fn instant(self, offset: UtcOffset) -> Option<UtcInstant> {
        let seconds = i128::from(self.hour) * 3_600 + i128::from(self.minute) * 60 + i128::from(self.second);
        let local = days_from_civil(self.year, self.month, self.day) * 86_400 + seconds;
        i64::try_from(local - i128::from(offset.secs)).ok().map(|secs| UtcInstant { secs })
    }

    pub fn is_valid(self) -> bool {
        (1..=12).contains(&self.month)
            && (1..=days_in_month(self.year, self.month)).contains(&self.day)
            && self.hour < 24
            && self.minute < 60
            && self.second < 60
    }
}

impl FileTime {
    /// Whole seconds from this file time to `now`, the nanoseconds counted.
    pub fn age_secs(self, now: UtcInstant) -> i64 {
        let elapsed = (i128::from(now.secs) - i128::from(self.secs)) * NANOS_PER_SECOND - i128::from(self.nanos);
        elapsed.div_euclid(NANOS_PER_SECOND).clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
    }
}

fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        _ => 28,
    }
}

/// Days since 1970-01-01 to a proleptic Gregorian date (Howard Hinnant's algorithm).
fn civil_from_days(days: i128) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = if m <= 2 { y + 1 } else { y };
    (i64::try_from(year).expect("an i64 instant spans fewer than i64 years"), m, d)
}

/// A proleptic Gregorian date to days since 1970-01-01 (the inverse of `civil_from_days`).
fn days_from_civil(year: i64, month: u32, day: u32) -> i128 {
    let y = i128::from(year) - i128::from(month <= 2);
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let m = i128::from(month);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i128::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}
```

`crates/vpt-domain/src/digest.rs`, above its test module:

```rust
//! A SHA-256 digest and the prefixes the identity and note names use.

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Sha256Digest(pub [u8; 32]);

impl Sha256Digest {
    pub fn hex(&self) -> String {
        self.0.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    pub fn hash12(&self) -> String {
        self.hex()[..12].to_owned()
    }

    pub fn hash8(&self) -> String {
        self.hex()[..8].to_owned()
    }

    pub fn from_hex(text: &str) -> Option<Sha256Digest> {
        if text.len() != 64 || !text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }
        let mut bytes = [0u8; 32];
        for (index, chunk) in text.as_bytes().chunks(2).enumerate() {
            bytes[index] = u8::from_str_radix(std::str::from_utf8(chunk).ok()?, 16).ok()?;
        }
        Some(Sha256Digest(bytes))
    }
}

impl std::fmt::Debug for Sha256Digest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Sha256Digest({})", self.hex())
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo fmt --all`

Run: `cargo test -p vpt-domain`

Expected: the nine tests of `time` and `digest` PASS alongside the earlier domain tests. Run
`cargo clippy -p vpt-domain --all-targets -- -D warnings` and expect no warnings.

- [ ] **Step 5: Commit**

Run: `cargo fmt --all`

```bash
git add crates/vpt-domain
SKIP_AI_COMMIT=1 git commit -m "feat(domain): instants, offsets, file times and digests"
```

______________________________________________________________________

### Task 8: The recording identity

**Files:**

- Create: `crates/vpt-domain/src/identity.rs`
- Modify: `crates/vpt-domain/src/lib.rs`

**Interfaces:**

- Consumes: `time::{Civil, UtcInstant, UtcOffset}`, `digest::Sha256Digest`.

- Produces: `vpt_domain::identity::{RecordingId, IdentityError::{Malformed, Unrepresentable},`
  `local_timestamp(captured: UtcInstant, offset: UtcOffset) -> String,`
  `parse_local_timestamp(text: &str) -> Result<Civil, IdentityError>}` (the `YYYY-MM-DDThhmmss` form,
  refused unless the civil time exists on the calendar);
  `RecordingId::derive(captured: UtcInstant, offset: UtcOffset, digest: &Sha256Digest) ->`
  `Result<RecordingId, IdentityError>` (`Unrepresentable` when the local year has no four-digit form),
  `RecordingId::parse(text: &str) -> Result<RecordingId, IdentityError>` (the shape, then the Gregorian
  month, day and leap-year rules and the hour, minute and second bounds), `fn as_str(&self) -> &str`,
  `fn local_timestamp(&self) -> &str`, `fn capture_date(&self) -> &str`, `fn hash12(&self) -> &str`.
  Every call site uses the fallible `derive`; ingest maps an unrepresentable capture instant to the
  `invalid_container` deferral.

- [ ] **Step 1: Write the failing tests**

`crates/vpt-domain/src/lib.rs` gains `pub mod identity;`. `crates/vpt-domain/src/identity.rs` starts as
its test module alone:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn digest() -> Sha256Digest {
        let mut bytes = [0u8; 32];
        bytes[..6].copy_from_slice(&[0x4f, 0x3a, 0xb1, 0x9c, 0x02, 0xde]);
        Sha256Digest(bytes)
    }

    #[test]
    fn the_identity_is_the_local_capture_time_and_twelve_hex_characters() {
        let id = RecordingId::derive(UtcInstant { secs: 1_787_604_456 }, UtcOffset { secs: -21_600 }, &digest())
            .expect("representable");
        assert_eq!(id.as_str(), "2026-08-24T144736-4f3ab19c02de");
        assert_eq!(id.local_timestamp(), "2026-08-24T144736");
        assert_eq!(id.capture_date(), "2026-08-24");
        assert_eq!(id.hash12(), "4f3ab19c02de");
    }

    #[test]
    fn the_timestamp_carries_no_colon() {
        assert_eq!(local_timestamp(UtcInstant { secs: 0 }, UtcOffset { secs: 0 }), "1970-01-01T000000");
    }

    #[test]
    fn a_well_formed_identity_parses_and_a_malformed_one_is_refused() {
        assert!(RecordingId::parse("2026-08-24T144736-4f3ab19c02de").is_ok());
        assert_eq!(RecordingId::parse("2026-08-24-4f3ab19c02de"), Err(IdentityError::Malformed));
        assert_eq!(RecordingId::parse("2026-08-24T144736-4f3ab19c02dg"), Err(IdentityError::Malformed));
        assert_eq!(RecordingId::parse("2026-08-24T144736-4F3AB19C02DE"), Err(IdentityError::Malformed));
        assert_eq!(RecordingId::parse("../2026-08-24T144736-4f3ab19c02de"), Err(IdentityError::Malformed));
    }

    #[test]
    fn an_impossible_date_or_time_is_refused_in_the_right_shape() {
        assert_eq!(RecordingId::parse("2023-02-29T144736-4f3ab19c02de"), Err(IdentityError::Malformed));
        assert!(RecordingId::parse("2024-02-29T144736-4f3ab19c02de").is_ok());
        assert_eq!(RecordingId::parse("2026-13-01T000000-4f3ab19c02de"), Err(IdentityError::Malformed));
        assert_eq!(RecordingId::parse("2026-04-31T000000-4f3ab19c02de"), Err(IdentityError::Malformed));
        assert_eq!(RecordingId::parse("2026-08-24T240000-4f3ab19c02de"), Err(IdentityError::Malformed));
        assert_eq!(RecordingId::parse("2026-08-24T146000-4f3ab19c02de"), Err(IdentityError::Malformed));
        assert_eq!(RecordingId::parse("2026-08-24T144760-4f3ab19c02de"), Err(IdentityError::Malformed));
    }

    #[test]
    fn the_local_timestamp_parses_back_to_its_civil_time() {
        let civil = parse_local_timestamp("2026-08-24T144736").expect("civil");
        assert_eq!((civil.year, civil.month, civil.day), (2026, 8, 24));
        assert_eq!((civil.hour, civil.minute, civil.second), (14, 47, 36));
        assert_eq!(civil.instant(UtcOffset { secs: -21_600 }), Some(UtcInstant { secs: 1_787_604_456 }));
        assert_eq!(parse_local_timestamp("2026-08-24T14:47:36"), Err(IdentityError::Malformed));
        assert_eq!(parse_local_timestamp("2026-08-24T144736-4f3ab19c02de"), Err(IdentityError::Malformed));
    }

    #[test]
    fn a_capture_outside_the_four_digit_years_is_unrepresentable() {
        let year_ten_thousand = UtcInstant { secs: 253_402_300_800 };
        assert_eq!(
            RecordingId::derive(year_ten_thousand, UtcOffset { secs: 0 }, &digest()),
            Err(IdentityError::Unrepresentable)
        );
        assert!(RecordingId::derive(year_ten_thousand, UtcOffset { secs: -3_600 }, &digest()).is_ok());
        let before_year_zero = UtcInstant { secs: -62_167_219_201 };
        assert_eq!(
            RecordingId::derive(before_year_zero, UtcOffset { secs: 0 }, &digest()),
            Err(IdentityError::Unrepresentable)
        );
        assert_eq!(
            RecordingId::derive(UtcInstant { secs: i64::MAX }, UtcOffset { secs: 0 }, &digest()),
            Err(IdentityError::Unrepresentable)
        );
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-domain identity`

Expected: the build fails with `cannot find` for `RecordingId`, `IdentityError`, `local_timestamp` and
`parse_local_timestamp`.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-domain/src/identity.rs`, above its test module:

```rust
//! `<local-capture-timestamp>-<hash12>`: both halves come from the file.

use crate::digest::Sha256Digest;
use crate::time::{Civil, UtcInstant, UtcOffset};

const TIMESTAMP_LEN: usize = 17;
const IDENTITY_LEN: usize = 30;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RecordingId(String);

#[derive(Debug, PartialEq, Eq)]
pub enum IdentityError {
    Malformed,
    Unrepresentable,
}

pub fn local_timestamp(captured: UtcInstant, offset: UtcOffset) -> String {
    let c = captured.civil(offset);
    format!("{:04}-{:02}-{:02}T{:02}{:02}{:02}", c.year, c.month, c.day, c.hour, c.minute, c.second)
}

/// `YYYY-MM-DDThhmmss` back to a civil time that exists on the calendar.
pub fn parse_local_timestamp(text: &str) -> Result<Civil, IdentityError> {
    let bytes = text.as_bytes();
    let shape_ok = bytes.len() == TIMESTAMP_LEN
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[10] == b'T'
        && bytes[..4].iter().chain(&bytes[5..7]).chain(&bytes[8..10]).chain(&bytes[11..]).all(u8::is_ascii_digit);
    if !shape_ok {
        return Err(IdentityError::Malformed);
    }
    let field = |range: std::ops::Range<usize>| text[range].parse::<u32>().map_err(|_| IdentityError::Malformed);
    let civil = Civil {
        year: i64::from(field(0..4)?),
        month: field(5..7)?,
        day: field(8..10)?,
        hour: field(11..13)?,
        minute: field(13..15)?,
        second: field(15..17)?,
    };
    if civil.is_valid() { Ok(civil) } else { Err(IdentityError::Malformed) }
}

impl RecordingId {
    /// The identity of a capture at `captured` read in `offset`; a local year
    /// outside `0000` to `9999` has no identity.
    pub fn derive(captured: UtcInstant, offset: UtcOffset, digest: &Sha256Digest) -> Result<RecordingId, IdentityError> {
        RecordingId::parse(&format!("{}-{}", local_timestamp(captured, offset), digest.hash12()))
            .map_err(|_| IdentityError::Unrepresentable)
    }

    pub fn parse(text: &str) -> Result<RecordingId, IdentityError> {
        let bytes = text.as_bytes();
        let shape_ok = bytes.len() == IDENTITY_LEN
            && bytes[TIMESTAMP_LEN] == b'-'
            && bytes[TIMESTAMP_LEN + 1..].iter().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b));
        if !shape_ok {
            return Err(IdentityError::Malformed);
        }
        parse_local_timestamp(&text[..TIMESTAMP_LEN])?;
        Ok(RecordingId(text.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn local_timestamp(&self) -> &str {
        &self.0[..TIMESTAMP_LEN]
    }

    pub fn capture_date(&self) -> &str {
        &self.0[..10]
    }

    pub fn hash12(&self) -> &str {
        &self.0[TIMESTAMP_LEN + 1..]
    }
}

impl std::fmt::Display for RecordingId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
```

The byte at index 17 is checked to be the ASCII hyphen before the text is sliced there, so the slice
never lands inside a multi-byte character.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo fmt --all`

Run: `cargo test -p vpt-domain identity`

Expected: 6 tests PASS. Run `cargo clippy -p vpt-domain --all-targets -- -D warnings` and expect no
warnings.

- [ ] **Step 5: Commit**

Run: `cargo fmt --all`

```bash
git add crates/vpt-domain
SKIP_AI_COMMIT=1 git commit -m "feat(domain): the recording identity"
```

______________________________________________________________________

### Task 9: The MPEG-4 wholeness gate

The gate walks the top-level boxes reading bounded headers with checked offsets, never the whole file;
the sum of box lengths must equal the file size exactly and a `moov` box with an `mvhd` must be present.
The `mvhd` supplies the capture time (seconds since 1904-01-01 UTC) and the duration. The domain owns no
I/O: the gate takes the file's length and a bounded read callback, and the adapter that holds the
descriptor supplies both.

**Files:**

- Create: `crates/vpt-domain/src/container.rs`, `crates/vpt-domain/src/fixtures.rs`
- Modify: `crates/vpt-domain/src/lib.rs`

**Interfaces:**

- Consumes: `time::UtcInstant`.

- Produces: `vpt_domain::container::{ReadFailure,`
  `Container { pub creation_time: UtcInstant, pub duration_secs: u64 }, ContainerError,`
  `inspect(len: u64, read: impl FnMut(u64, &mut [u8]) -> Result<(), ReadFailure>) ->`
  `Result<Container, ContainerError>}` where the callback fills the whole buffer from the given offset or
  fails; `ContainerError::{PastEnd { offset: u64 }, InvalidLength { offset: u64 }, Overflow,`
  `MissingMoov, MissingMvhd, InvalidMvhd, Unreadable}`;
  `vpt_domain::fixtures::{m4a(creation_unix: i64, duration_secs: u32, payload: &[u8]) ->`
  `Vec<u8>, box_of(kind: &[u8; 4], body: &[u8]) -> Vec<u8>, mvhd(creation_unix: i64,`
  `duration_secs: u32) -> Vec<u8>, mvhd_v1(creation_unix: i64, duration_secs: u64) -> Vec<u8>,`
  `reader(bytes: &[u8]) -> impl FnMut(u64, &mut [u8]) -> Result<(), ReadFailure> + '_,`
  `inspect_bytes(bytes: &[u8]) -> Result<Container, ContainerError>}` behind the `fixtures` feature.

- [ ] **Step 1: Write the failing tests**

Declare the modules first: `crates/vpt-domain/src/lib.rs` gains `pub mod container;` and, under
`#[cfg(any(test, feature = "fixtures"))]`, `pub mod fixtures;`. The fixtures are test support, so
`crates/vpt-domain/src/fixtures.rs` is written whole now:

```rust
//! Assembled MPEG-4 bytes for tests. Behind the `fixtures` feature so other
//! crates' tests can build the same files.

use crate::container::{Container, ContainerError, ReadFailure, inspect};

const MAC_EPOCH_OFFSET: i64 = 2_082_844_800;

/// A bounded read over a byte slice, the shape `inspect` takes.
pub fn reader(bytes: &[u8]) -> impl FnMut(u64, &mut [u8]) -> Result<(), ReadFailure> + '_ {
    move |offset, buf| {
        let start = usize::try_from(offset).map_err(|_| ReadFailure)?;
        let end = start.checked_add(buf.len()).ok_or(ReadFailure)?;
        buf.copy_from_slice(bytes.get(start..end).ok_or(ReadFailure)?);
        Ok(())
    }
}

/// The gate over a whole byte slice.
pub fn inspect_bytes(bytes: &[u8]) -> Result<Container, ContainerError> {
    inspect(bytes.len() as u64, reader(bytes))
}

pub fn box_of(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut out = ((8 + body.len()) as u32).to_be_bytes().to_vec();
    out.extend_from_slice(kind);
    out.extend_from_slice(body);
    out
}

/// A version 0 `mvhd` with a timescale of 1000.
pub fn mvhd(creation_unix: i64, duration_secs: u32) -> Vec<u8> {
    let mac = (creation_unix + MAC_EPOCH_OFFSET) as u32;
    let mut body = vec![0u8; 4];
    body.extend(mac.to_be_bytes());
    body.extend(mac.to_be_bytes());
    body.extend(1_000u32.to_be_bytes());
    body.extend((duration_secs * 1_000).to_be_bytes());
    body.extend([0u8; 80]);
    box_of(b"mvhd", &body)
}

/// A version 1 `mvhd` with 64-bit times and a timescale of 1000.
pub fn mvhd_v1(creation_unix: i64, duration_secs: u64) -> Vec<u8> {
    let mac = (creation_unix + MAC_EPOCH_OFFSET) as u64;
    let mut body = vec![1u8, 0, 0, 0];
    body.extend(mac.to_be_bytes());
    body.extend(mac.to_be_bytes());
    body.extend(1_000u32.to_be_bytes());
    body.extend((duration_secs * 1_000).to_be_bytes());
    body.extend([0u8; 80]);
    box_of(b"mvhd", &body)
}

/// `ftyp`, `mdat`, then `moov` last, the way Voice Memos writes them.
pub fn m4a(creation_unix: i64, duration_secs: u32, payload: &[u8]) -> Vec<u8> {
    let mut out = box_of(b"ftyp", b"M4A \0\0\0\0M4A mp42isom");
    out.extend(box_of(b"mdat", payload));
    out.extend(box_of(b"moov", &mvhd(creation_unix, duration_secs)));
    out
}
```

`crates/vpt-domain/src/container.rs` starts as its test module alone:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::{box_of, inspect_bytes, m4a, mvhd, mvhd_v1, reader};

    const CAPTURED: i64 = 1_787_604_456;

    #[test]
    fn a_whole_file_yields_its_capture_time_and_duration() {
        let container = inspect_bytes(&m4a(CAPTURED, 612, b"audio")).expect("whole");
        assert_eq!(container.creation_time, UtcInstant { secs: CAPTURED });
        assert_eq!(container.duration_secs, 612);
    }

    #[test]
    fn a_truncated_download_loses_moov_and_is_refused_past_the_end() {
        let mut bytes = m4a(CAPTURED, 612, b"audio");
        bytes.truncate(bytes.len() - 10);
        assert!(matches!(inspect_bytes(&bytes), Err(ContainerError::PastEnd { .. })));
    }

    #[test]
    fn a_file_without_moov_is_refused() {
        let mut bytes = box_of(b"ftyp", b"M4A ");
        bytes.extend(box_of(b"mdat", b"audio"));
        assert_eq!(inspect_bytes(&bytes), Err(ContainerError::MissingMoov));
    }

    #[test]
    fn a_moov_without_mvhd_is_refused() {
        let mut bytes = box_of(b"ftyp", b"M4A ");
        bytes.extend(box_of(b"moov", &box_of(b"udta", b"")));
        assert_eq!(inspect_bytes(&bytes), Err(ContainerError::MissingMvhd));
    }

    #[test]
    fn a_box_length_below_its_header_is_invalid() {
        let mut bytes = vec![0, 0, 0, 4];
        bytes.extend(b"ftyp");
        assert_eq!(inspect_bytes(&bytes), Err(ContainerError::InvalidLength { offset: 0 }));
    }

    #[test]
    fn a_largesize_that_overflows_is_refused_not_wrapped() {
        let mut bytes = box_of(b"free", b"");
        bytes.extend([0, 0, 0, 1]);
        bytes.extend(b"mdat");
        bytes.extend(u64::MAX.to_be_bytes());
        bytes.extend([0u8; 8]);
        assert_eq!(inspect_bytes(&bytes), Err(ContainerError::Overflow));
    }

    #[test]
    fn an_extended_header_that_runs_past_the_end_is_past_end_not_unreadable() {
        let mut bytes = vec![0, 0, 0, 1];
        bytes.extend(b"mdat");
        bytes.extend([0u8; 4]);
        assert_eq!(inspect_bytes(&bytes), Err(ContainerError::PastEnd { offset: 0 }));
    }

    #[test]
    fn a_version_one_mvhd_is_read_with_its_64_bit_fields() {
        let mut bytes = box_of(b"ftyp", b"M4A ");
        bytes.extend(box_of(b"moov", &mvhd_v1(CAPTURED, 7_200)));
        let container = inspect_bytes(&bytes).expect("v1");
        assert_eq!(container.creation_time.secs, CAPTURED);
        assert_eq!(container.duration_secs, 7_200);
    }

    #[test]
    fn a_zero_timescale_is_an_invalid_mvhd() {
        let mut body = mvhd(CAPTURED, 1);
        body[8 + 12..8 + 16].copy_from_slice(&[0, 0, 0, 0]);
        let mut bytes = box_of(b"ftyp", b"M4A ");
        bytes.extend(box_of(b"moov", &body));
        assert_eq!(inspect_bytes(&bytes), Err(ContainerError::InvalidMvhd));
    }

    #[test]
    fn a_creation_time_beyond_i64_is_an_invalid_mvhd() {
        let mut body = mvhd_v1(CAPTURED, 1);
        body[8 + 4..8 + 12].copy_from_slice(&u64::MAX.to_be_bytes());
        let mut bytes = box_of(b"ftyp", b"M4A ");
        bytes.extend(box_of(b"moov", &body));
        assert_eq!(inspect_bytes(&bytes), Err(ContainerError::InvalidMvhd));
    }

    #[test]
    fn a_failed_read_is_unreadable() {
        let bytes = m4a(CAPTURED, 1, b"audio");
        let outcome = inspect(bytes.len() as u64, |_offset, _buf: &mut [u8]| Err(ReadFailure));
        assert_eq!(outcome, Err(ContainerError::Unreadable));
    }

    #[test]
    fn only_bounded_headers_are_read_never_the_payload() {
        let bytes = m4a(CAPTURED, 1, &[0u8; 100_000]);
        let mut source = reader(&bytes);
        let mut read = 0u64;
        let container = inspect(bytes.len() as u64, |offset, buf: &mut [u8]| {
            read += buf.len() as u64;
            source(offset, buf)
        });
        container.expect("whole");
        assert!(read < 1_000, "read {read} bytes");
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-domain --features fixtures container`

Expected: the build fails with `cannot find` for `inspect`, `Container`, `ContainerError` and
`ReadFailure`, reported from both `fixtures.rs` and the test module.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-domain/src/container.rs`, above its test module:

```rust
//! The wholeness gate: bounded box headers with checked offsets, the sum of
//! box lengths equal to the file size, `moov` present, `mvhd` inside it.

use crate::time::UtcInstant;

const MAC_EPOCH_OFFSET: i64 = 2_082_844_800;

#[derive(Debug, PartialEq, Eq)]
pub struct ReadFailure;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Container {
    pub creation_time: UtcInstant,
    pub duration_secs: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ContainerError {
    PastEnd { offset: u64 },
    InvalidLength { offset: u64 },
    Overflow,
    MissingMoov,
    MissingMvhd,
    InvalidMvhd,
    Unreadable,
}

struct Header {
    kind: [u8; 4],
    length: u64,
    header_len: u64,
}

/// Walk the boxes of a file `len` bytes long through `read`, which fills a
/// buffer from an offset or fails.
pub fn inspect(
    len: u64,
    mut read: impl FnMut(u64, &mut [u8]) -> Result<(), ReadFailure>,
) -> Result<Container, ContainerError> {
    let mut offset = 0;
    let mut moov = None;
    while offset < len {
        let header = read_header(&mut read, offset, len)?;
        if &header.kind == b"moov" {
            moov = Some((offset + header.header_len, offset + header.length));
        }
        offset = offset.checked_add(header.length).ok_or(ContainerError::Overflow)?;
    }
    let (start, end) = moov.ok_or(ContainerError::MissingMoov)?;
    let mut child = start;
    while child < end {
        let header = read_header(&mut read, child, end)?;
        if &header.kind == b"mvhd" {
            return read_mvhd(&mut read, child + header.header_len, header.length - header.header_len);
        }
        child = child.checked_add(header.length).ok_or(ContainerError::Overflow)?;
    }
    Err(ContainerError::MissingMvhd)
}

fn read_header(
    read: &mut impl FnMut(u64, &mut [u8]) -> Result<(), ReadFailure>,
    offset: u64,
    end: u64,
) -> Result<Header, ContainerError> {
    if offset.checked_add(8).ok_or(ContainerError::Overflow)? > end {
        return Err(ContainerError::PastEnd { offset });
    }
    let mut head = [0u8; 8];
    read(offset, &mut head).map_err(|_| ContainerError::Unreadable)?;
    let size32 = u32::from_be_bytes([head[0], head[1], head[2], head[3]]);
    let kind = [head[4], head[5], head[6], head[7]];
    let (length, header_len) = match size32 {
        0 => (end - offset, 8),
        1 => {
            if offset.checked_add(16).ok_or(ContainerError::Overflow)? > end {
                return Err(ContainerError::PastEnd { offset });
            }
            let mut large = [0u8; 8];
            read(offset + 8, &mut large).map_err(|_| ContainerError::Unreadable)?;
            (u64::from_be_bytes(large), 16)
        }
        n => (u64::from(n), 8),
    };
    if length < header_len {
        return Err(ContainerError::InvalidLength { offset });
    }
    let box_end = offset.checked_add(length).ok_or(ContainerError::Overflow)?;
    if box_end > end {
        return Err(ContainerError::PastEnd { offset });
    }
    Ok(Header { kind, length, header_len })
}

fn read_mvhd(
    read: &mut impl FnMut(u64, &mut [u8]) -> Result<(), ReadFailure>,
    body: u64,
    body_len: u64,
) -> Result<Container, ContainerError> {
    let mut version = [0u8; 1];
    if body_len < 1 {
        return Err(ContainerError::InvalidMvhd);
    }
    read(body, &mut version).map_err(|_| ContainerError::Unreadable)?;
    let (creation, timescale, duration) = match version[0] {
        0 => {
            if body_len < 20 {
                return Err(ContainerError::InvalidMvhd);
            }
            let mut fields = [0u8; 16];
            read(body + 4, &mut fields).map_err(|_| ContainerError::Unreadable)?;
            (
                u64::from(u32::from_be_bytes(fields[0..4].try_into().map_err(|_| ContainerError::InvalidMvhd)?)),
                u32::from_be_bytes(fields[8..12].try_into().map_err(|_| ContainerError::InvalidMvhd)?),
                u64::from(u32::from_be_bytes(fields[12..16].try_into().map_err(|_| ContainerError::InvalidMvhd)?)),
            )
        }
        1 => {
            if body_len < 32 {
                return Err(ContainerError::InvalidMvhd);
            }
            let mut fields = [0u8; 28];
            read(body + 4, &mut fields).map_err(|_| ContainerError::Unreadable)?;
            (
                u64::from_be_bytes(fields[0..8].try_into().map_err(|_| ContainerError::InvalidMvhd)?),
                u32::from_be_bytes(fields[16..20].try_into().map_err(|_| ContainerError::InvalidMvhd)?),
                u64::from_be_bytes(fields[20..28].try_into().map_err(|_| ContainerError::InvalidMvhd)?),
            )
        }
        _ => return Err(ContainerError::InvalidMvhd),
    };
    if timescale == 0 {
        return Err(ContainerError::InvalidMvhd);
    }
    let creation_unix = i64::try_from(creation).map_err(|_| ContainerError::InvalidMvhd)? - MAC_EPOCH_OFFSET;
    Ok(Container { creation_time: UtcInstant { secs: creation_unix }, duration_secs: duration / u64::from(timescale) })
}
```

`body + 4` and `offset + 8` follow a check that the box holds those bytes, so neither can overflow.
`crates/vpt-domain/src/lib.rs` now reads:

```rust
//! Pure policy: identity, the wholeness gate, sweep gates, retention and value types.

pub mod container;
pub mod digest;
pub mod duration;
#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
pub mod identity;
pub mod layout;
pub mod retention;
pub mod time;
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo fmt --all`

Run: `cargo test -p vpt-domain --features fixtures container`

Expected: 12 tests PASS. Run
`cargo clippy -p vpt-domain --all-targets --features fixtures -- -D warnings` and expect no warnings.

- [ ] **Step 5: Commit**

Run: `cargo fmt --all`

```bash
git add crates/vpt-domain
SKIP_AI_COMMIT=1 git commit -m "feat(domain): the MPEG-4 wholeness gate and the assembled fixtures"
```

______________________________________________________________________

### Task 10: The sweep gates

**Files:**

- Create: `crates/vpt-domain/src/sweep.rs`
- Modify: `crates/vpt-domain/src/lib.rs`

**Interfaces:**

- Consumes: `time::{FileTime, UtcInstant}`.

- Produces: `vpt_domain::sweep::{DeferralReason, CandidateFacts, SeenFacts, SweepLimits, PreOpen,`
  `pre_open, size_gate, rest_gate, SF_DATALESS}`;
  `CandidateFacts { pub size: u64, pub mtime: FileTime, pub flags: u32 }` (the source's whole `st_flags`
  word; `SF_DATALESS` is `0x4000_0000`, the bit macOS sets on a file whose bytes are still in iCloud);
  `SeenFacts { pub size: u64, pub mtime: FileTime, pub ingested: bool, pub deferred_size: Option<u64> }`;
  `SweepLimits { pub max_audio_bytes: u64, pub quiet_period_secs: u64 }`;
  `DeferralReason::{Dataless, AudioTooLarge, InvalidContainer, NotAtRest, ChangedDuringRead}` with
  `fn as_str(self) -> &'static str` and `fn parse(text: &str) -> Option<DeferralReason>`;
  `PreOpen::{Dataless, Unchanged, Open}`;
  `pre_open(candidate: &CandidateFacts, seen: Option<&SeenFacts>) -> PreOpen`;
  `size_gate(size: u64, limits: &SweepLimits) -> Result<(), DeferralReason>`;
  `rest_gate(mtime: FileTime, size: u64, now: UtcInstant, seen: Option<&SeenFacts>,`
  `limits: &SweepLimits) -> Result<(), DeferralReason>`.

- [ ] **Step 1: Write the failing tests**

`crates/vpt-domain/src/lib.rs` gains `pub mod sweep;`. `crates/vpt-domain/src/sweep.rs` starts as its
test module alone:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const LIMITS: SweepLimits = SweepLimits { max_audio_bytes: 2_147_483_648, quiet_period_secs: 30 };
    const UF_HIDDEN: u32 = 0x0000_8000;

    fn candidate(size: u64, secs: i64, flags: u32) -> CandidateFacts {
        CandidateFacts { size, mtime: FileTime { secs, nanos: 0 }, flags }
    }

    #[test]
    fn a_dataless_entry_is_never_opened_whatever_else_is_set() {
        assert_eq!(pre_open(&candidate(10, 0, SF_DATALESS), None), PreOpen::Dataless);
        assert_eq!(pre_open(&candidate(10, 0, SF_DATALESS | UF_HIDDEN), None), PreOpen::Dataless);
        assert_eq!(pre_open(&candidate(10, 0, UF_HIDDEN), None), PreOpen::Open);
    }

    #[test]
    fn an_unchanged_ingested_triple_is_skipped_and_a_changed_one_is_opened() {
        let seen = SeenFacts { size: 10, mtime: FileTime { secs: 5, nanos: 0 }, ingested: true, deferred_size: None };
        assert_eq!(pre_open(&candidate(10, 5, 0), Some(&seen)), PreOpen::Unchanged);
        assert_eq!(pre_open(&candidate(11, 5, 0), Some(&seen)), PreOpen::Open);
        assert_eq!(pre_open(&candidate(10, 6, 0), Some(&seen)), PreOpen::Open);
        let deferred = SeenFacts { ingested: false, ..seen };
        assert_eq!(pre_open(&candidate(10, 5, 0), Some(&deferred)), PreOpen::Open);
    }

    #[test]
    fn the_size_gate_defers_one_byte_over_the_limit_and_accepts_the_limit() {
        assert_eq!(size_gate(2_147_483_648, &LIMITS), Ok(()));
        assert_eq!(size_gate(2_147_483_649, &LIMITS), Err(DeferralReason::AudioTooLarge));
    }

    #[test]
    fn the_rest_gate_needs_the_whole_quiet_period_to_the_nanosecond() {
        let now = UtcInstant { secs: 1_000 };
        assert_eq!(rest_gate(FileTime { secs: 971, nanos: 0 }, 10, now, None, &LIMITS), Err(DeferralReason::NotAtRest));
        assert_eq!(rest_gate(FileTime { secs: 970, nanos: 1 }, 10, now, None, &LIMITS), Err(DeferralReason::NotAtRest));
        assert_eq!(rest_gate(FileTime { secs: 970, nanos: 0 }, 10, now, None, &LIMITS), Ok(()));
        let unbounded = SweepLimits { max_audio_bytes: 1, quiet_period_secs: u64::MAX };
        assert_eq!(rest_gate(FileTime { secs: 0, nanos: 0 }, 10, now, None, &unbounded), Err(DeferralReason::NotAtRest));
    }

    #[test]
    fn a_size_that_moved_since_the_last_deferral_is_not_at_rest() {
        let now = UtcInstant { secs: 1_000 };
        let seen = SeenFacts { size: 9, mtime: FileTime { secs: 1, nanos: 0 }, ingested: false, deferred_size: Some(9) };
        assert_eq!(rest_gate(FileTime { secs: 1, nanos: 0 }, 10, now, Some(&seen), &LIMITS), Err(DeferralReason::NotAtRest));
        assert_eq!(rest_gate(FileTime { secs: 1, nanos: 0 }, 9, now, Some(&seen), &LIMITS), Ok(()));
    }

    #[test]
    fn reasons_round_trip_through_their_names() {
        for reason in [
            DeferralReason::Dataless,
            DeferralReason::AudioTooLarge,
            DeferralReason::InvalidContainer,
            DeferralReason::NotAtRest,
            DeferralReason::ChangedDuringRead,
        ] {
            assert_eq!(DeferralReason::parse(reason.as_str()), Some(reason));
        }
        assert_eq!(DeferralReason::parse("busy"), None);
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-domain sweep`

Expected: the build fails with `cannot find` for `pre_open`, `size_gate`, `rest_gate`, `CandidateFacts`,
`SeenFacts`, `SweepLimits`, `PreOpen`, `DeferralReason` and `SF_DATALESS`.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-domain/src/sweep.rs`, above its test module:

```rust
//! The sweep gates of spec section 5.2, cheapest first, as pure decisions.

use crate::time::{FileTime, UtcInstant};

/// The `st_flags` bit macOS sets on a file whose bytes are still in iCloud.
pub const SF_DATALESS: u32 = 0x4000_0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeferralReason {
    Dataless,
    AudioTooLarge,
    InvalidContainer,
    NotAtRest,
    ChangedDuringRead,
}

impl DeferralReason {
    pub fn as_str(self) -> &'static str {
        match self {
            DeferralReason::Dataless => "dataless",
            DeferralReason::AudioTooLarge => "audio_too_large",
            DeferralReason::InvalidContainer => "invalid_container",
            DeferralReason::NotAtRest => "not_at_rest",
            DeferralReason::ChangedDuringRead => "changed_during_read",
        }
    }

    pub fn parse(text: &str) -> Option<DeferralReason> {
        [
            DeferralReason::Dataless,
            DeferralReason::AudioTooLarge,
            DeferralReason::InvalidContainer,
            DeferralReason::NotAtRest,
            DeferralReason::ChangedDuringRead,
        ]
        .into_iter()
        .find(|reason| reason.as_str() == text)
    }
}

pub struct CandidateFacts {
    pub size: u64,
    pub mtime: FileTime,
    pub flags: u32,
}

impl CandidateFacts {
    pub fn dataless(&self) -> bool {
        self.flags & SF_DATALESS != 0
    }
}

pub struct SeenFacts {
    pub size: u64,
    pub mtime: FileTime,
    pub ingested: bool,
    pub deferred_size: Option<u64>,
}

pub struct SweepLimits {
    pub max_audio_bytes: u64,
    pub quiet_period_secs: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PreOpen {
    Dataless,
    Unchanged,
    Open,
}

pub fn pre_open(candidate: &CandidateFacts, seen: Option<&SeenFacts>) -> PreOpen {
    if candidate.dataless() {
        return PreOpen::Dataless;
    }
    match seen {
        Some(seen) if seen.ingested && seen.size == candidate.size && seen.mtime == candidate.mtime => PreOpen::Unchanged,
        _ => PreOpen::Open,
    }
}

pub fn size_gate(size: u64, limits: &SweepLimits) -> Result<(), DeferralReason> {
    if size > limits.max_audio_bytes { Err(DeferralReason::AudioTooLarge) } else { Ok(()) }
}

pub fn rest_gate(
    mtime: FileTime,
    size: u64,
    now: UtcInstant,
    seen: Option<&SeenFacts>,
    limits: &SweepLimits,
) -> Result<(), DeferralReason> {
    if mtime.age_secs(now) < i64::try_from(limits.quiet_period_secs).unwrap_or(i64::MAX) {
        return Err(DeferralReason::NotAtRest);
    }
    if let Some(deferred) = seen.and_then(|seen| seen.deferred_size)
        && deferred != size
    {
        return Err(DeferralReason::NotAtRest);
    }
    Ok(())
}
```

`FileTime::age_secs` counts the nanoseconds (Task 7), so a file touched 29.999999999 seconds ago is not
at rest under a 30 second quiet period.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo fmt --all`

Run: `cargo test -p vpt-domain sweep`

Expected: 6 tests PASS. Run `cargo clippy -p vpt-domain --all-targets -- -D warnings` and expect no
warnings.

- [ ] **Step 5: Commit**

Run: `cargo fmt --all`

```bash
git add crates/vpt-domain
SKIP_AI_COMMIT=1 git commit -m "feat(domain): the sweep gates and deferral reasons"
```

______________________________________________________________________

### Task 11: The SQLite ledger: open, permissions, WAL and the versioned schema

The schema created here is the whole ledger of spec section 4.4, not stage 1's slice of it: `seen`,
`recordings`, `transcripts`, `proposals`, `flags`, `occasions`, `tags`, `relations`, `releases`,
`dirty_publications` and `retention_intents`. Stages 2 to 4 add rows and repositories over these tables;
they add columns through a migration, never by editing version 1. Stage 1 implements repositories over
`seen`, `recordings`, `dirty_publications` and `retention_intents` only.

Two ways in. A mutating command has created the private state directory and holds its lock (Task 27), so
it opens the ledger writable through the state root's descriptor: the database file is created
exclusively at mode 0600 when absent, repaired to 0600 when present, refused when a link or a non-file
stands at its name, switched to WAL and migrated. An observational command or a dry run opens read-only:
no directory, file, mode, journal setting or schema is created or changed, and an absent state directory
or database, or a database file with no schema yet, is reported as `None` so the composition root
substitutes the empty memory ledger of Task 12.

**Files:**

- Create: `crates/vpt-application/src/ports/ledger.rs` (the error type only in this task)
- Modify: `crates/vpt-application/src/ports/mod.rs`
- Create: `crates/vpt-adapters/src/ledger/mod.rs`, `crates/vpt-adapters/src/ledger/sqlite/mod.rs`,
  `crates/vpt-adapters/src/ledger/sqlite/migrations.rs`,
  `crates/vpt-adapters/src/ledger/sqlite/connection.rs`,
  `crates/vpt-adapters/src/ledger/sqlite/boundary_tests.rs`
- Modify: `crates/vpt-adapters/src/lib.rs`

**Interfaces:**

- Consumes: `vpt_adapters::{Access, ContainedError, Kind, RootDir}`.

- Produces: `vpt_application::ports::LedgerError::{Busy, UnsupportedSchema(u32), Conflict(String),`
  `Corrupt(String), Io(String), PathEscape(PathBuf)}`;
  `vpt_adapters::{SqliteLedger, OpenError, SCHEMA_VERSION}` (`ledger/mod.rs` keeps `sqlite` private and
  re-exports these) with `OpenError::{Contained(ContainedError), Ledger(LedgerError)}` (`From` both ways
  in), `SqliteLedger::BUSY_TIMEOUT: Duration = Duration::from_secs(5)`,
  `SqliteLedger::open(state: &RootDir) -> Result<SqliteLedger, OpenError>`,
  `SqliteLedger::open_with_timeout(state: &RootDir, busy: Duration) -> Result<SqliteLedger, OpenError>`,
  `SqliteLedger::open_read_only(state_dir: &Path) -> Result<Option<SqliteLedger>, OpenError>`,
  `fn database_path(&self) -> &Path`, `fn schema_version(&self) -> Result<u32, LedgerError>`,
  `pub(crate) fn transaction<T>(&self, op: impl FnOnce(&rusqlite::Transaction<'_>) ->`
  `Result<T, LedgerError>) -> Result<T, LedgerError>` (an `Io` error on a read-only ledger),
  `pub(crate) fn read<T>(&self, op: impl FnOnce(&rusqlite::Connection) -> Result<T,`
  `LedgerError>) -> Result<T, LedgerError>`; `SCHEMA_VERSION: u32 = 1`;
  `pub(crate) fn map(error: rusqlite::Error) -> LedgerError` in `ledger::sqlite`.

- [ ] **Step 1: Write the failing tests**

Declare the modules first. `crates/vpt-application/src/ports/mod.rs` gains `mod ledger;` and
`pub use ledger::LedgerError;`. `crates/vpt-adapters/src/lib.rs` gains `mod ledger;` and
`pub use ledger::{OpenError, SCHEMA_VERSION, SqliteLedger};`; `crates/vpt-adapters/src/ledger/mod.rs` is:

```rust
//! The ledger: one SQLite type implementing every repository, and an
//! in-memory twin that runs the same contract.

mod sqlite;

pub use sqlite::{OpenError, SCHEMA_VERSION, SqliteLedger};
```

`crates/vpt-adapters/src/ledger/sqlite/mod.rs` starts as `mod migrations;` plus its test module, and
`migrations.rs` starts empty:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};

    fn state() -> (tempfile::TempDir, RootDir) {
        let temp = tempfile::tempdir().expect("temp");
        let dir = temp.path().canonicalize().expect("canonical").join("state");
        std::fs::DirBuilder::new().mode(0o700).create(&dir).expect("state dir");
        let root = RootDir::open(&dir).expect("root");
        (temp, root)
    }

    fn journal_mode(ledger: &SqliteLedger) -> String {
        ledger.read(|c| c.query_row("PRAGMA journal_mode", [], |row| row.get(0)).map_err(map)).expect("mode")
    }

    #[test]
    fn open_creates_a_0600_database_at_schema_version_1_in_wal_mode() {
        let (_temp, state) = state();
        let ledger = SqliteLedger::open(&state).expect("opens");
        assert_eq!(ledger.database_path(), state.path().join("vpt.db"));
        assert_eq!(std::fs::metadata(ledger.database_path()).expect("db").permissions().mode() & 0o777, 0o600);
        assert_eq!(ledger.schema_version().expect("version"), 1);
        assert_eq!(journal_mode(&ledger), "wal");
    }

    #[test]
    fn opening_twice_is_idempotent_repairs_the_mode_and_every_table_exists() {
        let (_temp, state) = state();
        SqliteLedger::open(&state).expect("first");
        std::fs::set_permissions(state.path().join("vpt.db"), std::fs::Permissions::from_mode(0o644)).expect("loosen");
        let ledger = SqliteLedger::open(&state).expect("second");
        assert_eq!(std::fs::metadata(ledger.database_path()).expect("db").permissions().mode() & 0o777, 0o600);
        let names: Vec<String> = ledger
            .read(|c| {
                let mut statement = c.prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name").map_err(map)?;
                let rows = statement.query_map([], |row| row.get(0)).map_err(map)?;
                rows.collect::<Result<_, _>>().map_err(map)
            })
            .expect("names");
        for table in [
            "seen", "recordings", "transcripts", "proposals", "flags", "occasions", "tags", "relations", "releases",
            "dirty_publications", "retention_intents",
        ] {
            assert!(names.iter().any(|name| name == table), "{table} missing from {names:?}");
        }
    }

    #[test]
    fn a_future_schema_version_is_refused_with_its_number() {
        let (_temp, state) = state();
        let ledger = SqliteLedger::open(&state).expect("opens");
        ledger.read(|c| c.pragma_update(None, "user_version", 99).map_err(map)).expect("bump");
        assert_eq!(SqliteLedger::open(&state).unwrap_err(), OpenError::Ledger(LedgerError::UnsupportedSchema(99)));
        assert_eq!(
            SqliteLedger::open_read_only(state.path()).unwrap_err(),
            OpenError::Ledger(LedgerError::UnsupportedSchema(99))
        );
    }

    #[test]
    fn a_link_in_place_of_the_database_is_refused_before_anything_is_opened() {
        let (temp, state) = state();
        let elsewhere = temp.path().join("elsewhere.db");
        std::fs::write(&elsewhere, b"").expect("elsewhere");
        std::os::unix::fs::symlink(&elsewhere, state.path().join("vpt.db")).expect("link");
        let refused = ContainedError::NotRegular(state.path().join("vpt.db"));
        assert_eq!(SqliteLedger::open(&state).unwrap_err(), OpenError::Contained(refused.clone()));
        assert_eq!(SqliteLedger::open_read_only(state.path()).unwrap_err(), OpenError::Contained(refused));
        assert_eq!(std::fs::metadata(&elsewhere).expect("target").len(), 0);
    }

    #[test]
    fn a_held_write_lock_surfaces_as_busy_after_the_bounded_wait() {
        let (_temp, state) = state();
        let ledger = SqliteLedger::open_with_timeout(&state, Duration::from_millis(50)).expect("opens");
        let mut holder = rusqlite::Connection::open(ledger.database_path()).expect("second connection");
        let held = holder.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).expect("held");

        let outcome = ledger.transaction(|t| t.execute("DELETE FROM seen", []).map(|_| ()).map_err(map));

        assert_eq!(outcome.unwrap_err(), LedgerError::Busy);
        drop(held);
    }

    #[test]
    fn read_only_open_reports_none_and_creates_nothing_when_state_or_database_is_absent() {
        let temp = tempfile::tempdir().expect("temp");
        let absent = temp.path().canonicalize().expect("canonical").join("state");
        assert!(SqliteLedger::open_read_only(&absent).expect("absent dir").is_none());
        assert!(!absent.exists());
        let (_temp, state) = state();
        assert!(SqliteLedger::open_read_only(state.path()).expect("absent db").is_none());
        assert_eq!(state.names().expect("names"), Vec::<String>::new());
        std::fs::write(state.path().join("vpt.db"), b"").expect("empty file");
        assert!(SqliteLedger::open_read_only(state.path()).expect("no schema").is_none());
        assert_eq!(std::fs::metadata(state.path().join("vpt.db")).expect("db").len(), 0);
    }

    #[test]
    fn read_only_open_reads_an_existing_ledger_and_refuses_to_write() {
        let (_temp, state) = state();
        SqliteLedger::open(&state).expect("create");
        std::fs::set_permissions(state.path().join("vpt.db"), std::fs::Permissions::from_mode(0o644)).expect("loosen");
        let ledger = SqliteLedger::open_read_only(state.path()).expect("opens").expect("present");
        assert_eq!(ledger.schema_version().expect("version"), 1);
        assert_eq!(std::fs::metadata(ledger.database_path()).expect("db").permissions().mode() & 0o777, 0o644);
        let outcome = ledger.transaction(|t| t.execute("DELETE FROM seen", []).map(|_| ()).map_err(map));
        assert!(matches!(outcome, Err(LedgerError::Io(_))), "{outcome:?}");
    }
}
```

Add `#[cfg(test)] mod boundary_tests;` at the end of `ledger/sqlite/mod.rs` before the red run.
`crates/vpt-adapters/src/ledger/sqlite/boundary_tests.rs`:

```rust
use super::*;
use std::os::unix::fs::{MetadataExt, PermissionsExt};

fn fixture() -> (tempfile::TempDir, RootDir) {
    let temp = tempfile::tempdir().expect("temp");
    let path = temp.path().canonicalize().expect("canonical").join("state");
    std::fs::create_dir(&path).expect("state");
    let root = RootDir::open(&path).expect("root");
    (temp, root)
}

#[derive(Debug, PartialEq, Eq)]
struct SnapshotRow {
    path: PathBuf,
    mode: u32,
    mtime_secs: i64,
    mtime_nanos: i64,
    bytes: Vec<u8>,
}

fn snapshot(root: &RootDir) -> Vec<SnapshotRow> {
    let mut paths = vec![root.path().to_path_buf()];
    paths.extend(
        root.names()
            .expect("names")
            .into_iter()
            .map(|name| root.path().join(name)),
    );
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let metadata = path.symlink_metadata().expect("metadata");
            assert!(!metadata.file_type().is_symlink());
            let bytes = if metadata.is_file() {
                std::fs::read(&path).expect("bytes")
            } else {
                Vec::new()
            };
            SnapshotRow {
                path,
                mode: metadata.permissions().mode() & 0o777,
                mtime_secs: metadata.mtime(),
                mtime_nanos: metadata.mtime_nsec(),
                bytes,
            }
        })
        .collect()
}

#[test]
fn reconnect_refuses_a_replaced_state_root() {
    let (temp, root) = fixture();
    let ledger = SqliteLedger::open(&root).expect("ledger");
    std::fs::rename(root.path(), temp.path().join("held")).expect("move");
    std::fs::create_dir(root.path()).expect("replacement");
    let replacement = root.path().join(DATABASE);
    std::fs::write(&replacement, b"replacement").expect("replacement db");

    assert_eq!(
        ledger.schema_version(),
        Err(LedgerError::PathEscape(root.path().to_path_buf()))
    );
    assert_eq!(
        std::fs::read(replacement).expect("unchanged"),
        b"replacement"
    );
}

#[test]
fn reconnect_refuses_a_database_link_added_after_open() {
    let (temp, root) = fixture();
    let ledger = SqliteLedger::open(&root).expect("ledger");
    let path = root.path().join(DATABASE);
    std::fs::rename(&path, temp.path().join("held.db")).expect("move database");
    let target = temp.path().join("target");
    std::fs::write(&target, b"keep").expect("target");
    std::os::unix::fs::symlink(&target, &path).expect("link");

    assert_eq!(ledger.schema_version(), Err(LedgerError::PathEscape(path)));
    assert_eq!(std::fs::read(target).expect("unchanged"), b"keep");
}

#[test]
fn read_only_queries_preserve_files_while_open_and_after_close() {
    let (_temp, root) = fixture();
    drop(SqliteLedger::open(&root).expect("ledger"));
    let before = snapshot(&root);
    let ledger = SqliteLedger::open_read_only(root.path())
        .expect("read-only")
        .expect("present");
    ledger
        .read(|connection| {
            let version: u32 = connection
                .pragma_query_value(None, "user_version", |row| row.get(0))
                .map_err(map)?;
            assert_eq!(version, 1);
            assert!(snapshot(&root) == before, "the state changed while open");
            Ok(())
        })
        .expect("query");
    drop(ledger);
    assert!(snapshot(&root) == before, "the state changed after close");
}

#[test]
fn read_only_open_refuses_a_missing_wal_without_creating_it() {
    let (temp, root) = fixture();
    drop(SqliteLedger::open(&root).expect("ledger"));
    let wal = root.path().join("vpt.db-wal");
    std::fs::rename(&wal, temp.path().join("held.wal")).expect("move WAL");
    let before = snapshot(&root);

    assert!(matches!(
        SqliteLedger::open_read_only(root.path()),
        Err(OpenError::Ledger(LedgerError::Io(_)))
    ));
    assert_eq!(snapshot(&root), before);
    assert!(!wal.exists());
}

#[test]
fn read_only_queries_see_committed_rows_still_in_a_live_wal() {
    let (_temp, root) = fixture();
    let writer = SqliteLedger::open(&root).expect("ledger");
    let connection = writer.connect().expect("writer connection");
    connection
        .pragma_update(None, "wal_autocheckpoint", 0)
        .expect("disable checkpoint");
    connection
        .execute_batch(
            "CREATE TABLE read_probe(value INTEGER NOT NULL);
             INSERT INTO read_probe(value) VALUES (42);",
        )
        .expect("committed WAL row");
    let reader = SqliteLedger::open_read_only(root.path())
        .expect("read-only")
        .expect("present");
    let value: i64 = reader
        .read(|connection| {
            connection
                .query_row("SELECT value FROM read_probe", [], |row| row.get(0))
                .map_err(map)
        })
        .expect("WAL row");
    assert_eq!(value, 42);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-adapters ledger`

Expected: the build fails with `cannot find` for `SqliteLedger`, `OpenError`, `LedgerError` and `map` in
`ledger::sqlite::tests`, and `unresolved import` for the re-exports in `ledger/mod.rs`. A run that
selects zero tests does not satisfy this step.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-application/src/ports/ledger.rs` (this task's part; Task 12 adds the rows and traits):

```rust
//! The ledger repositories a use case reads and writes.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerError {
    Busy,
    PathEscape(std::path::PathBuf),
    UnsupportedSchema(u32),
    Conflict(String),
    Corrupt(String),
    Io(String),
}
```

`crates/vpt-adapters/src/ledger/sqlite/migrations.rs`:

```rust
//! The versioned schema. Version 1 is the whole ledger of spec section 4.4;
//! a later stage adds columns with a new version, never by editing this one.

use rusqlite::{Connection, TransactionBehavior};
use vpt_application::ports::LedgerError;

pub const VERSION: u32 = 1;

const SCHEMA_V1: &str = "
CREATE TABLE seen (
  path TEXT PRIMARY KEY,
  file_name TEXT NOT NULL,
  size INTEGER NOT NULL,
  mtime_secs INTEGER NOT NULL,
  mtime_nanos INTEGER NOT NULL,
  flags INTEGER NOT NULL,
  first_seen INTEGER NOT NULL,
  last_seen INTEGER NOT NULL,
  deferral_count INTEGER NOT NULL DEFAULT 0,
  deferral_reason TEXT,
  deferred_size INTEGER,
  source_gone_at INTEGER,
  recording TEXT
);
CREATE TABLE recordings (
  id TEXT PRIMARY KEY,
  source_path TEXT,
  digest TEXT NOT NULL UNIQUE,
  captured_at INTEGER NOT NULL,
  captured_offset INTEGER NOT NULL,
  duration_secs INTEGER NOT NULL,
  title TEXT,
  title_source TEXT NOT NULL,
  ingested_at INTEGER NOT NULL,
  audio_path TEXT NOT NULL,
  stage_transcribe TEXT NOT NULL,
  stage_note TEXT NOT NULL,
  stage_synthesis TEXT NOT NULL,
  transcript_note_path TEXT,
  analysis_note_path TEXT,
  engines TEXT,
  language TEXT,
  open_flags INTEGER NOT NULL DEFAULT 0,
  diagnostics INTEGER NOT NULL DEFAULT 0,
  audio_trashed_at INTEGER
);
CREATE TABLE transcripts (
  recording TEXT PRIMARY KEY REFERENCES recordings(id),
  engine TEXT NOT NULL,
  body TEXT NOT NULL,
  accepted_at INTEGER NOT NULL
);
CREATE TABLE proposals (
  recording TEXT PRIMARY KEY REFERENCES recordings(id),
  input_digest TEXT NOT NULL,
  body TEXT NOT NULL,
  accepted_at INTEGER NOT NULL
);
CREATE TABLE flags (
  identifier TEXT PRIMARY KEY,
  recording TEXT NOT NULL REFERENCES recordings(id),
  shape TEXT NOT NULL,
  class TEXT NOT NULL,
  ranges TEXT,
  artifact TEXT,
  claim_index INTEGER,
  claim_digest TEXT,
  record_text TEXT,
  alternative_text TEXT,
  confidence REAL,
  state TEXT NOT NULL,
  resolution_text TEXT,
  resolved_at INTEGER
);
CREATE TABLE occasions (
  id TEXT PRIMARY KEY,
  provider_key TEXT NOT NULL UNIQUE,
  source TEXT NOT NULL,
  at INTEGER NOT NULL,
  duration_secs INTEGER,
  title TEXT NOT NULL,
  participants TEXT NOT NULL,
  tags TEXT NOT NULL,
  pack TEXT,
  brief_path TEXT,
  briefed_at INTEGER
);
CREATE TABLE tags (
  recording TEXT NOT NULL REFERENCES recordings(id),
  tag TEXT NOT NULL,
  state TEXT NOT NULL,
  provenance TEXT NOT NULL,
  PRIMARY KEY (recording, tag)
);
CREATE TABLE relations (
  recording TEXT NOT NULL REFERENCES recordings(id),
  kind TEXT NOT NULL,
  target TEXT NOT NULL,
  state TEXT NOT NULL,
  provenance TEXT NOT NULL,
  rule TEXT,
  ranges TEXT,
  PRIMARY KEY (recording, kind, target)
);
CREATE TABLE releases (
  sequence INTEGER PRIMARY KEY AUTOINCREMENT,
  recording TEXT NOT NULL REFERENCES recordings(id),
  stage TEXT NOT NULL,
  destination TEXT NOT NULL,
  content_digest TEXT NOT NULL,
  source_digest TEXT NOT NULL,
  report_path TEXT NOT NULL,
  snapshot TEXT NOT NULL,
  released_at INTEGER NOT NULL
);
CREATE TABLE dirty_publications (
  target TEXT PRIMARY KEY,
  expected_previous TEXT,
  intended TEXT NOT NULL,
  recorded_at INTEGER NOT NULL
);
CREATE TABLE retention_intents (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  artifact_kind TEXT NOT NULL,
  recording TEXT,
  path TEXT NOT NULL,
  expected TEXT NOT NULL,
  recorded_at INTEGER NOT NULL,
  completed_at INTEGER
);
";

pub fn migrate(connection: &mut Connection) -> Result<(), LedgerError> {
    let version = current(connection)?;
    if version == VERSION {
        return Ok(());
    }
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(super::map)?;
    let version = current(&transaction)?;
    if version == 0 {
        transaction.execute_batch(SCHEMA_V1).map_err(super::map)?;
    }
    transaction.pragma_update(None, "user_version", VERSION).map_err(super::map)?;
    transaction.commit().map_err(super::map)
}

/// The stored version, refused when it is newer than this build understands.
pub fn current(connection: &Connection) -> Result<u32, LedgerError> {
    let version: u32 = connection.pragma_query_value(None, "user_version", |row| row.get(0)).map_err(super::map)?;
    if version > VERSION {
        return Err(LedgerError::UnsupportedSchema(version));
    }
    Ok(version)
}
```

`crates/vpt-adapters/src/ledger/sqlite/mod.rs`, above its test module:

```rust
//! One SQLite database: WAL, a bounded busy timeout, restrictive modes, and
//! versioned migrations at open. The retained state root is checked at each connection.

mod connection;
mod migrations;

use crate::contained::{Access, ContainedError, Kind, RootDir};
use rusqlite::{Connection, ErrorCode, Transaction, TransactionBehavior};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;
use vpt_application::ports::LedgerError;

pub use migrations::VERSION as SCHEMA_VERSION;

const DATABASE: &str = "vpt.db";
const SIDE_FILES: [&str; 2] = ["vpt.db-wal", "vpt.db-shm"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenError {
    Contained(ContainedError),
    Ledger(LedgerError),
}

impl From<ContainedError> for OpenError {
    fn from(error: ContainedError) -> OpenError {
        OpenError::Contained(error)
    }
}

impl From<LedgerError> for OpenError {
    fn from(error: LedgerError) -> OpenError {
        OpenError::Ledger(error)
    }
}

#[derive(Debug)]
pub struct SqliteLedger {
    state: RootDir,
    path: PathBuf,
    busy_timeout: Duration,
    read_only: bool,
}

impl SqliteLedger {
    pub const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

    pub fn open(state: &RootDir) -> Result<SqliteLedger, OpenError> {
        Self::open_with_timeout(state, Self::BUSY_TIMEOUT)
    }

    /// Writable: the file is created at 0600 when absent, repaired to 0600 when
    /// present, refused when a link or a non-file stands at its name, then
    /// switched to WAL and migrated.
    pub fn open_with_timeout(
        state: &RootDir,
        busy_timeout: Duration,
    ) -> Result<SqliteLedger, OpenError> {
        state.revalidate()?;
        let database = match state.open_file(Path::new(DATABASE), Access::ReadWrite) {
            Ok(file) => file,
            Err(ContainedError::Io {
                kind: std::io::ErrorKind::NotFound,
                ..
            }) => state.create_file(Path::new(DATABASE), 0o600)?,
            Err(error) => return Err(error.into()),
        };
        database
            .set_permissions(std::fs::Permissions::from_mode(0o600))
            .map_err(|error| LedgerError::Io(error.to_string()))?;
        drop(database);
        no_link_beside(state)?;
        let ledger = SqliteLedger {
            state: state.try_clone()?,
            path: state.path().join(DATABASE),
            busy_timeout,
            read_only: false,
        };
        let mut connection = ledger.connect()?;
        migrations::migrate(&mut connection)?;
        ledger.state.revalidate()?;
        Ok(ledger)
    }

    /// Read-only: nothing is created, changed or migrated. `None` when the state
    /// directory or the database is absent, or the file holds no schema yet.
    pub fn open_read_only(state_dir: &Path) -> Result<Option<SqliteLedger>, OpenError> {
        let state = match RootDir::open(state_dir) {
            Ok(state) => state,
            Err(ContainedError::Io {
                kind: std::io::ErrorKind::NotFound,
                ..
            }) => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let path = match state.regular(Path::new(DATABASE)) {
            Ok(path) => path,
            Err(ContainedError::Io {
                kind: std::io::ErrorKind::NotFound,
                ..
            }) => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        no_link_beside(&state)?;
        if state.stat(Path::new(DATABASE))?.size == 0 {
            return Ok(None);
        }
        let ledger = SqliteLedger {
            state,
            path,
            busy_timeout: Self::BUSY_TIMEOUT,
            read_only: true,
        };
        match ledger.read(migrations::current)? {
            0 => Ok(None),
            _ => Ok(Some(ledger)),
        }
    }

    pub fn database_path(&self) -> &Path {
        &self.path
    }

    pub fn schema_version(&self) -> Result<u32, LedgerError> {
        self.read(|c| {
            c.pragma_query_value(None, "user_version", |row| row.get(0))
                .map_err(map)
        })
    }

    fn connect(&self) -> Result<Connection, LedgerError> {
        connection::open(&self.state, self.busy_timeout, self.read_only)
    }

    pub(crate) fn transaction<T>(
        &self,
        operation: impl FnOnce(&Transaction<'_>) -> Result<T, LedgerError>,
    ) -> Result<T, LedgerError> {
        if self.read_only {
            return Err(LedgerError::Io("the ledger is read-only".into()));
        }
        let mut connection = self.connect()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(map)?;
        let value = operation(&transaction)?;
        connection::check(&self.state, false)?;
        transaction.commit().map_err(map)?;
        Ok(value)
    }

    pub(crate) fn read<T>(
        &self,
        operation: impl FnOnce(&Connection) -> Result<T, LedgerError>,
    ) -> Result<T, LedgerError> {
        let connection = self.connect()?;
        let value = operation(&connection)?;
        connection::check(&self.state, self.read_only)?;
        Ok(value)
    }
}

/// SQLite opens the journal and shared-memory files by name beside the
/// database; a link standing at either name is refused first.
fn no_link_beside(state: &RootDir) -> Result<(), ContainedError> {
    for name in SIDE_FILES {
        match state.stat(Path::new(name)) {
            Ok(stat) if stat.kind != Kind::File => {
                return Err(ContainedError::NotRegular(state.path().join(name)));
            }
            Ok(_)
            | Err(ContainedError::Io {
                kind: std::io::ErrorKind::NotFound,
                ..
            }) => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

pub(crate) fn map(error: rusqlite::Error) -> LedgerError {
    match &error {
        rusqlite::Error::SqliteFailure(failure, _) if failure.code == ErrorCode::DatabaseBusy => {
            LedgerError::Busy
        }
        rusqlite::Error::SqliteFailure(failure, _)
            if failure.code == ErrorCode::ConstraintViolation =>
        {
            LedgerError::Conflict(error.to_string())
        }
        rusqlite::Error::SqliteFailure(failure, _)
            if failure.code == ErrorCode::DatabaseCorrupt
                || failure.code == ErrorCode::NotADatabase =>
        {
            LedgerError::Corrupt(error.to_string())
        }
        _ => LedgerError::Io(error.to_string()),
    }
}
```

`crates/vpt-adapters/src/ledger/sqlite/connection.rs`:

```rust
use super::{DATABASE, SIDE_FILES, map, no_link_beside};
use crate::contained::{ContainedError, RootDir};
use rusqlite::{Connection, OpenFlags};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::time::Duration;
use vpt_application::ports::LedgerError;

pub(super) fn open(
    state: &RootDir,
    busy_timeout: Duration,
    read_only: bool,
) -> Result<Connection, LedgerError> {
    check(state, read_only)?;
    let path = state.path().join(DATABASE);
    let common = OpenFlags::SQLITE_OPEN_NO_MUTEX | OpenFlags::SQLITE_OPEN_NOFOLLOW;
    let opened = if read_only {
        Connection::open_with_flags(
            read_only_uri(&path),
            common | OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
        )
    } else {
        Connection::open_with_flags(&path, common | OpenFlags::SQLITE_OPEN_READ_WRITE)
    };
    let connection = match opened {
        Ok(connection) => connection,
        Err(error) => {
            check(state, read_only)?;
            return Err(open_error(error, &path));
        }
    };
    check(state, read_only)?;
    persist_wal(&connection)?;
    connection.busy_timeout(busy_timeout).map_err(map)?;
    if !read_only {
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .map_err(map)?;
    }
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .map_err(map)?;
    check(state, read_only)?;
    Ok(connection)
}

pub(super) fn check(state: &RootDir, require_sidecars: bool) -> Result<(), LedgerError> {
    state.revalidate().map_err(contained)?;
    state.regular(Path::new(DATABASE)).map_err(contained)?;
    no_link_beside(state).map_err(contained)?;
    if require_sidecars {
        for name in SIDE_FILES {
            match state.regular(Path::new(name)) {
                Ok(_) => {}
                Err(ContainedError::Io {
                    kind: std::io::ErrorKind::NotFound,
                    ..
                }) => {
                    return Err(LedgerError::Io(format!(
                        "read-only ledger requires existing {name}"
                    )));
                }
                Err(error) => return Err(contained(error)),
            }
        }
    }
    Ok(())
}

fn persist_wal(connection: &Connection) -> Result<(), LedgerError> {
    let mut enabled: std::ffi::c_int = 1;
    // SAFETY: the connection and integer outlive this synchronous call.
    let result = unsafe {
        rusqlite::ffi::sqlite3_file_control(
            connection.handle(),
            c"main".as_ptr(),
            rusqlite::ffi::SQLITE_FCNTL_PERSIST_WAL,
            (&mut enabled as *mut std::ffi::c_int).cast(),
        )
    };
    if result == rusqlite::ffi::SQLITE_OK {
        Ok(())
    } else {
        Err(LedgerError::Io(format!(
            "could not preserve SQLite sidecars: code {result}"
        )))
    }
}

fn read_only_uri(path: &Path) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut uri = String::from("file:");
    for &byte in path.as_os_str().as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'.' | b'_' | b'~') {
            uri.push(char::from(byte));
        } else {
            uri.push('%');
            uri.push(char::from(HEX[usize::from(byte >> 4)]));
            uri.push(char::from(HEX[usize::from(byte & 15)]));
        }
    }
    uri.push_str("?mode=ro&readonly_shm=1");
    uri
}

fn contained(error: ContainedError) -> LedgerError {
    match error {
        ContainedError::Escape { path, .. }
        | ContainedError::NotRegular(path)
        | ContainedError::NotADirectory(path) => LedgerError::PathEscape(path),
        ContainedError::Io { path, kind } => {
            LedgerError::Io(format!("{kind} at {}", path.display()))
        }
    }
}

fn open_error(error: rusqlite::Error, path: &Path) -> LedgerError {
    match &error {
        rusqlite::Error::SqliteFailure(failure, _)
            if failure.extended_code == rusqlite::ffi::SQLITE_CANTOPEN_SYMLINK =>
        {
            LedgerError::PathEscape(path.to_path_buf())
        }
        _ => map(error),
    }
}
```

Every connection revalidates the retained root and all database leaves. Writable connections preserve the
write-ahead log and shared-memory sidecars; read-only connections require those regular files and use
`readonly_shm=1`, so observation creates or changes no durable file. Missing sidecars fail without
repair. A zero-length database returns `None` before SQLite opens it.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo fmt --all`

Run: `cargo test -p vpt-adapters ledger`

Expected: 12 tests PASS. The busy test finishes in well under a second: the wait is 50 ms. Run
`cargo clippy -p vpt-adapters --all-targets -- -D warnings` and expect no warnings.

- [ ] **Step 5: Commit**

Run: `cargo fmt --all`

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(ledger): SQLite open through the state root, WAL, the version 1 schema"
```

______________________________________________________________________

### Task 12: The recording ledger, the publication journal and the in-memory twin, under one contract

The ledger's write protocol is one operation: a `LedgerCommit` batch of recordings, seen rows and dirty
publication entries lands in one SQLite transaction or not at all, and the memory twin applies the same
batch atomically. That is what spec section 4.4 requires of a publication: the state a rendering depends
on and the journal entry that makes its target dirty are committed together, never as two calls. The
contract tests run the same scenarios against both implementations, including a batch whose last write
violates a uniqueness constraint after the seen row and the journal entry were written, and require both
of those to roll back.

**Files:**

- Modify: `crates/vpt-application/src/ports/ledger.rs`, `crates/vpt-application/src/ports/mod.rs`
- Create: `crates/vpt-adapters/src/ledger/sqlite/recordings.rs`,
  `crates/vpt-adapters/src/ledger/sqlite/journal.rs`, `crates/vpt-adapters/src/ledger/memory.rs`,
  `crates/vpt-adapters/src/ledger/contract.rs`
- Modify: `crates/vpt-adapters/src/ledger/mod.rs`, `crates/vpt-adapters/src/ledger/sqlite/mod.rs`

**Interfaces:**

- Consumes: `SqliteLedger::{transaction, read}`, `map`; domain `RecordingId`, `Sha256Digest`,
  `UtcInstant`, `UtcOffset`, `FileTime`, `DeferralReason`.

- Produces, in `vpt_application::ports` (the file `ports/ledger.rs` stays private, `ports/mod.rs`
  re-exports every name below):

  - `SeenRow { pub path: PathBuf, pub file_name: String, pub size: u64, pub mtime: FileTime,`
    `pub flags: u32, pub first_seen: UtcInstant, pub last_seen: UtcInstant,`
    `pub deferral_count: u32, pub deferral_reason: Option<DeferralReason>,`
    `pub deferred_size: Option<u64>, pub source_gone_at: Option<UtcInstant>,`
    `pub recording: Option<RecordingId> }` (`flags` is the source's whole `st_flags` word).
  - `StageState::{Pending, Succeeded, Failed, Disabled, Expired}` with `as_str(self) -> &'static str`,
    `parse(text: &str) -> Option<StageState>`;
    `StageStates { pub transcribe: StageState, pub note: StageState, pub synthesis: StageState }` with
    `StageStates::fresh() -> StageStates` (all `Pending`).
  - `TitleOrigin::{VoiceMemos, Unavailable}` with `as_str(self) -> &'static str`,
    `parse(text: &str) -> Option<TitleOrigin>` (named apart from the recorder port's `TitleLookup` of
    Task 15).
  - `RecordingRecord { pub id: RecordingId, pub source_path: Option<PathBuf>,`
    `pub digest: Sha256Digest, pub captured_at: UtcInstant, pub captured_offset: UtcOffset,`
    `pub duration_secs: u64, pub title: Option<String>, pub title_source: TitleOrigin,`
    `pub ingested_at: UtcInstant, pub audio_path: PathBuf, pub stages: StageStates,`
    `pub audio_trashed_at: Option<UtcInstant> }`
  - `DirtyPublication { pub target: PathBuf, pub expected_previous: Option<Sha256Digest>,`
    `pub intended: Sha256Digest, pub recorded_at: UtcInstant }`
  - `LedgerCommit { pub recordings: Vec<RecordingRecord>, pub seen: Vec<SeenRow>,`
    `pub publications: Vec<DirtyPublication> }` (`Default`, `Clone`, `Debug`, `PartialEq`).
  - `trait RecordingLedger { fn seen(&self, path: &Path) -> Result<Option<SeenRow>,`
    `LedgerError>; fn seen_all(&self) -> Result<Vec<SeenRow>, LedgerError>;`
    `fn record_seen(&self, row: &SeenRow) -> Result<(), LedgerError>; fn by_digest(&self,`
    `digest: &Sha256Digest) -> Result<Option<RecordingRecord>, LedgerError>; fn by_id(&self,`
    `id: &RecordingId) -> Result<Option<RecordingRecord>, LedgerError>;`
    `fn recordings(&self) -> Result<Vec<RecordingRecord>, LedgerError>;`
    `fn commit(&self, batch: &LedgerCommit) -> Result<(), LedgerError>;`
    `fn set_source_path(&self, id: &RecordingId, path: &Path) -> Result<(), LedgerError>;`
    `fn set_audio_trashed(&self, id: &RecordingId, at: UtcInstant) -> Result<(), LedgerError>; }`.
    `seen_all` is ordered by path, `recordings` by capture instant then identity; `record_seen` keeps the
    stored `first_seen`; `commit` writes every row or none.
  - `trait PublicationJournal { fn pending_publications(&self) -> Result<Vec<DirtyPublication>,`
    `LedgerError>; fn clear_publication(&self, target: &Path) -> Result<(), LedgerError>; }`, pending
    entries ordered by instant then target; an entry is recorded only through `commit`.

- `vpt_adapters::MemoryLedger::new() -> MemoryLedger` (implements both traits, state behind one
  `std::sync::Mutex`; `ledger/mod.rs` keeps `memory` private and re-exports the type).

- `crates/vpt-adapters/src/ledger/contract.rs` (test-only): `pub(crate) mod recording` and
  `pub(crate) mod journal` with one generic function per scenario, and the macros
  `recording_ledger_contract!(make)` and `publication_journal_contract!(make)` that emit one `#[test]`
  per scenario; `make` returns `(guard, ledger)` where the guard keeps a temporary directory alive for
  SQLite and is `()` for memory.

- [ ] **Step 1: Write the failing tests**

Declare the modules first. `crates/vpt-adapters/src/ledger/mod.rs` becomes:

```rust
//! The ledger: one SQLite type implementing every repository, and an
//! in-memory twin that runs the same contract.

mod memory;
mod sqlite;

pub use memory::MemoryLedger;
pub use sqlite::{OpenError, SCHEMA_VERSION, SqliteLedger};

#[cfg(test)]
pub(crate) mod contract;
```

`crates/vpt-adapters/src/lib.rs` adds `pub use ledger::MemoryLedger;`.

`crates/vpt-adapters/src/ledger/sqlite/mod.rs` gains `mod journal;` and `mod recordings;` beside
`mod migrations;`, both files empty for now. `crates/vpt-adapters/src/ledger/contract.rs`:

```rust
//! The behavioral contract every ledger implementation runs.

use std::path::{Path, PathBuf};
use vpt_application::ports::{
    DirtyPublication, LedgerCommit, LedgerError, PublicationJournal, RecordingLedger, RecordingRecord, SeenRow,
    StageStates, TitleOrigin,
};
use vpt_domain::digest::Sha256Digest;
use vpt_domain::identity::RecordingId;
use vpt_domain::sweep::DeferralReason;
use vpt_domain::time::{FileTime, UtcInstant, UtcOffset};

pub fn digest(seed: u8) -> Sha256Digest {
    let mut bytes = [seed; 32];
    bytes[0] = 0x4f;
    Sha256Digest(bytes)
}

pub fn seen_row(path: &str) -> SeenRow {
    SeenRow {
        path: PathBuf::from(path),
        file_name: Path::new(path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        size: 1_024,
        mtime: FileTime { secs: 1_787_604_456, nanos: 17 },
        flags: 0,
        first_seen: UtcInstant { secs: 1_787_700_000 },
        last_seen: UtcInstant { secs: 1_787_700_000 },
        deferral_count: 0,
        deferral_reason: None,
        deferred_size: None,
        source_gone_at: None,
        recording: None,
    }
}

pub fn record(seed: u8) -> RecordingRecord {
    let digest = digest(seed);
    let captured_at = UtcInstant { secs: 1_787_604_456 + i64::from(seed) };
    let captured_offset = UtcOffset { secs: -21_600 };
    RecordingRecord {
        id: RecordingId::derive(captured_at, captured_offset, &digest).expect("representable"),
        source_path: Some(PathBuf::from(format!("/vm/Recordings/{seed}.m4a"))),
        digest,
        captured_at,
        captured_offset,
        duration_secs: 612,
        title: Some("Invoice call".into()),
        title_source: TitleOrigin::VoiceMemos,
        ingested_at: UtcInstant { secs: 1_787_700_000 },
        audio_path: PathBuf::from(format!("/h/audio/{seed}.m4a")),
        stages: StageStates::fresh(),
        audio_trashed_at: None,
    }
}

pub fn publication(target: &str, seed: u8, at: i64) -> DirtyPublication {
    DirtyPublication {
        target: PathBuf::from(target),
        expected_previous: None,
        intended: digest(seed),
        recorded_at: UtcInstant { secs: at },
    }
}

pub fn ingest(recording: RecordingRecord, seen: SeenRow) -> LedgerCommit {
    LedgerCommit { recordings: vec![recording], seen: vec![seen], publications: vec![] }
}

pub mod recording {
    use super::*;

    pub fn seen_is_absent_until_recorded_and_updated_by_a_second_record<L: RecordingLedger>(ledger: &L) {
        assert_eq!(ledger.seen(Path::new("/vm/Recordings/a.m4a")).expect("read"), None);
        let mut row = seen_row("/vm/Recordings/a.m4a");
        row.flags = 0x0000_8000;
        ledger.record_seen(&row).expect("record");
        assert_eq!(ledger.seen(&row.path).expect("read"), Some(row.clone()));
        row.deferral_count = 1;
        row.deferral_reason = Some(DeferralReason::NotAtRest);
        row.deferred_size = Some(1_024);
        ledger.record_seen(&row).expect("update");
        assert_eq!(ledger.seen(&row.path).expect("read"), Some(row));
        assert_eq!(ledger.seen_all().expect("all").len(), 1);
    }

    pub fn a_second_record_keeps_the_first_seen_instant_and_moves_last_seen<L: RecordingLedger>(ledger: &L) {
        let mut row = seen_row("/vm/Recordings/a.m4a");
        ledger.record_seen(&row).expect("first");
        row.first_seen = UtcInstant { secs: 1_787_800_000 };
        row.last_seen = UtcInstant { secs: 1_787_800_000 };
        ledger.record_seen(&row).expect("second");
        let stored = ledger.seen(&row.path).expect("read").expect("present");
        assert_eq!(stored.first_seen, UtcInstant { secs: 1_787_700_000 });
        assert_eq!(stored.last_seen, UtcInstant { secs: 1_787_800_000 });
    }

    pub fn seen_all_is_ordered_by_path_whatever_the_insertion_order<L: RecordingLedger>(ledger: &L) {
        for name in ["c.m4a", "a.m4a", "b.m4a"] {
            ledger.record_seen(&seen_row(&format!("/vm/Recordings/{name}"))).expect("record");
        }
        let paths: Vec<PathBuf> = ledger.seen_all().expect("all").into_iter().map(|row| row.path).collect();
        assert_eq!(paths, ["a.m4a", "b.m4a", "c.m4a"].map(|name| PathBuf::from(format!("/vm/Recordings/{name}"))));
    }

    pub fn commit_writes_the_recording_its_seen_row_and_its_publication_together<L>(ledger: &L)
    where
        L: RecordingLedger + PublicationJournal,
    {
        let recording = record(1);
        let mut seen = seen_row("/vm/Recordings/1.m4a");
        seen.recording = Some(recording.id.clone());
        let entry = publication("/h/transcripts/1.md", 11, 1);
        let mut batch = ingest(recording.clone(), seen.clone());
        batch.publications.push(entry.clone());
        ledger.commit(&batch).expect("commit");
        assert_eq!(ledger.by_id(&recording.id).expect("read"), Some(recording.clone()));
        assert_eq!(ledger.by_digest(&recording.digest).expect("read"), Some(recording.clone()));
        assert_eq!(ledger.seen(&seen.path).expect("read").and_then(|row| row.recording), Some(recording.id.clone()));
        assert_eq!(ledger.recordings().expect("list"), vec![recording]);
        assert_eq!(ledger.pending_publications().expect("pending"), vec![entry]);
    }

    pub fn a_conflicting_recording_rolls_back_the_rows_written_before_it<L>(ledger: &L)
    where
        L: RecordingLedger + PublicationJournal,
    {
        let first = record(2);
        ledger.commit(&ingest(first, seen_row("/vm/Recordings/2.m4a"))).expect("first");
        let mut second = record(2);
        second.id = RecordingId::parse("2026-08-24T144736-000000000002").expect("id");
        second.source_path = Some(PathBuf::from("/vm/Recordings/2b.m4a"));
        let mut batch = ingest(second, seen_row("/vm/Recordings/2b.m4a"));
        batch.publications.push(publication("/h/transcripts/2b.md", 12, 1));
        let outcome = ledger.commit(&batch);
        assert!(matches!(outcome, Err(LedgerError::Conflict(_))), "{outcome:?}");
        assert_eq!(ledger.seen(Path::new("/vm/Recordings/2b.m4a")).expect("read"), None);
        assert_eq!(ledger.pending_publications().expect("pending"), vec![]);
        assert_eq!(ledger.recordings().expect("list").len(), 1);
    }

    pub fn the_source_path_and_the_audio_trashed_instant_are_updatable<L: RecordingLedger>(ledger: &L) {
        let recording = record(3);
        ledger.commit(&ingest(recording.clone(), seen_row("/vm/Recordings/3.m4a"))).expect("commit");
        ledger.set_source_path(&recording.id, Path::new("/vm/Recordings/renamed.m4a")).expect("path");
        ledger.set_audio_trashed(&recording.id, UtcInstant { secs: 1_787_800_000 }).expect("trashed");
        let stored = ledger.by_id(&recording.id).expect("read").expect("present");
        assert_eq!(stored.source_path, Some(PathBuf::from("/vm/Recordings/renamed.m4a")));
        assert_eq!(stored.audio_trashed_at, Some(UtcInstant { secs: 1_787_800_000 }));
    }

    pub fn an_unknown_identity_reads_as_absent<L: RecordingLedger>(ledger: &L) {
        let id = RecordingId::parse("2026-08-24T144736-ffffffffffff").expect("id");
        assert_eq!(ledger.by_id(&id).expect("read"), None);
        assert_eq!(ledger.by_digest(&digest(9)).expect("read"), None);
    }

    pub fn a_recording_with_no_source_and_no_seen_row_commits_alone<L: RecordingLedger>(ledger: &L) {
        let mut recording = record(4);
        recording.source_path = None;
        let batch = LedgerCommit { recordings: vec![recording.clone()], ..LedgerCommit::default() };
        ledger.commit(&batch).expect("recovered");
        assert_eq!(ledger.by_id(&recording.id).expect("read"), Some(recording));
        assert!(ledger.seen_all().expect("all").is_empty());
    }

    pub fn recordings_are_ordered_by_capture_instant_then_identity<L: RecordingLedger>(ledger: &L) {
        for seed in [7, 5, 6] {
            let batch = LedgerCommit { recordings: vec![record(seed)], ..LedgerCommit::default() };
            ledger.commit(&batch).expect("commit");
        }
        let captured: Vec<i64> = ledger.recordings().expect("list").iter().map(|r| r.captured_at.secs).collect();
        assert_eq!(captured, [5, 6, 7].map(|seed| 1_787_604_456 + seed));
    }
}

pub mod journal {
    use super::*;

    pub fn a_pending_publication_is_listed_until_cleared<L>(ledger: &L)
    where
        L: RecordingLedger + PublicationJournal,
    {
        let entry = publication("/h/transcripts/a.md", 4, 1);
        let batch = LedgerCommit { publications: vec![entry.clone()], ..LedgerCommit::default() };
        ledger.commit(&batch).expect("record");
        assert_eq!(ledger.pending_publications().expect("pending"), vec![entry.clone()]);
        ledger.clear_publication(&entry.target).expect("clear");
        assert_eq!(ledger.pending_publications().expect("pending"), vec![]);
    }

    pub fn recording_the_same_target_twice_keeps_the_latest_entry<L>(ledger: &L)
    where
        L: RecordingLedger + PublicationJournal,
    {
        let mut entry = publication("/h/transcripts/a.md", 6, 1);
        entry.expected_previous = Some(digest(5));
        ledger.commit(&LedgerCommit { publications: vec![entry.clone()], ..LedgerCommit::default() }).expect("first");
        entry.intended = digest(7);
        ledger.commit(&LedgerCommit { publications: vec![entry.clone()], ..LedgerCommit::default() }).expect("second");
        assert_eq!(ledger.pending_publications().expect("pending"), vec![entry]);
    }

    pub fn pending_publications_are_ordered_by_instant_then_target<L>(ledger: &L)
    where
        L: RecordingLedger + PublicationJournal,
    {
        let entries = vec![
            publication("/h/transcripts/z.md", 1, 2),
            publication("/h/transcripts/b.md", 2, 1),
            publication("/h/transcripts/a.md", 3, 1),
        ];
        ledger.commit(&LedgerCommit { publications: entries.clone(), ..LedgerCommit::default() }).expect("commit");
        let targets: Vec<PathBuf> = ledger.pending_publications().expect("pending").into_iter().map(|e| e.target).collect();
        assert_eq!(targets, ["a.md", "b.md", "z.md"].map(|name| PathBuf::from(format!("/h/transcripts/{name}"))));
    }
}

macro_rules! recording_ledger_contract {
    ($make:expr) => {
        mod recording_ledger_contract {
            use super::*;
            use crate::ledger::contract::recording::*;

            #[test]
            fn seen_is_absent_until_recorded_and_updated_by_a_second_record_() {
                let (_guard, ledger) = $make();
                seen_is_absent_until_recorded_and_updated_by_a_second_record(&ledger);
            }
            #[test]
            fn a_second_record_keeps_the_first_seen_instant_and_moves_last_seen_() {
                let (_guard, ledger) = $make();
                a_second_record_keeps_the_first_seen_instant_and_moves_last_seen(&ledger);
            }
            #[test]
            fn seen_all_is_ordered_by_path_whatever_the_insertion_order_() {
                let (_guard, ledger) = $make();
                seen_all_is_ordered_by_path_whatever_the_insertion_order(&ledger);
            }
            #[test]
            fn commit_writes_the_recording_its_seen_row_and_its_publication_together_() {
                let (_guard, ledger) = $make();
                commit_writes_the_recording_its_seen_row_and_its_publication_together(&ledger);
            }
            #[test]
            fn a_conflicting_recording_rolls_back_the_rows_written_before_it_() {
                let (_guard, ledger) = $make();
                a_conflicting_recording_rolls_back_the_rows_written_before_it(&ledger);
            }
            #[test]
            fn the_source_path_and_the_audio_trashed_instant_are_updatable_() {
                let (_guard, ledger) = $make();
                the_source_path_and_the_audio_trashed_instant_are_updatable(&ledger);
            }
            #[test]
            fn an_unknown_identity_reads_as_absent_() {
                let (_guard, ledger) = $make();
                an_unknown_identity_reads_as_absent(&ledger);
            }
            #[test]
            fn a_recording_with_no_source_and_no_seen_row_commits_alone_() {
                let (_guard, ledger) = $make();
                a_recording_with_no_source_and_no_seen_row_commits_alone(&ledger);
            }
            #[test]
            fn recordings_are_ordered_by_capture_instant_then_identity_() {
                let (_guard, ledger) = $make();
                recordings_are_ordered_by_capture_instant_then_identity(&ledger);
            }
        }
    };
}
pub(crate) use recording_ledger_contract;

macro_rules! publication_journal_contract {
    ($make:expr) => {
        mod publication_journal_contract {
            use super::*;
            use crate::ledger::contract::journal::*;

            #[test]
            fn a_pending_publication_is_listed_until_cleared_() {
                let (_guard, ledger) = $make();
                a_pending_publication_is_listed_until_cleared(&ledger);
            }
            #[test]
            fn recording_the_same_target_twice_keeps_the_latest_entry_() {
                let (_guard, ledger) = $make();
                recording_the_same_target_twice_keeps_the_latest_entry(&ledger);
            }
            #[test]
            fn pending_publications_are_ordered_by_instant_then_target_() {
                let (_guard, ledger) = $make();
                pending_publications_are_ordered_by_instant_then_target(&ledger);
            }
        }
    };
}
pub(crate) use publication_journal_contract;
```

At the bottom of the `tests` module of `crates/vpt-adapters/src/ledger/sqlite/mod.rs`, after its seven
tests, add a factory beside them and both invocations:

```rust
    fn open_ledger() -> (tempfile::TempDir, SqliteLedger) {
        let (temp, state) = state();
        let ledger = SqliteLedger::open(&state).expect("opens");
        (temp, ledger)
    }

    crate::ledger::contract::recording_ledger_contract!(open_ledger);
    crate::ledger::contract::publication_journal_contract!(open_ledger);
```

`crates/vpt-adapters/src/ledger/memory.rs` starts as its test module alone:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> ((), MemoryLedger) {
        ((), MemoryLedger::new())
    }

    crate::ledger::contract::recording_ledger_contract!(fresh);
    crate::ledger::contract::publication_journal_contract!(fresh);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-adapters ledger`

Expected: the build fails with `unresolved import` for `SeenRow`, `RecordingRecord`, `RecordingLedger`,
`LedgerCommit`, `DirtyPublication` and `PublicationJournal` in `contract.rs`, and `cannot find`
`MemoryLedger` in `memory.rs`.

- [ ] **Step 3: Write the minimal implementation**

Append to `crates/vpt-application/src/ports/ledger.rs`:

```rust
use std::path::{Path, PathBuf};
use vpt_domain::digest::Sha256Digest;
use vpt_domain::identity::RecordingId;
use vpt_domain::sweep::DeferralReason;
use vpt_domain::time::{FileTime, UtcInstant, UtcOffset};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeenRow {
    pub path: PathBuf,
    pub file_name: String,
    pub size: u64,
    pub mtime: FileTime,
    pub flags: u32,
    pub first_seen: UtcInstant,
    pub last_seen: UtcInstant,
    pub deferral_count: u32,
    pub deferral_reason: Option<DeferralReason>,
    pub deferred_size: Option<u64>,
    pub source_gone_at: Option<UtcInstant>,
    pub recording: Option<RecordingId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageState {
    Pending,
    Succeeded,
    Failed,
    Disabled,
    Expired,
}

impl StageState {
    pub fn as_str(self) -> &'static str {
        match self {
            StageState::Pending => "pending",
            StageState::Succeeded => "succeeded",
            StageState::Failed => "failed",
            StageState::Disabled => "disabled",
            StageState::Expired => "expired",
        }
    }

    pub fn parse(text: &str) -> Option<StageState> {
        [StageState::Pending, StageState::Succeeded, StageState::Failed, StageState::Disabled, StageState::Expired]
            .into_iter()
            .find(|state| state.as_str() == text)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StageStates {
    pub transcribe: StageState,
    pub note: StageState,
    pub synthesis: StageState,
}

impl StageStates {
    pub fn fresh() -> StageStates {
        StageStates { transcribe: StageState::Pending, note: StageState::Pending, synthesis: StageState::Pending }
    }
}

/// Where a title came from; named apart from the recorder port's `TitleLookup`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleOrigin {
    VoiceMemos,
    Unavailable,
}

impl TitleOrigin {
    pub fn as_str(self) -> &'static str {
        match self {
            TitleOrigin::VoiceMemos => "voice_memos",
            TitleOrigin::Unavailable => "unavailable",
        }
    }

    pub fn parse(text: &str) -> Option<TitleOrigin> {
        [TitleOrigin::VoiceMemos, TitleOrigin::Unavailable].into_iter().find(|origin| origin.as_str() == text)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordingRecord {
    pub id: RecordingId,
    pub source_path: Option<PathBuf>,
    pub digest: Sha256Digest,
    pub captured_at: UtcInstant,
    pub captured_offset: UtcOffset,
    pub duration_secs: u64,
    pub title: Option<String>,
    pub title_source: TitleOrigin,
    pub ingested_at: UtcInstant,
    pub audio_path: PathBuf,
    pub stages: StageStates,
    pub audio_trashed_at: Option<UtcInstant>,
}

/// A target whose bytes may not match the ledger until the entry is cleared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirtyPublication {
    pub target: PathBuf,
    pub expected_previous: Option<Sha256Digest>,
    pub intended: Sha256Digest,
    pub recorded_at: UtcInstant,
}

/// Everything one operation writes: all of it lands, or none of it does.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LedgerCommit {
    pub recordings: Vec<RecordingRecord>,
    pub seen: Vec<SeenRow>,
    pub publications: Vec<DirtyPublication>,
}

pub trait RecordingLedger {
    fn seen(&self, path: &Path) -> Result<Option<SeenRow>, LedgerError>;
    /// Ordered by path.
    fn seen_all(&self) -> Result<Vec<SeenRow>, LedgerError>;
    /// Inserts or updates by path; a stored `first_seen` is kept.
    fn record_seen(&self, row: &SeenRow) -> Result<(), LedgerError>;
    fn by_digest(&self, digest: &Sha256Digest) -> Result<Option<RecordingRecord>, LedgerError>;
    fn by_id(&self, id: &RecordingId) -> Result<Option<RecordingRecord>, LedgerError>;
    /// Ordered by capture instant, then identity.
    fn recordings(&self) -> Result<Vec<RecordingRecord>, LedgerError>;
    /// One transaction for the whole batch; a second identity for a known
    /// digest is `Conflict` and nothing in the batch is written.
    fn commit(&self, batch: &LedgerCommit) -> Result<(), LedgerError>;
    fn set_source_path(&self, id: &RecordingId, path: &Path) -> Result<(), LedgerError>;
    fn set_audio_trashed(&self, id: &RecordingId, at: UtcInstant) -> Result<(), LedgerError>;
}

pub trait PublicationJournal {
    /// Ordered by the instant recorded, then target.
    fn pending_publications(&self) -> Result<Vec<DirtyPublication>, LedgerError>;
    fn clear_publication(&self, target: &Path) -> Result<(), LedgerError>;
}
```

`crates/vpt-application/src/ports/mod.rs` now re-exports
`ledger::{DirtyPublication, LedgerCommit, LedgerError, PublicationJournal, RecordingLedger,`
`RecordingRecord, SeenRow, StageState, StageStates, TitleOrigin}`.

`crates/vpt-adapters/src/ledger/sqlite/recordings.rs`:

```rust
//! `RecordingLedger` over SQLite: the `seen` and `recordings` tables and the
//! one-transaction commit. Every stored integer is range-checked on the way
//! out and on the way in.

use super::{SqliteLedger, journal, map};
use rusqlite::{Connection, OptionalExtension, Row, params};
use std::path::{Path, PathBuf};
use vpt_application::ports::{LedgerCommit, LedgerError, RecordingLedger, RecordingRecord, SeenRow, StageState, StageStates, TitleOrigin};
use vpt_domain::digest::Sha256Digest;
use vpt_domain::identity::RecordingId;
use vpt_domain::sweep::DeferralReason;
use vpt_domain::time::{FileTime, UtcInstant, UtcOffset};

const SEEN_COLUMNS: &str = "path, file_name, size, mtime_secs, mtime_nanos, flags, first_seen, last_seen, deferral_count, \
    deferral_reason, deferred_size, source_gone_at, recording";
const RECORDING_COLUMNS: &str = "id, source_path, digest, captured_at, captured_offset, duration_secs, title, \
    title_source, ingested_at, audio_path, stage_transcribe, stage_note, stage_synthesis, audio_trashed_at";

fn corrupt(what: &str) -> LedgerError {
    LedgerError::Corrupt(format!("unreadable {what}"))
}

fn unsigned(value: i64, what: &str) -> Result<u64, LedgerError> {
    u64::try_from(value).map_err(|_| corrupt(what))
}

fn narrow(value: i64, what: &str) -> Result<u32, LedgerError> {
    u32::try_from(value).map_err(|_| corrupt(what))
}

fn stored(value: u64, what: &str) -> Result<i64, LedgerError> {
    i64::try_from(value).map_err(|_| corrupt(what))
}

fn nanos(value: i64) -> Result<u32, LedgerError> {
    match narrow(value, "mtime nanoseconds")? {
        nanos if nanos < 1_000_000_000 => Ok(nanos),
        _ => Err(corrupt("mtime nanoseconds")),
    }
}

fn offset(value: i64) -> Result<UtcOffset, LedgerError> {
    match i32::try_from(value) {
        Ok(secs) if secs.unsigned_abs() < 86_400 => Ok(UtcOffset { secs }),
        _ => Err(corrupt("captured offset")),
    }
}

fn seen_from_row(row: &Row<'_>) -> Result<SeenRow, LedgerError> {
    let reason: Option<String> = row.get(9).map_err(map)?;
    let recording: Option<String> = row.get(12).map_err(map)?;
    Ok(SeenRow {
        path: PathBuf::from(row.get::<_, String>(0).map_err(map)?),
        file_name: row.get(1).map_err(map)?,
        size: unsigned(row.get(2).map_err(map)?, "size")?,
        mtime: FileTime { secs: row.get(3).map_err(map)?, nanos: nanos(row.get(4).map_err(map)?)? },
        flags: narrow(row.get(5).map_err(map)?, "flags")?,
        first_seen: UtcInstant { secs: row.get(6).map_err(map)? },
        last_seen: UtcInstant { secs: row.get(7).map_err(map)? },
        deferral_count: narrow(row.get(8).map_err(map)?, "deferral count")?,
        deferral_reason: reason.map(|text| DeferralReason::parse(&text).ok_or_else(|| corrupt("deferral reason"))).transpose()?,
        deferred_size: row.get::<_, Option<i64>>(10).map_err(map)?.map(|n| unsigned(n, "deferred size")).transpose()?,
        source_gone_at: row.get::<_, Option<i64>>(11).map_err(map)?.map(|secs| UtcInstant { secs }),
        recording: recording.map(|text| RecordingId::parse(&text).map_err(|_| corrupt("recording id"))).transpose()?,
    })
}

fn record_from_row(row: &Row<'_>) -> Result<RecordingRecord, LedgerError> {
    let digest: String = row.get(2).map_err(map)?;
    let stage = |index: usize| -> Result<StageState, LedgerError> {
        StageState::parse(&row.get::<_, String>(index).map_err(map)?).ok_or_else(|| corrupt("stage state"))
    };
    Ok(RecordingRecord {
        id: RecordingId::parse(&row.get::<_, String>(0).map_err(map)?).map_err(|_| corrupt("recording id"))?,
        source_path: row.get::<_, Option<String>>(1).map_err(map)?.map(PathBuf::from),
        digest: Sha256Digest::from_hex(&digest).ok_or_else(|| corrupt("digest"))?,
        captured_at: UtcInstant { secs: row.get(3).map_err(map)? },
        captured_offset: offset(row.get(4).map_err(map)?)?,
        duration_secs: unsigned(row.get(5).map_err(map)?, "duration")?,
        title: row.get(6).map_err(map)?,
        title_source: TitleOrigin::parse(&row.get::<_, String>(7).map_err(map)?).ok_or_else(|| corrupt("title source"))?,
        ingested_at: UtcInstant { secs: row.get(8).map_err(map)? },
        audio_path: PathBuf::from(row.get::<_, String>(9).map_err(map)?),
        stages: StageStates { transcribe: stage(10)?, note: stage(11)?, synthesis: stage(12)? },
        audio_trashed_at: row.get::<_, Option<i64>>(13).map_err(map)?.map(|secs| UtcInstant { secs }),
    })
}

fn upsert_seen(connection: &Connection, row: &SeenRow) -> Result<(), LedgerError> {
    connection
        .execute(
            &format!(
                "INSERT INTO seen ({SEEN_COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13) \
                 ON CONFLICT(path) DO UPDATE SET file_name = excluded.file_name, size = excluded.size, \
                 mtime_secs = excluded.mtime_secs, mtime_nanos = excluded.mtime_nanos, flags = excluded.flags, \
                 last_seen = excluded.last_seen, deferral_count = excluded.deferral_count, \
                 deferral_reason = excluded.deferral_reason, deferred_size = excluded.deferred_size, \
                 source_gone_at = excluded.source_gone_at, recording = excluded.recording"
            ),
            params![
                row.path.to_string_lossy(),
                row.file_name,
                stored(row.size, "size")?,
                row.mtime.secs,
                i64::from(row.mtime.nanos),
                i64::from(row.flags),
                row.first_seen.secs,
                row.last_seen.secs,
                i64::from(row.deferral_count),
                row.deferral_reason.map(DeferralReason::as_str),
                row.deferred_size.map(|n| stored(n, "deferred size")).transpose()?,
                row.source_gone_at.map(|at| at.secs),
                row.recording.as_ref().map(RecordingId::as_str),
            ],
        )
        .map(|_| ())
        .map_err(map)
}

fn insert_recording(connection: &Connection, recording: &RecordingRecord) -> Result<(), LedgerError> {
    connection
        .execute(
            &format!("INSERT INTO recordings ({RECORDING_COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)"),
            params![
                recording.id.as_str(),
                recording.source_path.as_ref().map(|path| path.to_string_lossy().into_owned()),
                recording.digest.hex(),
                recording.captured_at.secs,
                recording.captured_offset.secs,
                stored(recording.duration_secs, "duration")?,
                recording.title,
                recording.title_source.as_str(),
                recording.ingested_at.secs,
                recording.audio_path.to_string_lossy(),
                recording.stages.transcribe.as_str(),
                recording.stages.note.as_str(),
                recording.stages.synthesis.as_str(),
                recording.audio_trashed_at.map(|at| at.secs),
            ],
        )
        .map(|_| ())
        .map_err(map)
}

fn one_record(connection: &Connection, filter: &str, key: &str) -> Result<Option<RecordingRecord>, LedgerError> {
    connection
        .query_row(&format!("SELECT {RECORDING_COLUMNS} FROM recordings WHERE {filter} = ?1"), [key], |row| Ok(record_from_row(row)))
        .optional()
        .map_err(map)?
        .transpose()
}

impl RecordingLedger for SqliteLedger {
    fn seen(&self, path: &Path) -> Result<Option<SeenRow>, LedgerError> {
        self.read(|c| {
            c.query_row(&format!("SELECT {SEEN_COLUMNS} FROM seen WHERE path = ?1"), [path.to_string_lossy()], |row| Ok(seen_from_row(row)))
                .optional()
                .map_err(map)?
                .transpose()
        })
    }

    fn seen_all(&self) -> Result<Vec<SeenRow>, LedgerError> {
        self.read(|c| {
            let mut statement = c.prepare(&format!("SELECT {SEEN_COLUMNS} FROM seen ORDER BY path")).map_err(map)?;
            let rows = statement.query_map([], |row| Ok(seen_from_row(row))).map_err(map)?;
            rows.map(|row| row.map_err(map).and_then(|row| row)).collect()
        })
    }

    fn record_seen(&self, row: &SeenRow) -> Result<(), LedgerError> {
        self.transaction(|t| upsert_seen(t, row))
    }

    fn by_digest(&self, digest: &Sha256Digest) -> Result<Option<RecordingRecord>, LedgerError> {
        self.read(|c| one_record(c, "digest", &digest.hex()))
    }

    fn by_id(&self, id: &RecordingId) -> Result<Option<RecordingRecord>, LedgerError> {
        self.read(|c| one_record(c, "id", id.as_str()))
    }

    fn recordings(&self) -> Result<Vec<RecordingRecord>, LedgerError> {
        self.read(|c| {
            let mut statement = c.prepare(&format!("SELECT {RECORDING_COLUMNS} FROM recordings ORDER BY captured_at, id")).map_err(map)?;
            let rows = statement.query_map([], |row| Ok(record_from_row(row))).map_err(map)?;
            rows.map(|row| row.map_err(map).and_then(|row| row)).collect()
        })
    }

    fn commit(&self, batch: &LedgerCommit) -> Result<(), LedgerError> {
        self.transaction(|t| {
            for row in &batch.seen {
                upsert_seen(t, row)?;
            }
            for entry in &batch.publications {
                journal::upsert_publication(t, entry)?;
            }
            for recording in &batch.recordings {
                insert_recording(t, recording)?;
            }
            Ok(())
        })
    }

    fn set_source_path(&self, id: &RecordingId, path: &Path) -> Result<(), LedgerError> {
        self.transaction(|t| {
            t.execute("UPDATE recordings SET source_path = ?1 WHERE id = ?2", params![path.to_string_lossy(), id.as_str()])
                .map(|_| ())
                .map_err(map)
        })
    }

    fn set_audio_trashed(&self, id: &RecordingId, at: UtcInstant) -> Result<(), LedgerError> {
        self.transaction(|t| {
            t.execute("UPDATE recordings SET audio_trashed_at = ?1 WHERE id = ?2", params![at.secs, id.as_str()])
                .map(|_| ())
                .map_err(map)
        })
    }
}
```

The commit writes the seen rows and the journal entries before the recordings, so the uniqueness
constraints on `recordings` are the last thing checked and a violation rolls back everything before it.

`crates/vpt-adapters/src/ledger/sqlite/journal.rs`:

```rust
//! `PublicationJournal` over SQLite: the `dirty_publications` table. An entry
//! is written only inside the ledger's one-transaction commit.

use super::{SqliteLedger, map};
use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use vpt_application::ports::{DirtyPublication, LedgerError, PublicationJournal};
use vpt_domain::digest::Sha256Digest;
use vpt_domain::time::UtcInstant;

pub(super) fn upsert_publication(connection: &Connection, entry: &DirtyPublication) -> Result<(), LedgerError> {
    connection
        .execute(
            "INSERT INTO dirty_publications (target, expected_previous, intended, recorded_at) VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT(target) DO UPDATE SET expected_previous = excluded.expected_previous, \
             intended = excluded.intended, recorded_at = excluded.recorded_at",
            params![
                entry.target.to_string_lossy(),
                entry.expected_previous.as_ref().map(Sha256Digest::hex),
                entry.intended.hex(),
                entry.recorded_at.secs,
            ],
        )
        .map(|_| ())
        .map_err(map)
}

fn digest_column(text: &str, what: &str) -> Result<Sha256Digest, LedgerError> {
    Sha256Digest::from_hex(text).ok_or_else(|| LedgerError::Corrupt(format!("unreadable {what}")))
}

impl PublicationJournal for SqliteLedger {
    fn pending_publications(&self) -> Result<Vec<DirtyPublication>, LedgerError> {
        self.read(|c| {
            let mut statement = c
                .prepare("SELECT target, expected_previous, intended, recorded_at FROM dirty_publications ORDER BY recorded_at, target")
                .map_err(map)?;
            let rows = statement
                .query_map([], |row| {
                    let target: String = row.get(0)?;
                    let previous: Option<String> = row.get(1)?;
                    let intended: String = row.get(2)?;
                    let recorded_at: i64 = row.get(3)?;
                    Ok((target, previous, intended, recorded_at))
                })
                .map_err(map)?;
            rows.map(|row| {
                let (target, previous, intended, recorded_at) = row.map_err(map)?;
                Ok(DirtyPublication {
                    target: PathBuf::from(target),
                    expected_previous: previous.map(|hex| digest_column(&hex, "expected digest")).transpose()?,
                    intended: digest_column(&intended, "intended digest")?,
                    recorded_at: UtcInstant { secs: recorded_at },
                })
            })
            .collect()
        })
    }

    fn clear_publication(&self, target: &Path) -> Result<(), LedgerError> {
        self.transaction(|t| {
            t.execute("DELETE FROM dirty_publications WHERE target = ?1", [target.to_string_lossy()])
                .map(|_| ())
                .map_err(map)
        })
    }
}
```

`crates/vpt-adapters/src/ledger/memory.rs`, above its test module:

```rust
//! The in-memory ledger: the same contract as SQLite, for use-case tests.

use std::path::Path;
use std::sync::Mutex;
use vpt_application::ports::{DirtyPublication, LedgerCommit, LedgerError, PublicationJournal, RecordingLedger, RecordingRecord, SeenRow};
use vpt_domain::digest::Sha256Digest;
use vpt_domain::identity::RecordingId;
use vpt_domain::time::UtcInstant;

#[derive(Default)]
struct State {
    seen: Vec<SeenRow>,
    recordings: Vec<RecordingRecord>,
    publications: Vec<DirtyPublication>,
}

#[derive(Default)]
pub struct MemoryLedger {
    state: Mutex<State>,
}

impl MemoryLedger {
    pub fn new() -> MemoryLedger {
        MemoryLedger::default()
    }

    fn with<T>(&self, operation: impl FnOnce(&mut State) -> T) -> T {
        let mut state = self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        operation(&mut state)
    }
}

/// Insert or update by path; a stored `first_seen` survives the update.
fn upsert(seen: &mut Vec<SeenRow>, row: &SeenRow) {
    match seen.iter_mut().find(|existing| existing.path == row.path) {
        Some(existing) => *existing = SeenRow { first_seen: existing.first_seen, ..row.clone() },
        None => seen.push(row.clone()),
    }
}

fn record_publication(publications: &mut Vec<DirtyPublication>, entry: &DirtyPublication) {
    publications.retain(|existing| existing.target != entry.target);
    publications.push(entry.clone());
}

fn conflicts(known: &[RecordingRecord], candidate: &RecordingRecord) -> bool {
    known.iter().any(|r| r.digest == candidate.digest || r.id == candidate.id)
}

impl RecordingLedger for MemoryLedger {
    fn seen(&self, path: &Path) -> Result<Option<SeenRow>, LedgerError> {
        Ok(self.with(|s| s.seen.iter().find(|row| row.path == path).cloned()))
    }

    fn seen_all(&self) -> Result<Vec<SeenRow>, LedgerError> {
        Ok(self.with(|s| {
            let mut all = s.seen.clone();
            all.sort_by(|a, b| a.path.cmp(&b.path));
            all
        }))
    }

    fn record_seen(&self, row: &SeenRow) -> Result<(), LedgerError> {
        self.with(|s| upsert(&mut s.seen, row));
        Ok(())
    }

    fn by_digest(&self, digest: &Sha256Digest) -> Result<Option<RecordingRecord>, LedgerError> {
        Ok(self.with(|s| s.recordings.iter().find(|r| r.digest == *digest).cloned()))
    }

    fn by_id(&self, id: &RecordingId) -> Result<Option<RecordingRecord>, LedgerError> {
        Ok(self.with(|s| s.recordings.iter().find(|r| r.id == *id).cloned()))
    }

    fn recordings(&self) -> Result<Vec<RecordingRecord>, LedgerError> {
        Ok(self.with(|s| {
            let mut all = s.recordings.clone();
            all.sort_by(|a, b| (a.captured_at, a.id.as_str()).cmp(&(b.captured_at, b.id.as_str())));
            all
        }))
    }

    fn commit(&self, batch: &LedgerCommit) -> Result<(), LedgerError> {
        self.with(|s| {
            let mut recordings = s.recordings.clone();
            for recording in &batch.recordings {
                if conflicts(&recordings, recording) {
                    return Err(LedgerError::Conflict("digest or identity already recorded".into()));
                }
                recordings.push(recording.clone());
            }
            s.recordings = recordings;
            for row in &batch.seen {
                upsert(&mut s.seen, row);
            }
            for entry in &batch.publications {
                record_publication(&mut s.publications, entry);
            }
            Ok(())
        })
    }

    fn set_source_path(&self, id: &RecordingId, path: &Path) -> Result<(), LedgerError> {
        self.with(|s| {
            if let Some(r) = s.recordings.iter_mut().find(|r| r.id == *id) {
                r.source_path = Some(path.to_path_buf());
            }
        });
        Ok(())
    }

    fn set_audio_trashed(&self, id: &RecordingId, at: UtcInstant) -> Result<(), LedgerError> {
        self.with(|s| {
            if let Some(r) = s.recordings.iter_mut().find(|r| r.id == *id) {
                r.audio_trashed_at = Some(at);
            }
        });
        Ok(())
    }
}

impl PublicationJournal for MemoryLedger {
    fn pending_publications(&self) -> Result<Vec<DirtyPublication>, LedgerError> {
        Ok(self.with(|s| {
            let mut all = s.publications.clone();
            all.sort_by(|a, b| (a.recorded_at, &a.target).cmp(&(b.recorded_at, &b.target)));
            all
        }))
    }

    fn clear_publication(&self, target: &Path) -> Result<(), LedgerError> {
        self.with(|s| s.publications.retain(|existing| existing.target != target));
        Ok(())
    }
}
```

The memory commit validates every recording of the batch against the rows it already holds and the
earlier rows of the same batch before it changes anything, which is the transaction the SQLite side gets
from the database.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo fmt --all`

Run: `cargo test -p vpt-adapters ledger`

Expected: the twelve open tests plus twelve contract tests per implementation, 36 in all, PASS. Run
`cargo clippy -p vpt-adapters --all-targets -- -D warnings` and expect no warnings.

- [ ] **Step 5: Commit**

Run: `cargo fmt --all`

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(ledger): recordings, seen rows and the publication journal under one commit"
```

______________________________________________________________________

### Task 13: The write lock

**Files:**

- Create: `crates/vpt-adapters/src/lock.rs`
- Modify: `crates/vpt-adapters/src/lib.rs`

**Interfaces:**

- Consumes: `vpt_adapters::{Access, ContainedError, RootDir}`.

- Produces: `vpt_adapters::{WriteLock, LockError::{Busy, Contained(ContainedError), Io(String)}}` with
  `WriteLock::acquire(state: &RootDir, wait: Duration) -> Result<WriteLock, LockError>`; the lock file is
  `write.lock` below the state root, created at mode 0600 and opened without following a link; dropping
  the value releases the lock.

- [ ] **Step 1: Write the failing tests**

`crates/vpt-adapters/src/lib.rs` gains `mod lock;` and `pub use lock::{LockError, WriteLock};`.
`crates/vpt-adapters/src/lock.rs` starts as its test module alone:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn state() -> (tempfile::TempDir, RootDir) {
        let temp = tempfile::tempdir().expect("temp");
        let root = RootDir::open(&temp.path().canonicalize().expect("canonical")).expect("root");
        (temp, root)
    }

    #[test]
    fn a_held_lock_makes_a_second_acquisition_busy_after_its_wait() {
        let (_temp, state) = state();
        let held = WriteLock::acquire(&state, Duration::ZERO).expect("first");
        assert_eq!(WriteLock::acquire(&state, Duration::ZERO).err(), Some(LockError::Busy));
        assert_eq!(WriteLock::acquire(&state, Duration::from_millis(30)).err(), Some(LockError::Busy));
        drop(held);
    }

    #[test]
    fn dropping_the_lock_lets_the_next_acquisition_through() {
        let (_temp, state) = state();
        drop(WriteLock::acquire(&state, Duration::ZERO).expect("first"));
        assert!(WriteLock::acquire(&state, Duration::ZERO).is_ok());
        let mode = std::fs::metadata(state.path().join("write.lock")).expect("lock file").permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn a_link_at_the_lock_name_is_refused() {
        let (temp, state) = state();
        let elsewhere = temp.path().join("elsewhere");
        std::fs::write(&elsewhere, b"").expect("elsewhere");
        std::os::unix::fs::symlink(&elsewhere, state.path().join("write.lock")).expect("link");
        let refused = ContainedError::NotRegular(state.path().join("write.lock"));
        assert_eq!(WriteLock::acquire(&state, Duration::ZERO).err(), Some(LockError::Contained(refused)));
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-adapters lock`

Expected: the build fails with `cannot find` for `WriteLock` and `LockError`.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-adapters/src/lock.rs`, above its test module:

```rust
//! The advisory write lock every mutating command holds: `flock` on
//! `<state_dir>/write.lock`, close-on-exec, a bounded wait.

use crate::contained::{Access, ContainedError, RootDir};
use std::fs::File;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::time::{Duration, Instant};

const LOCK_FILE: &str = "write.lock";
const POLL: Duration = Duration::from_millis(25);

#[derive(Debug, PartialEq, Eq)]
pub enum LockError {
    Busy,
    Contained(ContainedError),
    Io(String),
}

pub struct WriteLock {
    file: File,
}

impl WriteLock {
    pub fn acquire(state: &RootDir, wait: Duration) -> Result<WriteLock, LockError> {
        let file = open_lock_file(state).map_err(LockError::Contained)?;
        let deadline = Instant::now() + wait;
        loop {
            // SAFETY: the descriptor is open for the lifetime of `file`; flock takes
            // no pointer and LOCK_NB makes the call return immediately.
            let outcome = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
            if outcome == 0 {
                return Ok(WriteLock { file });
            }
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() != Some(libc::EWOULDBLOCK) {
                return Err(LockError::Io(error.to_string()));
            }
            if Instant::now() >= deadline {
                return Err(LockError::Busy);
            }
            std::thread::sleep(POLL.min(deadline.saturating_duration_since(Instant::now())));
        }
    }
}

/// Open the lock file below the state root, creating it privately when absent.
fn open_lock_file(state: &RootDir) -> Result<File, ContainedError> {
    let name = Path::new(LOCK_FILE);
    match state.open_file(name, Access::ReadWrite) {
        Err(ContainedError::Io { kind: std::io::ErrorKind::NotFound, .. }) => match state.create_file(name, 0o600) {
            Err(ContainedError::Io { kind: std::io::ErrorKind::AlreadyExists, .. }) => state.open_file(name, Access::ReadWrite),
            created => created,
        },
        opened => opened,
    }
}

impl Drop for WriteLock {
    fn drop(&mut self) {
        // SAFETY: the descriptor is still open; releasing a lock we hold cannot fail
        // in a way the drop can act on.
        unsafe {
            libc::flock(self.file.as_raw_fd(), libc::LOCK_UN);
        }
    }
}
```

Rust opens files with `O_CLOEXEC`, and so does `RootDir`, which is the close-on-exec the spec asks for. A
second process that created the file between the failed open and the exclusive creation is handled by
opening what it created.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo fmt --all`

Run: `cargo test -p vpt-adapters lock`

Expected: 3 tests PASS, none waiting longer than the 30 ms it asks for. Run
`cargo clippy -p vpt-adapters --all-targets -- -D warnings` and expect no warnings.

- [ ] **Step 5: Commit**

Run: `cargo fmt --all`

```bash
git add crates/vpt-adapters
SKIP_AI_COMMIT=1 git commit -m "feat(ledger): the bounded advisory write lock below the state root"
```

______________________________________________________________________

### Task 14: Publication repair and the store filesystem

Every mutating command repairs unfinished publications before new work (spec section 4.4). Stage 1
publishes no rendered artifact of its own, so its composition passes a rendering closure that knows no
target; stage 3 passes the note renderer. The repair decision and the filesystem operations are built and
tested here; the journal rows and their one-transaction commit arrived in Task 12.

**Files:**

- Create: `crates/vpt-application/src/ports/stores.rs`, `crates/vpt-application/src/publication.rs`
- Modify: `crates/vpt-application/src/ports/mod.rs`, `crates/vpt-application/src/lib.rs`
- Create: `crates/vpt-adapters/src/stores.rs`
- Modify: `crates/vpt-adapters/src/lib.rs`

**Interfaces:**

- Consumes: `LedgerError`, `DirtyPublication`, `PublicationJournal`;
  `vpt_adapters::{Access, ContainedError, RootDir}`.

- Produces:

  - `vpt_application::ports::{StoreError::{Escape(PathBuf), Io(String)}, Stores}` with
    `trait Stores { fn digest_of(&self, path: &Path) -> Result<Option<Sha256Digest>, StoreError>;`
    `fn validate_path(&self, path: &Path) -> Result<(), StoreError>;`
    `fn digest_bytes(&self, bytes: &[u8]) -> Sha256Digest; fn publish(&self, target: &Path,`
    `bytes: &[u8]) -> Result<(), StoreError>; fn sync_directory_of(&self, path: &Path) ->`
    `Result<(), StoreError>; }` (Task 28 adds `entries`); `Escape` is a path below no store root or one
    reached through a link, and the composition root maps it to exit 3, `path_escape`.
  - `vpt_application::{repair_publications, RepairReport, RepairError}` with
    `repair_publications<J: PublicationJournal, S: Stores>(journal: &J, stores: &S,`
    `render: impl Fn(&Path) -> Option<Vec<u8>>) -> Result<RepairReport, RepairError>` (`None` from the
    closure means no renderer knows the target), `RepairReport { pub completed: Vec<PathBuf>,`
    `pub republished: Vec<PathBuf> }`, `RepairError::{TargetModified(PathBuf),`
    `RenderedDigestMismatch(PathBuf), Sync { path: PathBuf, cause: StoreError }, Render(PathBuf),`
    `Ledger(LedgerError), Stores(StoreError)}`.
  - `vpt_adapters::{FilesystemStores, digest_open(file: &mut File) ->` `std::io::Result<Sha256Digest>,`
    `BUFFER: usize = 64 * 1024}` with
    `FilesystemStores::open(roots: &[PathBuf]) -> Result<FilesystemStores, ContainedError>` and
    `FilesystemStores::open_read_only(roots: &[PathBuf]) -> Result<FilesystemStores, ContainedError>`
    (held `RootDir` handles; read-only construction tolerates absent configured roots and creates
    nothing), implementing `Stores` (and, from Task 28 on, its `entries`). A publication writes a private
    temporary name below the target's own root, `.<name>.vpt-<pid>-<sequence>` with a per-instance
    sequence and exclusive creation, retrying only `AlreadyExists`; a temporary name abandoned by a
    failure is left for the owned cleanup and never reused.

- [ ] **Step 1: Write the failing tests**

Declare the modules first: `crates/vpt-application/src/ports/mod.rs` gains `mod stores;` and
`pub use stores::{StoreError, Stores};`; `crates/vpt-application/src/lib.rs` gains `mod publication;` and
`pub use publication::{RepairError, RepairReport, repair_publications};`;
`crates/vpt-adapters/src/lib.rs` gains `mod stores;` and
`pub use stores::{FilesystemStores, digest_open};`. The three new files start as their test modules.

`crates/vpt-application/src/publication.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::DirtyPublication;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use vpt_domain::digest::Sha256Digest;
    use vpt_domain::time::UtcInstant;

    fn digest_of_bytes(bytes: &[u8]) -> Sha256Digest {
        let mut out = [0u8; 32];
        for (index, byte) in bytes.iter().enumerate() {
            out[index % 32] ^= *byte;
        }
        Sha256Digest(out)
    }

    struct FakeJournal(RefCell<Vec<DirtyPublication>>);
    impl PublicationJournal for FakeJournal {
        fn pending_publications(&self) -> Result<Vec<DirtyPublication>, LedgerError> {
            Ok(self.0.borrow().clone())
        }
        fn clear_publication(&self, target: &Path) -> Result<(), LedgerError> {
            self.0.borrow_mut().retain(|entry| entry.target != target);
            Ok(())
        }
    }

    struct FakeStores {
        contents: RefCell<HashMap<PathBuf, Vec<u8>>>,
        synced: RefCell<Vec<PathBuf>>,
        sync_fails: bool,
    }
    impl Stores for FakeStores {
        fn validate_path(&self, _path: &Path) -> Result<(), StoreError> {
            Ok(())
        }
        fn digest_of(&self, path: &Path) -> Result<Option<Sha256Digest>, StoreError> {
            Ok(self.contents.borrow().get(path).map(|bytes| digest_of_bytes(bytes)))
        }
        fn digest_bytes(&self, bytes: &[u8]) -> Sha256Digest {
            digest_of_bytes(bytes)
        }
        fn publish(&self, target: &Path, bytes: &[u8]) -> Result<(), StoreError> {
            self.contents.borrow_mut().insert(target.to_path_buf(), bytes.to_vec());
            Ok(())
        }
        fn sync_directory_of(&self, path: &Path) -> Result<(), StoreError> {
            if self.sync_fails {
                return Err(StoreError::Io("disk gone".into()));
            }
            self.synced.borrow_mut().push(path.to_path_buf());
            Ok(())
        }
    }

    fn entry(target: &str, previous: Option<&[u8]>, intended: &[u8]) -> DirtyPublication {
        DirtyPublication {
            target: PathBuf::from(target),
            expected_previous: previous.map(digest_of_bytes),
            intended: digest_of_bytes(intended),
            recorded_at: UtcInstant { secs: 1 },
        }
    }

    fn journal(entries: Vec<DirtyPublication>) -> FakeJournal {
        FakeJournal(RefCell::new(entries))
    }

    fn stores(contents: &[(&str, &[u8])], sync_fails: bool) -> FakeStores {
        FakeStores {
            contents: RefCell::new(contents.iter().map(|(p, b)| (PathBuf::from(p), b.to_vec())).collect()),
            synced: RefCell::new(vec![]),
            sync_fails,
        }
    }

    fn renders(bytes: &'static [u8]) -> impl Fn(&Path) -> Option<Vec<u8>> {
        move |_target: &Path| Some(bytes.to_vec())
    }

    fn nothing(_target: &Path) -> Option<Vec<u8>> {
        None
    }

    #[test]
    fn a_target_already_holding_the_intended_bytes_is_completed_after_a_directory_sync() {
        let journal = journal(vec![entry("/t/a.md", Some(b"old"), b"new")]);
        let stores = stores(&[("/t/a.md", b"new")], false);
        let report = repair_publications(&journal, &stores, nothing).expect("repaired");
        assert_eq!(report.completed, vec![PathBuf::from("/t/a.md")]);
        assert_eq!(stores.synced.borrow().as_slice(), [PathBuf::from("/t/a.md")]);
        assert!(journal.0.borrow().is_empty());
    }

    #[test]
    fn a_failed_directory_sync_leaves_the_entry_pending() {
        let journal = journal(vec![entry("/t/a.md", Some(b"old"), b"new")]);
        let stores = stores(&[("/t/a.md", b"new")], true);
        let error = repair_publications(&journal, &stores, nothing).unwrap_err();
        assert!(matches!(error, RepairError::Sync { .. }), "{error:?}");
        assert_eq!(journal.0.borrow().len(), 1);
    }

    #[test]
    fn a_target_holding_the_expected_previous_bytes_is_published_over_from_the_rendering() {
        let journal = journal(vec![entry("/t/a.md", Some(b"old"), b"new")]);
        let stores = stores(&[("/t/a.md", b"old")], false);
        let report = repair_publications(&journal, &stores, renders(b"new")).expect("repaired");
        assert_eq!(report.republished, vec![PathBuf::from("/t/a.md")]);
        assert_eq!(stores.contents.borrow()[Path::new("/t/a.md")], b"new");
        assert_eq!(stores.synced.borrow().as_slice(), [PathBuf::from("/t/a.md")]);
        assert!(journal.0.borrow().is_empty());
    }

    #[test]
    fn an_absent_target_expected_absent_is_published() {
        let journal = journal(vec![entry("/t/b.md", None, b"fresh")]);
        let stores = stores(&[], false);
        let report = repair_publications(&journal, &stores, renders(b"fresh")).expect("repaired");
        assert_eq!(report.republished, vec![PathBuf::from("/t/b.md")]);
        assert_eq!(stores.contents.borrow()[Path::new("/t/b.md")], b"fresh");
    }

    #[test]
    fn any_other_bytes_are_refused_as_target_modified_and_nothing_is_overwritten() {
        let journal = journal(vec![entry("/t/a.md", Some(b"old"), b"new")]);
        let stores = stores(&[("/t/a.md", b"someone else's prose")], false);
        let error = repair_publications(&journal, &stores, renders(b"new")).unwrap_err();
        assert_eq!(error, RepairError::TargetModified(PathBuf::from("/t/a.md")));
        assert_eq!(stores.contents.borrow()[Path::new("/t/a.md")], b"someone else's prose");
        assert_eq!(journal.0.borrow().len(), 1);
    }

    #[test]
    fn a_target_no_renderer_knows_is_reported_and_left_pending() {
        let journal = journal(vec![entry("/t/a.md", Some(b"old"), b"new")]);
        let stores = stores(&[("/t/a.md", b"old")], false);
        let error = repair_publications(&journal, &stores, nothing).unwrap_err();
        assert_eq!(error, RepairError::Render(PathBuf::from("/t/a.md")));
        assert_eq!(journal.0.borrow().len(), 1);
    }

    #[test]
    fn a_rendering_that_does_not_match_the_intended_digest_changes_nothing() {
        let journal = journal(vec![entry("/t/a.md", Some(b"old"), b"new")]);
        let stores = stores(&[("/t/a.md", b"old")], false);
        let error = repair_publications(&journal, &stores, renders(b"not new")).unwrap_err();
        assert_eq!(error, RepairError::RenderedDigestMismatch(PathBuf::from("/t/a.md")));
        assert_eq!(stores.contents.borrow()[Path::new("/t/a.md")], b"old");
        assert!(stores.synced.borrow().is_empty());
        assert_eq!(journal.0.borrow().len(), 1);
    }
}
```

`crates/vpt-adapters/src/stores.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn store() -> (tempfile::TempDir, PathBuf, FilesystemStores) {
        let temp = tempfile::tempdir().expect("temp");
        let root = temp.path().canonicalize().expect("canonical");
        let stores = FilesystemStores::open(std::slice::from_ref(&root)).expect("opens");
        (temp, root, stores)
    }

    #[test]
    fn digest_of_reads_the_file_agrees_with_digest_bytes_and_reports_absence_as_none() {
        let (_temp, root, stores) = store();
        std::fs::write(root.join("a.md"), b"hello").expect("write");
        let digest = stores.digest_of(&root.join("a.md")).expect("digest").expect("present");
        assert_eq!(digest.hex(), "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824");
        assert_eq!(stores.digest_bytes(b"hello"), digest);
        assert_eq!(stores.digest_of(&root.join("missing")).expect("absent"), None);
    }

    #[test]
    fn publish_replaces_the_target_atomically_with_mode_0644_and_leaves_no_temporary_name() {
        let (_temp, root, stores) = store();
        let target = root.join("a.md");
        std::fs::write(&target, b"old").expect("old");
        stores.publish(&target, b"new").expect("publish");
        assert_eq!(std::fs::read(&target).expect("read"), b"new");
        let names: Vec<_> = std::fs::read_dir(&root).expect("dir").map(|e| e.expect("entry").file_name()).collect();
        assert_eq!(names, vec![std::ffi::OsString::from("a.md")]);
        assert_eq!(std::fs::metadata(&target).expect("meta").permissions().mode() & 0o777, 0o644);
        assert_eq!(stores.sync_directory_of(&target), Ok(()));
    }

    #[test]
    fn a_leftover_temporary_name_is_skipped_never_reused() {
        let (_temp, root, stores) = store();
        let pid = std::process::id();
        for sequence in 0..2 {
            std::fs::write(root.join(format!(".a.md.vpt-{pid}-{sequence}")), b"abandoned").expect("blocker");
        }
        stores.publish(&root.join("a.md"), b"new").expect("publish");
        assert_eq!(std::fs::read(root.join("a.md")).expect("read"), b"new");
        for sequence in 0..2 {
            assert_eq!(std::fs::read(root.join(format!(".a.md.vpt-{pid}-{sequence}"))).expect("kept"), b"abandoned");
        }
        assert!(!root.join(format!(".a.md.vpt-{pid}-2")).exists());
    }

    #[test]
    fn a_path_below_no_store_root_or_reached_through_a_link_is_an_escape() {
        let (temp, root, stores) = store();
        let outside = temp.path().canonicalize().expect("canonical").parent().expect("parent").join("a.md");
        assert_eq!(stores.publish(&outside, b"x"), Err(StoreError::Escape(outside.clone())));
        assert_eq!(stores.digest_of(&outside), Err(StoreError::Escape(outside.clone())));
        assert_eq!(stores.sync_directory_of(&outside), Err(StoreError::Escape(outside)));
        let nested = root.join("sub/a.md");
        assert_eq!(stores.publish(&nested, b"x"), Err(StoreError::Escape(nested)));
        std::fs::write(root.join("real.md"), b"real").expect("real");
        std::os::unix::fs::symlink(root.join("real.md"), root.join("link.md")).expect("link");
        assert_eq!(stores.digest_of(&root.join("link.md")), Err(StoreError::Escape(root.join("link.md"))));
        assert_eq!(stores.validate_path(&root.join("link.md")), Err(StoreError::Escape(root.join("link.md"))));
        assert_eq!(stores.validate_path(&root.join("real.md")), Ok(()));
        assert_eq!(stores.validate_path(&root.join("absent.md")), Ok(()));
    }

    #[test]
    fn a_read_only_store_open_keeps_absent_roots_absent_and_rejects_links() {
        let temp = tempfile::tempdir().expect("temp");
        let base = temp.path().canonicalize().expect("canonical");
        let missing = base.join("missing");
        let stores = FilesystemStores::open_read_only(std::slice::from_ref(&missing)).expect("read only");
        assert!(stores.roots.is_empty());
        assert_eq!(stores.configured, vec![missing.clone()]);
        assert!(!missing.exists());
        std::os::unix::fs::symlink(&base, base.join("link")).expect("link");
        assert!(FilesystemStores::open_read_only(&[base.join("link")]).is_err());
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run independently, including the second command after the first expected failure:

Run: `cargo test -p vpt-application publication`

Run: `cargo test -p vpt-adapters stores`

Expected: the first build fails with `unresolved import` for `StoreError`, `Stores`,
`repair_publications`, `RepairReport` and `RepairError`; the second (run it after the first is green)
with `cannot find` for `FilesystemStores`.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-application/src/ports/stores.rs`:

```rust
//! The filesystem operations a publication needs, over the store roots.

use std::path::{Path, PathBuf};
use vpt_domain::digest::Sha256Digest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    /// Below no store root, nested, or reached through a link: exit 3, `path_escape`.
    Escape(PathBuf),
    Io(String),
}

pub trait Stores {
    /// Validate a store-owned leaf without following links, allowing an absent leaf.
    fn validate_path(&self, path: &Path) -> Result<(), StoreError>;
    /// `None` when the path is absent.
    fn digest_of(&self, path: &Path) -> Result<Option<Sha256Digest>, StoreError>;
    fn digest_bytes(&self, bytes: &[u8]) -> Sha256Digest;
    /// Write to a private temporary name beside the target, sync, rename over the target.
    fn publish(&self, target: &Path, bytes: &[u8]) -> Result<(), StoreError>;
    fn sync_directory_of(&self, path: &Path) -> Result<(), StoreError>;
}
```

`crates/vpt-application/src/publication.rs`, above its test module:

```rust
//! Repair unfinished publications before new work, per spec section 4.4.

use crate::ports::{LedgerError, PublicationJournal, StoreError, Stores};
use std::path::{Path, PathBuf};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct RepairReport {
    pub completed: Vec<PathBuf>,
    pub republished: Vec<PathBuf>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RepairError {
    TargetModified(PathBuf),
    RenderedDigestMismatch(PathBuf),
    Sync { path: PathBuf, cause: StoreError },
    Render(PathBuf),
    Ledger(LedgerError),
    Stores(StoreError),
}

/// Complete or redo every pending publication; `render` re-creates a target's
/// bytes from committed state and answers `None` for a target it does not know.
pub fn repair_publications<J: PublicationJournal, S: Stores>(
    journal: &J,
    stores: &S,
    render: impl Fn(&Path) -> Option<Vec<u8>>,
) -> Result<RepairReport, RepairError> {
    let mut report = RepairReport::default();
    for entry in journal.pending_publications().map_err(RepairError::Ledger)? {
        let current = stores.digest_of(&entry.target).map_err(RepairError::Stores)?;
        if current == Some(entry.intended) {
            sync(stores, &entry.target)?;
            journal.clear_publication(&entry.target).map_err(RepairError::Ledger)?;
            report.completed.push(entry.target);
        } else if current == entry.expected_previous {
            let bytes = render(&entry.target).ok_or_else(|| RepairError::Render(entry.target.clone()))?;
            if stores.digest_bytes(&bytes) != entry.intended {
                return Err(RepairError::RenderedDigestMismatch(entry.target));
            }
            stores.publish(&entry.target, &bytes).map_err(RepairError::Stores)?;
            sync(stores, &entry.target)?;
            journal.clear_publication(&entry.target).map_err(RepairError::Ledger)?;
            report.republished.push(entry.target);
        } else {
            return Err(RepairError::TargetModified(entry.target));
        }
    }
    Ok(report)
}

fn sync<S: Stores>(stores: &S, path: &Path) -> Result<(), RepairError> {
    stores.sync_directory_of(path).map_err(|cause| RepairError::Sync { path: path.to_path_buf(), cause })
}
```

`crates/vpt-adapters/src/stores.rs`, above its test module:

```rust
//! Filesystem operations over the store roots, each held as a directory
//! descriptor: digests, atomic publication, directory syncs.

use crate::contained::{Access, ContainedError, RootDir};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use vpt_application::ports::{StoreError, Stores};
use vpt_domain::digest::Sha256Digest;

pub const BUFFER: usize = 64 * 1024;

pub struct FilesystemStores {
    roots: Vec<RootDir>,
    configured: Vec<PathBuf>,
    sequence: AtomicU64,
}

/// The SHA-256 of an open file, read from its start in 64 KiB buffers.
pub fn digest_open(file: &mut File) -> std::io::Result<Sha256Digest> {
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; BUFFER];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(Sha256Digest(hasher.finalize().into()))
}

fn store_error(path: &Path, error: ContainedError) -> StoreError {
    match error {
        ContainedError::Io { kind, .. } => StoreError::Io(format!("{kind} at {}", path.display())),
        ContainedError::Escape { .. } | ContainedError::NotRegular(_) | ContainedError::NotADirectory(_) => {
            StoreError::Escape(path.to_path_buf())
        }
    }
}

fn io(path: &Path, error: &std::io::Error) -> StoreError {
    StoreError::Io(format!("{} at {}", error.kind(), path.display()))
}

impl FilesystemStores {
    pub fn open(roots: &[PathBuf]) -> Result<FilesystemStores, ContainedError> {
        let configured = roots.to_vec();
        let roots = roots.iter().map(|root| RootDir::open(root)).collect::<Result<Vec<_>, _>>()?;
        Ok(FilesystemStores { roots, configured, sequence: AtomicU64::new(0) })
    }

    pub fn open_read_only(paths: &[PathBuf]) -> Result<FilesystemStores, ContainedError> {
        let mut roots = Vec::new();
        for path in paths {
            match RootDir::open(path) {
                Ok(root) => roots.push(root),
                Err(ContainedError::Io { kind: std::io::ErrorKind::NotFound, .. }) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(FilesystemStores { roots, configured: paths.to_vec(), sequence: AtomicU64::new(0) })
    }

    /// The root that owns `path` and the validated leaf below it.
    fn owner(&self, path: &Path) -> Result<(&RootDir, PathBuf), StoreError> {
        if !self.configured.iter().any(|root| path.parent() == Some(root.as_path())) {
            return Err(StoreError::Escape(path.to_path_buf()));
        }
        let root = self
            .roots
            .iter()
            .find(|root| path.parent() == Some(root.path()))
            .ok_or_else(|| StoreError::Escape(path.to_path_buf()))?;
        let leaf = root.leaf(path).map_err(|error| store_error(path, error))?;
        Ok((root, leaf))
    }

    /// A fresh private temporary name beside the target: exclusive creation,
    /// a per-instance sequence, and only `AlreadyExists` retried.
    fn create_temporary(&self, root: &RootDir, name: &str) -> Result<(File, PathBuf), StoreError> {
        loop {
            let sequence = self.sequence.fetch_add(1, Ordering::Relaxed);
            let temporary = PathBuf::from(format!(".{name}.vpt-{}-{sequence}", std::process::id()));
            match root.create_file(&temporary, 0o644) {
                Ok(file) => return Ok((file, temporary)),
                Err(ContainedError::Io { kind: std::io::ErrorKind::AlreadyExists, .. }) => continue,
                Err(error) => return Err(store_error(&root.path().join(temporary), error)),
            }
        }
    }
}

impl Stores for FilesystemStores {
    fn validate_path(&self, path: &Path) -> Result<(), StoreError> {
        let (root, leaf) = self.owner(path)?;
        root.revalidate().map_err(|error| store_error(path, error))?;
        match root.stat(&leaf) {
            Ok(stat) if stat.kind == crate::contained::Kind::File => Ok(()),
            Ok(_) => Err(StoreError::Escape(path.to_path_buf())),
            Err(ContainedError::Io { kind: std::io::ErrorKind::NotFound, .. }) => Ok(()),
            Err(error) => Err(store_error(path, error)),
        }
    }

    fn digest_of(&self, path: &Path) -> Result<Option<Sha256Digest>, StoreError> {
        let (root, leaf) = self.owner(path)?;
        match root.open_file(&leaf, Access::Read) {
            Ok(mut file) => digest_open(&mut file).map(Some).map_err(|error| io(path, &error)),
            Err(ContainedError::Io { kind: std::io::ErrorKind::NotFound, .. }) => Ok(None),
            Err(error) => Err(store_error(path, error)),
        }
    }

    fn digest_bytes(&self, bytes: &[u8]) -> Sha256Digest {
        Sha256Digest(Sha256::digest(bytes).into())
    }

    fn publish(&self, target: &Path, bytes: &[u8]) -> Result<(), StoreError> {
        let (root, leaf) = self.owner(target)?;
        let name = leaf.file_name().and_then(|name| name.to_str()).ok_or_else(|| StoreError::Escape(target.to_path_buf()))?;
        let (mut file, temporary) = self.create_temporary(root, name)?;
        file.write_all(bytes).and_then(|()| file.sync_all()).map_err(|error| io(target, &error))?;
        drop(file);
        root.rename_over(&temporary, &leaf).map_err(|error| store_error(target, error))?;
        root.sync().map_err(|error| store_error(target, error))
    }

    fn sync_directory_of(&self, path: &Path) -> Result<(), StoreError> {
        let (root, _leaf) = self.owner(path)?;
        root.sync().map_err(|error| store_error(path, error))
    }
}
```

A target that is a link at the leaf is refused by `digest_of` before anything is written, so repair never
renders over a link; `rename_over` replaces the name itself, never what a link points at.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo fmt --all`

Run: `cargo test --workspace --features dev-tools`

Expected: all PASS, the seven repair tests and the five store tests included. Run
`cargo clippy --workspace --all-targets --features dev-tools -- -D warnings` and expect no warnings.

- [ ] **Step 5: Commit**

Run: `cargo fmt --all`

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(stores): publication repair before new work over the store descriptors"
```

______________________________________________________________________

### Task 15: The Voice Memos store, read-only

The recorder port is generic over its handle: the adapter's handle is the open read-only descriptor, and
every operation on it (metadata, bounded reads, the staging clone) is a method of the port, so the domain
gate reads through a closure and the application never names a file type. The recordings directory is
held as a root descriptor; a listing entry is judged through it and a candidate path is validated against
it before anything is opened.

**Files:**

- Create: `crates/vpt-adapters/src/voice_memos/store/tests.rs`

- Create: `crates/vpt-application/src/ports/recorder.rs`

- Modify: `crates/vpt-application/src/ports/mod.rs`

- Create: `crates/vpt-adapters/src/voice_memos/mod.rs`, `crates/vpt-adapters/src/voice_memos/store.rs`

- Modify: `crates/vpt-adapters/src/lib.rs`

**Interfaces:**

- Consumes: `vpt_domain::container::ReadFailure`, `vpt_domain::time::FileTime`,
  `vpt_adapters::{Access, ContainedError, Kind, RootDir}` and
  `RootDir::identity(&self) -> Result<(u64, u64), ContainedError>` (device and inode).

- Produces, in `vpt_application::ports` (from the private file `ports/recorder.rs`):

  - `Candidate { pub path: PathBuf, pub file_name: String, pub size: u64, pub mtime: FileTime,`
    `pub flags: u32 }` (the whole `st_flags` word);
    `SourceMetadata { pub device: u64, pub inode: u64, pub size: u64, pub mtime: FileTime }`;
    `RecorderError::{Unreadable(String), NotRegular(PathBuf), NotFound(PathBuf), Escape(PathBuf),`
    `Io(String)}`; `CloneKind::{CopyOnWrite, ByteCopy}`;
    `CloneError::{NoSpace, Exists, Escape(PathBuf), Io(String)}` (`Exists` when the destination name was
    already taken, so nothing of it is the caller's).
  - `trait RecorderStore { type Handle; fn candidates(&self) -> Result<Vec<Candidate>, RecorderError>;`
    `fn candidate(&self, path: &Path) -> Result<Candidate, RecorderError>;`
    `fn open(&self, path: &Path) -> Result<Self::Handle, RecorderError>;`
    `fn metadata(&self, handle: &Self::Handle) -> Result<SourceMetadata, RecorderError>;`
    `fn read_at(&self, handle: &Self::Handle, offset: u64, buf: &mut [u8]) -> Result<(), ReadFailure>;`
    `fn clone_into(&self, handle: &Self::Handle, directory: &Path, directory_identity: (u64, u64), name: &str) -> Result<CloneKind,`
    `CloneError>; fn subdirectory_counts(&self) -> Vec<(String, Option<u64>)>; }`. `clone_into` clones
    the open descriptor to `<directory>/<name>` relative to that directory's own descriptor, with the
    byte-copy fallback on `EXDEV` only; Task 16 adds `title`.

- `vpt_adapters::VoiceMemosStore::open(recordings_dir: &Path) ->`
  `Result<VoiceMemosStore, ContainedError>` with `type Handle = std::fs::File`, and
  `vpt_adapters::APPLE_SUBDIRECTORIES: [&str; 4]` (`voice_memos/mod.rs` keeps `store` private and
  re-exports both).

- [ ] **Step 1: Write the failing tests**

Declare the modules first: `crates/vpt-application/src/ports/mod.rs` gains `mod recorder;` and
`pub use recorder::{Candidate, CloneError, CloneKind, RecorderError, RecorderStore, SourceMetadata};`;
`crates/vpt-adapters/src/lib.rs` gains `mod voice_memos;` and
`pub use voice_memos::{APPLE_SUBDIRECTORIES, VoiceMemosStore};`;
`crates/vpt-adapters/src/voice_memos/mod.rs` is:

```rust
//! Apple's Voice Memos store, read-only: the listing, the descriptors and
//! the private copy of its database.

mod store;

pub use store::{APPLE_SUBDIRECTORIES, VoiceMemosStore};
```

`crates/vpt-adapters/src/voice_memos/store.rs` starts with the registered test child below. Create the
child before the red run:

```rust
#[cfg(test)]
mod tests;
```

`crates/vpt-adapters/src/voice_memos/store/tests.rs`:

```rust
use super::*;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use vpt_domain::fixtures::m4a;
use vpt_domain::sweep::SF_DATALESS;

fn store_with(files: &[(&str, &[u8])]) -> (tempfile::TempDir, PathBuf, VoiceMemosStore) {
    let temp = tempfile::tempdir().expect("temp");
    let recordings = temp.path().canonicalize().expect("canonical").join("Recordings");
    std::fs::create_dir_all(&recordings).expect("recordings");
    for subdirectory in APPLE_SUBDIRECTORIES {
        std::fs::create_dir_all(recordings.join(subdirectory)).expect("subdirectory");
        std::fs::write(recordings.join(subdirectory).join("inner.m4a"), b"never listed").expect("inner");
    }
    for (name, bytes) in files {
        std::fs::write(recordings.join(name), bytes).expect("fixture");
    }
    let store = VoiceMemosStore::open(&recordings).expect("opens");
    (temp, recordings, store)
}

#[test]
fn candidates_are_the_m4a_files_at_depth_one_and_nothing_else() {
    let (_temp, recordings, store) = store_with(&[("a.m4a", b"aaa"), ("b.M4A", b"bbb"), ("a.waveform", b"w"), ("notes.txt", b"n")]);
    std::os::unix::fs::symlink(recordings.join("a.m4a"), recordings.join("link.m4a")).expect("link");
    let names: Vec<String> = store.candidates().expect("list").into_iter().map(|c| c.file_name).collect();
    assert_eq!(names, vec!["a.m4a"]);
}

#[test]
fn a_candidate_carries_size_mtime_and_its_flags_word() {
    let (_temp, recordings, store) = store_with(&[("a.m4a", b"aaaa")]);
    let candidate = store.candidates().expect("list").remove(0);
    assert_eq!(candidate.path, recordings.join("a.m4a"));
    assert_eq!(candidate.size, 4);
    assert!(candidate.mtime.secs > 0 && candidate.mtime.nanos < 1_000_000_000);
    assert_eq!(candidate.flags & SF_DATALESS, 0);
    assert_eq!(store.candidate(&recordings.join("a.m4a")).expect("one"), candidate);
}

#[test]
fn a_candidate_is_refused_when_absent_a_link_a_directory_or_outside_the_root() {
    let (temp, recordings, store) = store_with(&[("a.m4a", b"aaaa")]);
    std::os::unix::fs::symlink(recordings.join("a.m4a"), recordings.join("link.m4a")).expect("link");
    let absent = recordings.join("absent.m4a");
    assert_eq!(store.candidate(&absent), Err(RecorderError::NotFound(absent)));
    assert_eq!(store.candidate(&recordings.join("link.m4a")), Err(RecorderError::NotRegular(recordings.join("link.m4a"))));
    assert_eq!(store.candidate(&recordings.join("Capture")), Err(RecorderError::NotFound(recordings.join("Capture"))));
    let outside = temp.path().canonicalize().expect("canonical").join("a.m4a");
    assert_eq!(store.candidate(&outside), Err(RecorderError::Escape(outside.clone())));
    assert_eq!(store.open(&outside).err(), Some(RecorderError::Escape(outside)));
    let nested = recordings.join("Capture/inner.m4a");
    assert_eq!(store.open(&nested).err(), Some(RecorderError::Escape(nested)));
}

#[test]
fn open_refuses_a_link_and_reads_a_regular_file_by_descriptor() {
    let (_temp, recordings, store) = store_with(&[("a.m4a", b"hello world")]);
    std::os::unix::fs::symlink(recordings.join("a.m4a"), recordings.join("link.m4a")).expect("link");
    assert_eq!(store.open(&recordings.join("link.m4a")).err(), Some(RecorderError::NotRegular(recordings.join("link.m4a"))));
    let handle = store.open(&recordings.join("a.m4a")).expect("open");
    let mut buffer = [0u8; 5];
    store.read_at(&handle, 6, &mut buffer).expect("read");
    assert_eq!(&buffer, b"world");
    assert!(store.read_at(&handle, 7, &mut buffer).is_err());
    let metadata = store.metadata(&handle).expect("metadata");
    assert_eq!(metadata.size, 11);
    assert!(metadata.inode > 0);
}

#[test]
fn clone_into_reproduces_the_bytes_and_reports_copy_on_write_on_the_same_volume() {
    let (temp, recordings, store) = store_with(&[("a.m4a", &m4a(1_787_604_456, 3, b"payload"))]);
    let audio = temp.path().canonicalize().expect("canonical").join("audio");
    std::fs::create_dir(&audio).expect("audio");
    let handle = store.open(&recordings.join("a.m4a")).expect("open");
    let kind = store.clone_into(&handle, &audio, RootDir::open(&audio).expect("root").identity().expect("identity"), ".vpt-staging-1.m4a").expect("clone");
    assert_eq!(std::fs::read(audio.join(".vpt-staging-1.m4a")).expect("read"), m4a(1_787_604_456, 3, b"payload"));
    assert_eq!(kind, CloneKind::CopyOnWrite);
    assert_eq!(store.clone_into(&handle, &audio, RootDir::open(&audio).expect("root").identity().expect("identity"), ".vpt-staging-1.m4a"), Err(CloneError::Exists));
    assert!(matches!(store.clone_into(&handle, &audio, RootDir::open(&audio).expect("root").identity().expect("identity"), "sub/x.m4a"), Err(CloneError::Escape(_))));
}

#[test]
fn the_byte_copy_reproduces_the_bytes_privately_and_reports_itself() {
    let (temp, recordings, store) = store_with(&[("a.m4a", &[7u8; 200_000])]);
    let audio = temp.path().canonicalize().expect("canonical").join("audio");
    std::fs::create_dir(&audio).expect("audio");
    let handle = store.open(&recordings.join("a.m4a")).expect("open");
    let directory = RootDir::open(&audio).expect("root");
    assert_eq!(byte_copy(&handle, &directory, Path::new("copy.m4a")), Ok(CloneKind::ByteCopy));
    assert_eq!(std::fs::read(audio.join("copy.m4a")).expect("read"), vec![7u8; 200_000]);
    assert_eq!(std::fs::metadata(audio.join("copy.m4a")).expect("meta").permissions().mode() & 0o777, 0o600);
}

#[test]
fn subdirectory_counts_report_each_apple_directory_without_listing_it_as_candidates() {
    let (_temp, _recordings, store) = store_with(&[]);
    let counts = store.subdirectory_counts();
    assert_eq!(counts.len(), 4);
    assert!(counts.iter().all(|(_, count)| *count == Some(1)), "{counts:?}");
    assert!(store.candidates().expect("list").is_empty());
}

#[test]
fn a_missing_recordings_directory_is_refused_at_open() {
    let temp = tempfile::tempdir().expect("temp");
    let absent = temp.path().canonicalize().expect("canonical").join("Recordings");
    let outcome = VoiceMemosStore::open(&absent);
    assert!(matches!(outcome, Err(ContainedError::Io { kind: std::io::ErrorKind::NotFound, .. })));
}
#[test]
fn cloning_refuses_a_replacement_destination_root_before_creating_a_file() {
    let (temp, recordings, store) = store_with(&[("a.m4a", b"audio")]);
    let audio = temp.path().canonicalize().expect("canonical").join("audio");
    std::fs::create_dir(&audio).expect("audio");
    let held = RootDir::open(&audio).expect("root");
    let identity = held.identity().expect("identity");
    std::fs::rename(&audio, audio.with_extension("saved")).expect("move root");
    std::fs::create_dir(&audio).expect("replacement");
    let source = store.open(&recordings.join("a.m4a")).expect("source");
    assert_eq!(store.clone_into(&source, &audio, identity, "stage.m4a"), Err(CloneError::Escape(audio.clone())));
    assert!(std::fs::read_dir(&audio).expect("replacement").next().is_none());
}

#[test]
fn subdirectory_counts_never_follow_a_link_below_the_source_root() {
    let (temp, recordings, store) = store_with(&[]);
    let outside = temp.path().join("outside");
    std::fs::create_dir(&outside).expect("outside");
    std::fs::write(outside.join("one"), b"one").expect("outside entry");
    std::fs::rename(recordings.join("Capture"), recordings.join("old-capture")).expect("move child");
    std::os::unix::fs::symlink(&outside, recordings.join("Capture")).expect("link");
    let counts = store.subdirectory_counts();
    assert_eq!(counts.iter().find(|(name, _)| name == "Capture").expect("count").1, None);
    assert_eq!(std::fs::read(outside.join("one")).expect("untouched"), b"one");
}

```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-adapters voice_memos`

Expected: the build fails with `unresolved import` for the recorder names in `ports/mod.rs` and
`cannot find` for `VoiceMemosStore`, `APPLE_SUBDIRECTORIES` and `byte_copy`.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-application/src/ports/recorder.rs`:

```rust
//! The recorder store: list, open read-only, read and clone through the handle.

use std::path::{Path, PathBuf};
use vpt_domain::container::ReadFailure;
use vpt_domain::time::FileTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub path: PathBuf,
    pub file_name: String,
    pub size: u64,
    pub mtime: FileTime,
    pub flags: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceMetadata {
    pub device: u64,
    pub inode: u64,
    pub size: u64,
    pub mtime: FileTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecorderError {
    Unreadable(String),
    NotRegular(PathBuf),
    NotFound(PathBuf),
    /// Below no recordings root or nested: exit 3, `path_escape`.
    Escape(PathBuf),
    Io(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloneKind {
    CopyOnWrite,
    ByteCopy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloneError {
    NoSpace,
    /// The destination name was already taken; nothing there is the caller's.
    Exists,
    Escape(PathBuf),
    Io(String),
}

pub trait RecorderStore {
    /// The open read-only descriptor of one candidate.
    type Handle;

    fn candidates(&self) -> Result<Vec<Candidate>, RecorderError>;
    fn candidate(&self, path: &Path) -> Result<Candidate, RecorderError>;
    fn open(&self, path: &Path) -> Result<Self::Handle, RecorderError>;
    fn metadata(&self, handle: &Self::Handle) -> Result<SourceMetadata, RecorderError>;
    /// Fill `buf` from `offset`, or fail; the wholeness gate's read callback.
    fn read_at(&self, handle: &Self::Handle, offset: u64, buf: &mut [u8]) -> Result<(), ReadFailure>;
    /// Clone the descriptor to `<directory>/<name>`, relative to that directory's
    /// own descriptor; a byte copy on `EXDEV` only.
    fn clone_into(&self, handle: &Self::Handle, directory: &Path, directory_identity: (u64, u64), name: &str) -> Result<CloneKind, CloneError>;
    fn subdirectory_counts(&self) -> Vec<(String, Option<u64>)>;
}
```

`crates/vpt-adapters/src/voice_memos/store.rs`, above its test module:

```rust
//! The listing at depth one and read-only descriptors that never follow a link.

use crate::contained::{Access, ContainedError, Kind, RootDir};
use std::fs::File;
use std::io::Write;
use std::os::fd::AsFd;
use std::os::unix::fs::{FileExt, MetadataExt};
use std::os::unix::io::AsRawFd;
use std::path::Path;
use vpt_application::ports::{Candidate, CloneError, CloneKind, RecorderError, RecorderStore, SourceMetadata};
use vpt_domain::container::ReadFailure;
use vpt_domain::time::FileTime;

pub const APPLE_SUBDIRECTORIES: [&str; 4] = ["Capture", "CaptureRecovery", "CloudRecordings_ckAssets", "EncryptedCloudRecordings"];

const BUFFER: usize = 64 * 1024;

pub struct VoiceMemosStore {
    recordings: RootDir,
}

fn recorder_error(path: &Path, error: ContainedError) -> RecorderError {
    match error {
        ContainedError::Escape { .. } | ContainedError::NotADirectory(_) => RecorderError::Escape(path.to_path_buf()),
        ContainedError::NotRegular(leaf) => RecorderError::NotRegular(leaf),
        ContainedError::Io { kind: std::io::ErrorKind::NotFound, .. } => RecorderError::NotFound(path.to_path_buf()),
        ContainedError::Io { kind, .. } => RecorderError::Io(format!("{kind} at {}", path.display())),
    }
}

fn file_time(secs: i64, nanos: i64) -> FileTime {
    FileTime { secs, nanos: u32::try_from(nanos).unwrap_or(0) }
}

impl VoiceMemosStore {
    pub fn open(recordings_dir: &Path) -> Result<VoiceMemosStore, ContainedError> {
        Ok(VoiceMemosStore { recordings: RootDir::open(recordings_dir)? })
    }

    pub fn recordings_dir(&self) -> &Path {
        self.recordings.path()
    }

    /// The candidate at one validated name, `None` when the name is not an
    /// `.m4a` file.
    fn candidate_named(&self, name: &str) -> Result<Option<Candidate>, RecorderError> {
        if !name.ends_with(".m4a") {
            return Ok(None);
        }
        let path = self.recordings.path().join(name);
        let stat = self.recordings.stat(Path::new(name)).map_err(|error| recorder_error(&path, error))?;
        match stat.kind {
            Kind::File => Ok(Some(Candidate {
                path,
                file_name: name.to_owned(),
                size: stat.size,
                mtime: FileTime { secs: stat.mtime_secs, nanos: stat.mtime_nanos },
                flags: stat.flags,
            })),
            Kind::Link => Err(RecorderError::NotRegular(path)),
            Kind::Directory | Kind::Other => Ok(None),
        }
    }
}

impl RecorderStore for VoiceMemosStore {
    type Handle = File;

    fn candidates(&self) -> Result<Vec<Candidate>, RecorderError> {
        let names = self.recordings.names().map_err(|error| match error {
            ContainedError::Escape { .. } | ContainedError::NotADirectory(_) | ContainedError::NotRegular(_) => RecorderError::Escape(self.recordings.path().to_path_buf()),
            other => RecorderError::Unreadable(format!("{other:?}")),
        })?;
        let mut candidates = Vec::new();
        for name in names {
            match self.candidate_named(&name) {
                Ok(Some(candidate)) => candidates.push(candidate),
                Ok(None) | Err(RecorderError::NotRegular(_)) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(candidates)
    }

    fn candidate(&self, path: &Path) -> Result<Candidate, RecorderError> {
        let leaf = self.recordings.leaf(path).map_err(|error| recorder_error(path, error))?;
        let name = leaf.file_name().and_then(|name| name.to_str()).ok_or_else(|| RecorderError::Escape(path.to_path_buf()))?;
        self.candidate_named(name)?.ok_or_else(|| RecorderError::NotFound(path.to_path_buf()))
    }

    fn open(&self, path: &Path) -> Result<File, RecorderError> {
        self.recordings.open_file(path, Access::Read).map_err(|error| recorder_error(path, error))
    }

    fn metadata(&self, handle: &File) -> Result<SourceMetadata, RecorderError> {
        let metadata = handle.metadata().map_err(|error| RecorderError::Io(error.kind().to_string()))?;
        Ok(SourceMetadata {
            device: metadata.dev(),
            inode: metadata.ino(),
            size: metadata.len(),
            mtime: file_time(metadata.mtime(), metadata.mtime_nsec()),
        })
    }

    fn read_at(&self, handle: &File, offset: u64, buf: &mut [u8]) -> Result<(), ReadFailure> {
        handle.read_exact_at(buf, offset).map_err(|_| ReadFailure)
    }

    fn clone_into(&self, handle: &File, directory: &Path, directory_identity: (u64, u64), name: &str) -> Result<CloneKind, CloneError> {
        let expected_path = directory.to_path_buf();
        let directory = RootDir::open(directory).map_err(|error| clone_path_error(&expected_path, error))?;
        let observed = directory.identity().map_err(|error| clone_path_error(&expected_path, error))?;
        if observed != directory_identity {
            return Err(CloneError::Escape(expected_path));
        }
        let target = directory.c_name(Path::new(name)).map_err(|error| clone_path_error(&expected_path.join(name), error))?;
        // SAFETY: `target` is NUL-terminated and outlives the call; both descriptors
        // stay open for the lifetime of their owners.
        let outcome = unsafe { libc::fclonefileat(handle.as_raw_fd(), directory.as_fd().as_raw_fd(), target.as_ptr(), 0) };
        if outcome == 0 {
            return Ok(CloneKind::CopyOnWrite);
        }
        let error = std::io::Error::last_os_error();
        match error.raw_os_error() {
            Some(libc::EXDEV) => byte_copy(handle, &directory, Path::new(name)),
            Some(libc::EEXIST) => Err(CloneError::Exists),
            Some(libc::ENOSPC) => Err(CloneError::NoSpace),
            _ => Err(CloneError::Io(error.kind().to_string())),
        }
    }

    fn subdirectory_counts(&self) -> Vec<(String, Option<u64>)> {
        APPLE_SUBDIRECTORIES
            .iter()
            .map(|name| {
                let count = self.recordings.open_subdirectory(name).ok().and_then(|directory| directory.names().ok()).map(|entries| entries.len() as u64);
                ((*name).to_owned(), count)
            })
            .collect()
    }
}

fn clone_path_error(path: &Path, error: ContainedError) -> CloneError {
    match error {
        ContainedError::Escape { .. } | ContainedError::NotRegular(_) | ContainedError::NotADirectory(_) => CloneError::Escape(path.to_path_buf()),
        ContainedError::Io { kind, .. } => CloneError::Io(kind.to_string()),
    }
}

/// The `EXDEV` fallback: a private 0600 file below the directory, filled in
/// 64 KiB bounded reads from the descriptor.
fn byte_copy(source: &File, directory: &RootDir, name: &Path) -> Result<CloneKind, CloneError> {
    let mut out = directory.create_file(name, 0o600).map_err(|error| match error {
        ContainedError::Io { kind: std::io::ErrorKind::StorageFull, .. } => CloneError::NoSpace,
        ContainedError::Io { kind: std::io::ErrorKind::AlreadyExists, .. } => CloneError::Exists,
        other => clone_path_error(&directory.path().join(name), other),
    })?;
    let total = source.metadata().map_err(|error| CloneError::Io(error.kind().to_string()))?.len();
    let mut buffer = vec![0u8; BUFFER];
    let mut offset = 0u64;
    while offset < total {
        let chunk = usize::try_from((total - offset).min(BUFFER as u64)).unwrap_or(BUFFER);
        source.read_exact_at(&mut buffer[..chunk], offset).map_err(|error| CloneError::Io(error.kind().to_string()))?;
        out.write_all(&buffer[..chunk]).map_err(|error| {
            if error.kind() == std::io::ErrorKind::StorageFull { CloneError::NoSpace } else { CloneError::Io(error.kind().to_string()) }
        })?;
        offset += chunk as u64;
    }
    Ok(CloneKind::ByteCopy)
}
```

`read_exact_at` is `std::os::unix::fs::FileExt`, whose offset arithmetic is checked in the standard
library. The destination directory is opened as its own descriptor and compared with the archive's
retained device and inode before cloning. `fclonefileat` names the new file relative to that checked
descriptor. A byte-copy destination that runs out of space at creation or during a write is `NoSpace`;
every other failure keeps its kind and no raw path text reaches the error.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo fmt --all`

Run: `cargo test -p vpt-adapters voice_memos`

Expected: 10 store tests PASS. Run `cargo clippy -p vpt-adapters --all-targets -- -D warnings` and expect
no warnings.

- [ ] **Step 5: Commit**

Run: `cargo fmt --all`

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(source): the Voice Memos store listed at depth one and read by descriptor"
```

______________________________________________________________________

### Task 16: The private title copy

**Files:**

- Create: `crates/vpt-adapters/src/voice_memos/titles/tests.rs`

- Create: `crates/vpt-adapters/src/voice_memos/titles.rs`

- Modify: `crates/vpt-adapters/src/voice_memos/mod.rs`, `crates/vpt-adapters/src/voice_memos/store.rs`,
  `crates/vpt-application/src/ports/recorder.rs`, `crates/vpt-application/src/ports/mod.rs`

**Interfaces:**

- Consumes: `RecorderStore`, `vpt_adapters::{Access, ContainedError, RootDir}`,
  `RootDir::try_clone(&self) -> Result<RootDir, ContainedError>`, and
  `RootDir::revalidate(&self) -> Result<(), ContainedError>`.

- Produces: `vpt_application::ports::TitleLookup::{Titled(String), Unavailable}` and, on `RecorderStore`,
  `fn title(&self, file_name: &str) -> TitleLookup`;
  `fn refresh_titles(&self) -> Result<(), RecorderError>`; `vpt_adapters::{TitleCopy, COPY_DIRECTORY}`
  with `TitleCopy::refresh(container: &Path, state_dir: &Path) -> TitleCopy` (copies the live database
  and its side files into `<state_dir>/title-copy`, then opens the copy),
  `TitleCopy::existing(state_dir: &Path) -> TitleCopy` (opens a copy already there, touching nothing),
  `fn title(&self, file_name: &str) -> TitleLookup`, `COPY_DIRECTORY: &str = "title-copy"`;
  `VoiceMemosStore::with_titles(self, state_dir: PathBuf, refresh: bool) -> VoiceMemosStore` (the state
  capability is retained at construction; `refresh_titles` replaces the copy once per non-dry sweep, and
  `title` reads that copy);
  `VoiceMemosStore::with_titles_root(self, state: &RootDir, refresh: bool) -> VoiceMemosStore` retains
  the same checked state root as the runtime lock and ledger. Ordinary copy and schema failures yield
  `Unavailable`. Containment failures during refresh return `RecorderError::Escape`; title lookup refuses
  an invalid copy as `Unavailable`.

- [ ] **Step 1: Write the failing tests**

Declare first: `crates/vpt-adapters/src/voice_memos/mod.rs` gains `mod titles;` and
`pub use titles::{COPY_DIRECTORY, TitleCopy};`; the adapters `lib.rs` adds
`pub use voice_memos::{COPY_DIRECTORY, TitleCopy};`; `crates/vpt-application/src/ports/mod.rs` adds
`TitleLookup` to the recorder re-export. `crates/vpt-adapters/src/voice_memos/titles.rs` starts as its
test module alone:

```rust
#[cfg(test)]
mod tests;
```

`crates/vpt-adapters/src/voice_memos/titles/tests.rs`:

```rust
use super::*;
use crate::voice_memos::VoiceMemosStore;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use vpt_application::ports::RecorderStore;

fn apple_store(rows: &[(&str, &str)], with_columns: bool) -> (tempfile::TempDir, PathBuf, rusqlite::Connection) {
    let temp = tempfile::tempdir().expect("temp");
    let container = temp.path().canonicalize().expect("canonical");
    std::fs::create_dir_all(container.join("Recordings")).expect("recordings");
    let database = rusqlite::Connection::open(container.join("CloudRecordings.db")).expect("db");
    database.pragma_update(None, "journal_mode", "WAL").expect("wal");
    if with_columns {
        database
            .execute_batch("CREATE TABLE ZCLOUDRECORDING (Z_PK INTEGER PRIMARY KEY, ZPATH TEXT, ZCUSTOMLABEL TEXT)")
            .expect("schema");
        for (path, label) in rows {
            database
                .execute("INSERT INTO ZCLOUDRECORDING (ZPATH, ZCUSTOMLABEL) VALUES (?1, ?2)", [path, label])
                .expect("row");
        }
    } else {
        database.execute_batch("CREATE TABLE ZSOMETHING (Z_PK INTEGER PRIMARY KEY, ZNAME TEXT)").expect("schema");
    }
    (temp, container, database)
}

fn state() -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().expect("state temp");
    let state = temp.path().canonicalize().expect("canonical").join("state");
    std::fs::create_dir(&state).expect("state dir");
    (temp, state)
}

fn names_in(directory: &Path) -> Vec<std::ffi::OsString> {
    let mut names: Vec<_> = std::fs::read_dir(directory).expect("dir").map(|e| e.expect("entry").file_name()).collect();
    names.sort();
    names
}

#[test]
fn the_title_is_read_from_the_copy_by_file_name_including_rows_still_in_the_live_wal() {
    let (_temp, container, _database) = apple_store(&[("20260824 144736-4F3AB19C.m4a", "Invoice call")], true);
    let (_state_temp, state) = state();
    let wal = std::fs::metadata(container.join("CloudRecordings.db-wal")).expect("live wal");
    assert!(wal.len() > 0, "the fixture rows must still sit in the WAL");
    let titles = TitleCopy::refresh(&container, &state);
    assert_eq!(titles.title("20260824 144736-4F3AB19C.m4a"), TitleLookup::Titled("Invoice call".into()));
    assert_eq!(titles.title("other.m4a"), TitleLookup::Unavailable);
}

#[test]
fn the_copy_lives_under_the_state_directory_with_restrictive_modes() {
    let (_temp, container, _database) = apple_store(&[("a.m4a", "A")], true);
    let (_state_temp, state) = state();
    TitleCopy::refresh(&container, &state);
    let copy_dir = state.join(COPY_DIRECTORY);
    assert_eq!(std::fs::metadata(&copy_dir).expect("dir").permissions().mode() & 0o777, 0o700);
    for name in ["CloudRecordings.db", "CloudRecordings.db-wal", "CloudRecordings.db-shm"] {
        let path = copy_dir.join(name);
        assert!(path.exists(), "{name} missing");
        assert_eq!(std::fs::metadata(&path).expect("file").permissions().mode() & 0o777, 0o600);
    }
}

#[test]
fn a_second_refresh_overwrites_the_copy_in_place_and_removes_nothing() {
    let (_temp, container, database) = apple_store(&[("a.m4a", "A")], true);
    let (_state_temp, state) = state();
    assert_eq!(TitleCopy::refresh(&container, &state).title("a.m4a"), TitleLookup::Titled("A".into()));
    database.execute("UPDATE ZCLOUDRECORDING SET ZCUSTOMLABEL = 'B'", []).expect("update");
    std::fs::write(state.join(COPY_DIRECTORY).join("stray"), b"kept").expect("stray");
    assert_eq!(TitleCopy::refresh(&container, &state).title("a.m4a"), TitleLookup::Titled("B".into()));
    assert_eq!(std::fs::read(state.join(COPY_DIRECTORY).join("stray")).expect("kept"), b"kept");
}

#[test]
fn an_existing_copy_is_read_without_refreshing_and_an_absent_one_is_unavailable() {
    let (_temp, container, database) = apple_store(&[("a.m4a", "A")], true);
    let (_state_temp, state) = state();
    TitleCopy::refresh(&container, &state);
    database.execute("UPDATE ZCLOUDRECORDING SET ZCUSTOMLABEL = 'B'", []).expect("update");
    assert_eq!(TitleCopy::existing(&state).title("a.m4a"), TitleLookup::Titled("A".into()));
    let (_fresh_temp, fresh) = self::state();
    assert_eq!(TitleCopy::existing(&fresh).title("a.m4a"), TitleLookup::Unavailable);
    assert_eq!(names_in(&fresh), Vec::<std::ffi::OsString>::new());
}

#[test]
fn a_changed_schema_yields_unavailable() {
    let (_temp, container, _database) = apple_store(&[], false);
    let (_state_temp, state) = state();
    assert_eq!(TitleCopy::refresh(&container, &state).title("a.m4a"), TitleLookup::Unavailable);
}

#[test]
fn a_missing_live_database_yields_unavailable_and_makes_no_copy() {
    let temp = tempfile::tempdir().expect("temp");
    let container = temp.path().canonicalize().expect("canonical");
    std::fs::create_dir_all(container.join("Recordings")).expect("recordings");
    let (_state_temp, state) = state();
    assert_eq!(TitleCopy::refresh(&container, &state).title("a.m4a"), TitleLookup::Unavailable);
    assert!(!state.join(COPY_DIRECTORY).exists());
}

#[test]
fn the_live_container_is_left_with_no_new_entries() {
    let (_temp, container, _database) = apple_store(&[("a.m4a", "A")], true);
    let (_state_temp, state) = state();
    let before = names_in(&container);
    TitleCopy::refresh(&container, &state);
    assert_eq!(names_in(&container), before);
}

#[test]
fn like_metacharacters_in_a_file_name_match_only_that_name() {
    let rows = [
        ("dir/axb.m4a", "x"),
        ("dir/a_b.m4a", "underscore"),
        ("dir/100x.m4a", "x too"),
        ("dir/100%.m4a", "percent"),
        ("dir/a\\b.m4a", "backslash"),
    ];
    let (_temp, container, _database) = apple_store(&rows, true);
    let (_state_temp, state) = state();
    let titles = TitleCopy::refresh(&container, &state);
    assert_eq!(titles.title("a_b.m4a"), TitleLookup::Titled("underscore".into()));
    assert_eq!(titles.title("100%.m4a"), TitleLookup::Titled("percent".into()));
    assert_eq!(titles.title("a\\b.m4a"), TitleLookup::Titled("backslash".into()));
    assert_eq!(titles.title("b.m4a"), TitleLookup::Unavailable);
}

#[test]
fn the_store_answers_through_its_port_and_unavailable_without_titles() {
    let (_temp, container, _database) = apple_store(&[("a.m4a", "A")], true);
    let (_state_temp, state) = state();
    let store = VoiceMemosStore::open(&container.join("Recordings")).expect("store");
    assert_eq!(store.title("a.m4a"), TitleLookup::Unavailable);
    let titled = VoiceMemosStore::open(&container.join("Recordings")).expect("store").with_titles(state.clone(), true);
    titled.refresh_titles().expect("refresh");
    assert_eq!(titled.title("a.m4a"), TitleLookup::Titled("A".into()));
    assert!(state.join(COPY_DIRECTORY).join("CloudRecordings.db").exists());
}
#[test]
fn a_missing_title_state_is_unavailable_without_refusing_the_recorder() {
    let (_temp, container, _database) = apple_store(&[("a.m4a", "A")], true);
    let (_state_temp, state) = state();
    let store = VoiceMemosStore::open(&container.join("Recordings")).expect("store")
        .with_titles(state.join("absent"), true);
    assert_eq!(store.refresh_titles(), Ok(()));
    assert_eq!(store.title("a.m4a"), TitleLookup::Unavailable);
    assert!(names_in(&state).is_empty());
}

#[test]
fn retained_title_state_refuses_replacement_before_writing_a_copy() {
    let (_temp, container, _database) = apple_store(&[("a.m4a", "A")], true);
    let (_state_temp, state) = state();
    let root = RootDir::open(&state).expect("root");
    let store = VoiceMemosStore::open(&container.join("Recordings")).expect("store").with_titles_root(&root, true);
    std::fs::rename(&state, state.with_extension("saved")).expect("move state");
    std::fs::create_dir(&state).expect("replacement state");
    assert_eq!(store.refresh_titles(), Err(vpt_application::ports::RecorderError::Escape(state.clone())));
    assert!(names_in(&state).is_empty());
}

#[test]
fn every_copied_database_leaf_is_checked_before_refresh_or_query() {
    for name in SIDE_FILES {
        let (_temp, container, _database) = apple_store(&[("a.m4a", "A")], true);
        let (state_temp, state) = state();
        let root = RootDir::open(&state).expect("root");
        let store = VoiceMemosStore::open(&container.join("Recordings")).expect("store").with_titles_root(&root, true);
        store.refresh_titles().expect("first refresh");
        let copy = state.join(COPY_DIRECTORY);
        let target = state_temp.path().join("outside");
        std::fs::write(&target, b"private outside data").expect("outside");
        std::fs::rename(copy.join(name), copy.join(format!("{name}.saved"))).expect("move leaf");
        std::os::unix::fs::symlink(&target, copy.join(name)).expect("link leaf");
        assert_eq!(store.title("a.m4a"), TitleLookup::Unavailable);
        assert_eq!(store.refresh_titles(), Err(vpt_application::ports::RecorderError::Escape(state.clone())));
        assert_eq!(std::fs::read(&target).expect("outside untouched"), b"private outside data");
    }
}

#[test]
fn an_open_title_copy_refuses_a_replaced_copy_directory() {
    let (_temp, container, _database) = apple_store(&[("a.m4a", "A")], true);
    let (_state_temp, state) = state();
    let titles = TitleCopy::refresh(&container, &state);
    let copy = state.join(COPY_DIRECTORY);
    std::fs::rename(&copy, state.join("saved-copy")).expect("move copy");
    std::fs::create_dir(&copy).expect("replacement");
    assert_eq!(titles.title("a.m4a"), TitleLookup::Unavailable);
}

```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-adapters titles`

Expected: the build fails with `cannot find` for `TitleCopy`, `COPY_DIRECTORY` and `TitleLookup`, and
`no method named title` on `VoiceMemosStore`.

- [ ] **Step 3: Write the minimal implementation**

Append to `crates/vpt-application/src/ports/recorder.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TitleLookup {
    Titled(String),
    Unavailable,
}
```

and add to `RecorderStore`:

```rust
    /// The human title from the private database copy; a copy that cannot be
    /// made or read answers `Unavailable`.
    fn title(&self, file_name: &str) -> TitleLookup;
    fn refresh_titles(&self) -> Result<(), RecorderError>;
```

`crates/vpt-adapters/src/voice_memos/titles.rs`, above its test module:

```rust
//! The database is confined to one thing, the human title, and it is read
//! from a private copy so SQLite never creates a journal inside Apple's
//! directory. The copy is overwritten in place and never removed.

use crate::contained::{Access, ContainedError, RootDir};
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use std::fs::File;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use vpt_application::ports::TitleLookup;

pub const COPY_DIRECTORY: &str = "title-copy";
const DATABASE: &str = "CloudRecordings.db";
const SIDE_FILES: [&str; 3] = ["CloudRecordings.db", "CloudRecordings.db-wal", "CloudRecordings.db-shm"];

pub struct TitleCopy {
    connection: Option<Connection>,
    table: Option<String>,
    root: Option<RootDir>,
}

impl TitleCopy {
    fn unavailable() -> TitleCopy {
        TitleCopy { connection: None, table: None, root: None }
    }

    pub fn refresh(container: &Path, state_dir: &Path) -> TitleCopy {
        RootDir::open(state_dir).and_then(|state| Self::refresh_at(container, &state))
            .unwrap_or_else(|_| Self::unavailable())
    }

    pub(crate) fn refresh_at(container: &Path, state: &RootDir) -> Result<TitleCopy, ContainedError> {
        let result = state.revalidate().and_then(|()| copy_database(container, state)).and_then(Self::open_copy);
        match result {
            Ok(copy) => Ok(copy),
            Err(error @ (ContainedError::Escape { .. } | ContainedError::NotRegular(_) | ContainedError::NotADirectory(_))) => Err(error),
            Err(_) => Ok(Self::unavailable()),
        }
    }

    pub fn existing(state_dir: &Path) -> TitleCopy {
        RootDir::open(state_dir).and_then(|state| state.open_subdirectory(COPY_DIRECTORY))
            .and_then(Self::open_copy).unwrap_or_else(|_| Self::unavailable())
    }

    fn open_copy(copy: RootDir) -> Result<TitleCopy, ContainedError> {
        let database = checked_database(&copy)?;
        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX | OpenFlags::SQLITE_OPEN_NOFOLLOW;
        let connection = Connection::open_with_flags(database, flags).ok();
        let table = connection.as_ref().and_then(table_with_title_columns);
        Ok(TitleCopy { connection, table, root: Some(copy) })
    }

    pub fn title(&self, file_name: &str) -> TitleLookup {
        if self.root.as_ref().is_none_or(|root| checked_database(root).is_err()) {
            return TitleLookup::Unavailable;
        }
        let (Some(connection), Some(table)) = (&self.connection, &self.table) else {
            return TitleLookup::Unavailable;
        };
        let query = format!(
            "SELECT ZCUSTOMLABEL FROM \"{}\" WHERE ZPATH = ?1 OR ZPATH LIKE ?2 ESCAPE '\\' LIMIT 1",
            table.replace('"', "\"\"")
        );
        let suffix = format!("%/{}", file_name.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
        let label: Option<Option<String>> = connection
            .query_row(&query, [file_name, suffix.as_str()], |row| row.get(0))
            .optional()
            .ok()
            .flatten();
        match label {
            Some(Some(title)) if !title.trim().is_empty() => TitleLookup::Titled(title),
            _ => TitleLookup::Unavailable,
        }
    }
}

fn checked_database(copy: &RootDir) -> Result<PathBuf, ContainedError> {
    copy.revalidate()?;
    let database = copy.regular(Path::new(DATABASE))?;
    for name in ["CloudRecordings.db-wal", "CloudRecordings.db-shm"] {
        match copy.regular(Path::new(name)) {
            Ok(_) | Err(ContainedError::Io { kind: std::io::ErrorKind::NotFound, .. }) => {}
            Err(error) => return Err(error),
        }
    }
    Ok(database)
}

fn copy_database(container: &Path, state: &RootDir) -> Result<RootDir, ContainedError> {
    let live = RootDir::open(container)?;
    live.regular(Path::new(DATABASE))?;
    state.revalidate()?;
    let copy = state.subdirectory(COPY_DIRECTORY)?;
    for name in SIDE_FILES {
        match copy.regular(Path::new(name)) {
            Ok(_) | Err(ContainedError::Io { kind: std::io::ErrorKind::NotFound, .. }) => {}
            Err(error) => return Err(error),
        }
    }
    for name in SIDE_FILES {
        let mut target = overwrite(&copy, name)?;
        match live.open_file(Path::new(name), Access::Read) {
            Ok(mut source) => {
                std::io::copy(&mut source, &mut target)
                    .map_err(|error| ContainedError::Io { path: copy.path().join(name), kind: error.kind() })?;
            }
            Err(ContainedError::Io { kind: std::io::ErrorKind::NotFound, .. }) => {}
            Err(error) => return Err(error),
        }
    }
    copy.revalidate()?;
    Ok(copy)
}

/// The copy's file at `name`: created privately, or truncated in place and
/// repaired to 0600 when it is already there.
fn overwrite(copy: &RootDir, name: &str) -> Result<File, ContainedError> {
    match copy.create_file(Path::new(name), 0o600) {
        Err(ContainedError::Io { kind: std::io::ErrorKind::AlreadyExists, .. }) => {
            let file = copy.open_file(Path::new(name), Access::Write)?;
            let failed = |error: std::io::Error| ContainedError::Io { path: copy.path().join(name), kind: error.kind() };
            file.set_len(0).map_err(failed)?;
            file.set_permissions(std::fs::Permissions::from_mode(0o600)).map_err(failed)?;
            Ok(file)
        }
        outcome => outcome,
    }
}

/// The table holding both `ZPATH` and `ZCUSTOMLABEL`, found by inspection so a
/// renamed table degrades to untitled rather than to a wrong guess.
fn table_with_title_columns(connection: &Connection) -> Option<String> {
    let mut statement = connection.prepare("SELECT name FROM sqlite_master WHERE type = 'table'").ok()?;
    let names: Vec<String> = statement.query_map([], |row| row.get(0)).ok()?.filter_map(Result::ok).collect();
    names.into_iter().find(|name| {
        let Ok(mut columns) = connection.prepare(&format!("PRAGMA table_info(\"{}\")", name.replace('"', "\"\""))) else {
            return false;
        };
        let found: Vec<String> = columns
            .query_map([], |row| row.get::<_, String>(1))
            .ok()
            .map(|rows| rows.filter_map(Result::ok).collect())
            .unwrap_or_default();
        found.iter().any(|c| c == "ZPATH") && found.iter().any(|c| c == "ZCUSTOMLABEL")
    })
}
```

The `LIKE` pattern escapes the escape character itself as well as `%` and `_`, and the query names `\` as
that character, so a file name is matched only as itself. A live side file that is absent leaves its copy
truncated to zero length, which is what SQLite expects beside a database with no journal.

In `crates/vpt-adapters/src/voice_memos/store.rs`, the store gains its titles:

```rust
use super::titles::TitleCopy;
use std::cell::RefCell;
use std::path::PathBuf;
use vpt_application::ports::TitleLookup;

/// Where the private copy lives and whether this run may refresh it.
struct Titles {
    state_dir: PathBuf,
    state: Result<RootDir, ContainedError>,
    refresh: bool,
    copy: RefCell<Option<TitleCopy>>,
}

pub struct VoiceMemosStore {
    recordings: RootDir,
    titles: Option<Titles>,
}
```

with `open` setting `titles: None`, this builder:

```rust
    pub fn with_titles(mut self, state_dir: PathBuf, refresh: bool) -> VoiceMemosStore {
        let state = RootDir::open(&state_dir);
        self.titles = Some(Titles { state_dir, state, refresh, copy: RefCell::new(None) });
        self
    }

    pub fn with_titles_root(mut self, state: &RootDir, refresh: bool) -> VoiceMemosStore {
        self.titles = Some(Titles {
            state_dir: state.path().to_path_buf(), state: state.try_clone(),
            refresh, copy: RefCell::new(None),
        });
        self
    }
```

and the port method in `impl RecorderStore for VoiceMemosStore`:

```rust
    fn title(&self, file_name: &str) -> TitleLookup {
        self.titles.as_ref().and_then(|titles| titles.copy.borrow().as_ref().map(|copy| copy.title(file_name)))
            .unwrap_or(TitleLookup::Unavailable)
    }

    fn refresh_titles(&self) -> Result<(), RecorderError> {
        let Some(titles) = &self.titles else { return Ok(()); };
        if !titles.refresh { return Ok(()); }
        *titles.copy.borrow_mut() = None;
        let state = match &titles.state {
            Ok(state) => state,
            Err(ContainedError::Escape { .. } | ContainedError::NotRegular(_) | ContainedError::NotADirectory(_)) => {
                return Err(RecorderError::Escape(titles.state_dir.clone()));
            }
            Err(_) => return Ok(()),
        };
        self.recordings.revalidate().map_err(|error| recorder_error(self.recordings.path(), error))?;
        let container = self.recordings.path().parent().unwrap_or(self.recordings.path());
        let copy = TitleCopy::refresh_at(container, state)
            .map_err(|_| RecorderError::Escape(titles.state_dir.clone()))?;
        *titles.copy.borrow_mut() = Some(copy);
        Ok(())
    }
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo fmt --all`

Run: `cargo test -p vpt-adapters voice_memos`

Expected: the 10 store tests and the 13 title tests PASS. Run
`cargo clippy -p vpt-adapters --all-targets -- -D warnings` and expect no warnings.

- [ ] **Step 5: Commit**

Run: `cargo fmt --all`

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(source): titles from a private copy of the Voice Memos database"
```

______________________________________________________________________

### Task 17: The archive: staging clones and digests

The archive holds the `audio` store as a root descriptor. Staging is the archive's operation: it chooses
a private name below its root and hands the root's path, device/inode identity and that name to a clone
closure the use case builds from the recorder port, so the archive port never names the recorder's handle
type and neither port names a file type. A staging failure says whether this invocation created the
staged file, because every ingest exit before publication trashes what it owns and nothing else.

**Files:**

- Create: `crates/vpt-adapters/src/archive/tests.rs`

- Create: `crates/vpt-application/src/ports/archive.rs`

- Modify: `crates/vpt-application/src/ports/mod.rs`

- Create: `crates/vpt-adapters/src/archive/mod.rs`

- Modify: `crates/vpt-adapters/src/lib.rs`

**Interfaces:**

- Consumes: `CloneKind`, `CloneError`, `vpt_domain::container::ReadFailure`, `vpt_adapters::digest_open`,
  `vpt_adapters::{Access, ContainedError, Kind, RootDir}`.

- Produces, in `vpt_application::ports` (from the private file `ports/archive.rs`):
  `Staged { pub path: PathBuf, pub digest: Sha256Digest, pub size: u64, pub copy_on_write: bool }`;
  `ArchiveError::{NoSpace, Sync(String), PlacedUnsynced(PathBuf), Escape(PathBuf), Io(String)}`
  (`PlacedUnsynced` is Task 18's directory sync failing after the target was placed: no row is recorded
  and the next sweep recovers the file);
  `StageFailure { pub cause: ArchiveError, pub owned_staging: Option<PathBuf> }` (`owned_staging` is set
  only when this invocation created the staged file; a name that was already taken is never claimed);
  `trait Archive { type Handle; fn stage<C>(&self, clone: C) -> Result<Staged, StageFailure>`
  `where C: FnOnce(&Path, (u64, u64), &str) -> Result<CloneKind, CloneError>;`
  `fn open(&self, path: &Path) -> Result<Self::Handle, ArchiveError>;`
  `fn size(&self, handle: &Self::Handle) -> Result<u64, ArchiveError>;`
  `fn read_at(&self, handle: &Self::Handle, offset: u64, buf: &mut [u8]) -> Result<(), ReadFailure>;`
  `fn validate_trash_path(&self, path: &Path) -> Result<(), ArchiveError>;`
  `fn digest(&self, path: &Path) -> Result<Sha256Digest, ArchiveError>; fn target(&self, name: &str)`
  `-> PathBuf; fn archived(&self) -> Result<Vec<PathBuf>, ArchiveError>;`
  `fn staged_leftovers(&self) -> Result<Vec<PathBuf>, ArchiveError>; }` (Task 18 adds `publish` and
  `sync_existing`); `vpt_adapters::{ClonefileArchive, STAGING_PREFIX}` with
  `ClonefileArchive::open(audio_store: &Path) -> Result<ClonefileArchive, ContainedError>`,
  `ClonefileArchive::open_read_only(audio_store: &Path) -> Result<ClonefileArchive, ContainedError>` (an
  absent root yields empty listings and stays absent), `type Handle = std::fs::File`,
  `STAGING_PREFIX: &str = ".vpt-staging-"`.

- [ ] **Step 1: Write the failing tests**

Declare first: `crates/vpt-application/src/ports/mod.rs` gains `mod archive;` and
`pub use archive::{Archive, ArchiveError, StageFailure, Staged};`; `crates/vpt-adapters/src/lib.rs` gains
`mod archive;` and `pub use archive::{ClonefileArchive, STAGING_PREFIX};`.
`crates/vpt-adapters/src/archive/mod.rs` starts with the registered test child below. Create the child
before the red run:

```rust
#[cfg(test)]
mod tests;
```

`crates/vpt-adapters/src/archive/tests.rs`:

```rust
use super::*;
use crate::voice_memos::VoiceMemosStore;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use vpt_application::ports::{CloneError, RecorderStore};
use vpt_domain::container::inspect;
use vpt_domain::fixtures::m4a;

fn source(bytes: &[u8]) -> (tempfile::TempDir, PathBuf, VoiceMemosStore, File) {
    let temp = tempfile::tempdir().expect("temp");
    let base = temp.path().canonicalize().expect("canonical");
    std::fs::create_dir_all(base.join("Recordings")).expect("recordings");
    std::fs::create_dir_all(base.join("audio")).expect("audio");
    std::fs::write(base.join("Recordings/a.m4a"), bytes).expect("fixture");
    let store = VoiceMemosStore::open(&base.join("Recordings")).expect("store");
    let handle = store.open(&base.join("Recordings/a.m4a")).expect("open");
    (temp, base.join("audio"), store, handle)
}

#[test]
fn staging_produces_a_0600_private_copy_with_the_bytes_digest_and_size() {
    let bytes = m4a(1_787_604_456, 5, b"payload");
    let (_temp, audio, store, handle) = source(&bytes);
    let archive = ClonefileArchive::open(&audio).expect("archive");

    let staged = archive.stage(|directory, directory_identity, name| store.clone_into(&handle, directory, directory_identity, name)).expect("staged");

    assert_eq!(staged.path.parent(), Some(audio.as_path()));
    assert!(staged.path.file_name().expect("name").to_string_lossy().starts_with(STAGING_PREFIX));
    assert_eq!(std::fs::read(&staged.path).expect("read"), bytes);
    assert_eq!(std::fs::metadata(&staged.path).expect("meta").permissions().mode() & 0o777, 0o600);
    assert_eq!(staged.size, bytes.len() as u64);
    assert_eq!(archive.digest(&staged.path).expect("digest"), staged.digest);
    assert!(staged.copy_on_write);
}

#[test]
fn a_staged_file_can_be_opened_for_the_wholeness_gate() {
    let (_temp, audio, store, handle) = source(&m4a(1_787_604_456, 5, b"payload"));
    let archive = ClonefileArchive::open(&audio).expect("archive");
    let staged = archive.stage(|directory, directory_identity, name| store.clone_into(&handle, directory, directory_identity, name)).expect("staged");
    let opened = archive.open(&staged.path).expect("open");
    let len = archive.size(&opened).expect("size");
    let container = inspect(len, |offset, buf: &mut [u8]| archive.read_at(&opened, offset, buf)).expect("whole");
    assert_eq!(container.duration_secs, 5);
}

#[test]
fn a_clone_that_created_nothing_owns_no_staging_name() {
    let (_temp, audio, _store, _handle) = source(b"x");
    let archive = ClonefileArchive::open(&audio).expect("archive");
    let failure = archive.stage(|_directory, _directory_identity, _name| Err(CloneError::Io("boom".into()))).unwrap_err();
    assert_eq!(failure.cause, ArchiveError::Io("boom".into()));
    assert_eq!(failure.owned_staging, None);
    assert!(archive.staged_leftovers().expect("list").is_empty());
}

#[test]
fn a_failure_after_the_file_was_created_owns_it_at_mode_0600() {
    let (_temp, audio, _store, _handle) = source(b"x");
    let archive = ClonefileArchive::open(&audio).expect("archive");
    let failure = archive
        .stage(|directory, _directory_identity, name| {
            std::fs::write(directory.join(name), b"partial").expect("partial");
            Err(CloneError::NoSpace)
        })
        .unwrap_err();
    assert_eq!(failure.cause, ArchiveError::NoSpace);
    let owned = failure.owned_staging.expect("owned");
    assert_eq!(owned.parent(), Some(audio.as_path()));
    assert_eq!(std::fs::metadata(&owned).expect("kept").permissions().mode() & 0o777, 0o600);
    assert_eq!(archive.staged_leftovers().expect("list"), vec![owned]);
}

#[test]
fn a_name_that_was_already_taken_is_never_claimed() {
    let (_temp, audio, _store, _handle) = source(b"x");
    let archive = ClonefileArchive::open(&audio).expect("archive");
    let failure = archive
        .stage(|directory, _directory_identity, name| {
            std::fs::write(directory.join(name), b"someone else's").expect("collision");
            Err(CloneError::Exists)
        })
        .unwrap_err();
    assert_eq!(failure.owned_staging, None);
    assert!(matches!(failure.cause, ArchiveError::Io(_)), "{:?}", failure.cause);
}

#[test]
fn archived_lists_only_m4a_files_and_leftovers_only_staging_names() {
    let (_temp, audio, _store, _handle) = source(b"x");
    std::fs::write(audio.join("2026-08-24T144736-4f3ab19c02de.m4a"), b"x").expect("archived");
    std::fs::write(audio.join(format!("{STAGING_PREFIX}123.m4a")), b"y").expect("leftover");
    std::fs::write(audio.join("notes.txt"), b"z").expect("other");
    std::os::unix::fs::symlink(audio.join("notes.txt"), audio.join("link.m4a")).expect("link");
    let archive = ClonefileArchive::open(&audio).expect("archive");
    assert_eq!(archive.archived().expect("list"), vec![audio.join("2026-08-24T144736-4f3ab19c02de.m4a")]);
    assert_eq!(archive.staged_leftovers().expect("list"), vec![audio.join(format!("{STAGING_PREFIX}123.m4a"))]);
    assert_eq!(archive.target("abc.m4a"), audio.join("abc.m4a"));
}

#[test]
fn open_and_digest_refuse_a_link_and_a_path_outside_the_store() {
    let (temp, audio, _store, _handle) = source(b"x");
    std::fs::write(audio.join("real.m4a"), b"x").expect("real");
    std::os::unix::fs::symlink(audio.join("real.m4a"), audio.join("link.m4a")).expect("link");
    let archive = ClonefileArchive::open(&audio).expect("archive");
    assert_eq!(archive.open(&audio.join("link.m4a")).err(), Some(ArchiveError::Escape(audio.join("link.m4a"))));
    let outside = temp.path().canonicalize().expect("canonical").join("Recordings/a.m4a");
    assert_eq!(archive.digest(&outside), Err(ArchiveError::Escape(outside)));
    let absent = archive.open(&audio.join("absent.m4a"));
    assert!(matches!(absent, Err(ArchiveError::Io(_))), "{absent:?}");
}

#[test]
fn reading_an_absent_archive_lists_nothing_and_never_creates_it() {
    let temp = tempfile::tempdir().expect("temp");
    let audio = temp.path().canonicalize().expect("canonical").join("audio");
    let archive = ClonefileArchive::open_read_only(&audio).expect("read only");
    assert!(archive.archived().expect("archives").is_empty());
    assert!(archive.staged_leftovers().expect("staging").is_empty());
    assert_eq!(archive.target("id.m4a"), audio.join("id.m4a"));
    let result = archive.stage(|_, _, _| panic!("an absent root cannot stage"));
    assert!(result.is_err());
    assert!(!audio.exists());
}
#[test]
fn a_root_replaced_during_staging_never_returns_a_cleanup_path() {
    let (_temp, audio, store, handle) = source(b"audio");
    let archive = ClonefileArchive::open(&audio).expect("archive");
    let saved = audio.with_extension("saved");
    let failure = archive.stage(|directory, identity, name| {
        let kind = store.clone_into(&handle, directory, identity, name)?;
        std::fs::rename(&audio, &saved).expect("move root");
        std::fs::create_dir(&audio).expect("replacement");
        std::fs::write(audio.join(name), b"unowned replacement").expect("unowned");
        Ok(kind)
    }).expect_err("root changed");
    assert_eq!(failure.cause, ArchiveError::Escape(audio.clone()));
    assert_eq!(failure.owned_staging, None);
    assert_eq!(std::fs::read_dir(&saved).expect("saved").count(), 1);
    assert_eq!(std::fs::read_dir(&audio).expect("replacement").count(), 1);
}

#[test]
fn cleanup_validation_refuses_links_and_replaced_archive_roots() {
    let (_temp, audio, _store, _handle) = source(b"audio");
    let archive = ClonefileArchive::open(&audio).expect("archive");
    let path = audio.join(".vpt-staging-test.m4a");
    std::fs::write(audio.join("unowned.m4a"), b"kept").expect("unowned");
    std::os::unix::fs::symlink(audio.join("unowned.m4a"), &path).expect("link");
    assert_eq!(archive.validate_trash_path(&path), Err(ArchiveError::Escape(path.clone())));
    std::fs::rename(&audio, audio.with_extension("saved")).expect("move root");
    std::fs::create_dir(&audio).expect("replacement");
    std::fs::write(&path, b"replacement").expect("replacement file");
    assert_eq!(archive.validate_trash_path(&path), Err(ArchiveError::Escape(audio)));
    assert_eq!(std::fs::read(path).expect("untouched"), b"replacement");
}

```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-adapters archive`

Expected: the build fails with `unresolved import` for the archive names in `ports/mod.rs` and
`cannot find` for `ClonefileArchive` and `STAGING_PREFIX`.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-application/src/ports/archive.rs`:

```rust
//! The audio archive: staging below its root, bounded reads, digests, and
//! (from Task 18) exclusive publication.

use super::recorder::{CloneError, CloneKind};
use std::path::{Path, PathBuf};
use vpt_domain::container::ReadFailure;
use vpt_domain::digest::Sha256Digest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Staged {
    pub path: PathBuf,
    pub digest: Sha256Digest,
    pub size: u64,
    pub copy_on_write: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchiveError {
    NoSpace,
    Sync(String),
    /// The target was placed and then its directory failed to sync: nothing is
    /// staged any more, no row is recorded, the next sweep recovers the file.
    PlacedUnsynced(PathBuf),
    /// Below no archive root, nested, or reached through a link: exit 3, `path_escape`.
    Escape(PathBuf),
    Io(String),
}

/// A staging failure, with the staged name when this invocation created it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageFailure {
    pub cause: ArchiveError,
    pub owned_staging: Option<PathBuf>,
}

pub trait Archive {
    /// The open read-only descriptor of one archive file.
    type Handle;

    /// Stage into a private name below the root: `clone` receives the root's
    /// path and the name and clones the source there.
    fn stage<C>(&self, clone: C) -> Result<Staged, StageFailure>
    where
        C: FnOnce(&Path, (u64, u64), &str) -> Result<CloneKind, CloneError>;
    fn open(&self, path: &Path) -> Result<Self::Handle, ArchiveError>;
    fn size(&self, handle: &Self::Handle) -> Result<u64, ArchiveError>;
    /// Fill `buf` from `offset`, or fail; the wholeness gate's read callback.
    fn read_at(&self, handle: &Self::Handle, offset: u64, buf: &mut [u8]) -> Result<(), ReadFailure>;
    fn digest(&self, path: &Path) -> Result<Sha256Digest, ArchiveError>;
    fn validate_trash_path(&self, path: &Path) -> Result<(), ArchiveError>;
    fn target(&self, name: &str) -> PathBuf;
    fn archived(&self) -> Result<Vec<PathBuf>, ArchiveError>;
    fn staged_leftovers(&self) -> Result<Vec<PathBuf>, ArchiveError>;
}
```

`crates/vpt-adapters/src/archive/mod.rs`, above its test module:

```rust
//! The archive on disk: staging from the source descriptor into a private
//! name below the `audio` root, digests in 64 KiB buffers.

use crate::contained::{Access, ContainedError, Kind, RootDir};
use crate::stores::digest_open;
use std::fs::File;
use std::os::unix::fs::{FileExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use vpt_application::ports::{Archive, ArchiveError, CloneError, CloneKind, StageFailure, Staged};
use vpt_domain::container::ReadFailure;
use vpt_domain::digest::Sha256Digest;

pub const STAGING_PREFIX: &str = ".vpt-staging-";

pub struct ClonefileArchive {
    directory: PathBuf,
    root: Option<RootDir>,
    sequence: AtomicU64,
}

pub(crate) fn io(error: &std::io::Error) -> ArchiveError {
    if error.kind() == std::io::ErrorKind::StorageFull { ArchiveError::NoSpace } else { ArchiveError::Io(error.kind().to_string()) }
}

pub(crate) fn archive_error(path: &Path, error: ContainedError) -> ArchiveError {
    match error {
        ContainedError::Io { kind: std::io::ErrorKind::StorageFull, .. } => ArchiveError::NoSpace,
        ContainedError::Io { kind, .. } => ArchiveError::Io(kind.to_string()),
        ContainedError::Escape { .. } | ContainedError::NotRegular(_) | ContainedError::NotADirectory(_) => {
            ArchiveError::Escape(path.to_path_buf())
        }
    }
}

fn clone_error(error: CloneError) -> ArchiveError {
    match error {
        CloneError::NoSpace => ArchiveError::NoSpace,
        CloneError::Exists => ArchiveError::Io("staging name already exists".into()),
        CloneError::Escape(path) => ArchiveError::Escape(path),
        CloneError::Io(detail) => ArchiveError::Io(detail),
    }
}

impl ClonefileArchive {
    pub fn open(audio_store: &Path) -> Result<ClonefileArchive, ContainedError> {
        Ok(ClonefileArchive { directory: audio_store.to_path_buf(), root: Some(RootDir::open(audio_store)?), sequence: AtomicU64::new(0) })
    }

    pub fn open_read_only(audio_store: &Path) -> Result<ClonefileArchive, ContainedError> {
        match Self::open(audio_store) {
            Ok(archive) => Ok(archive),
            Err(ContainedError::Io { kind: std::io::ErrorKind::NotFound, .. }) => {
                Ok(ClonefileArchive { directory: audio_store.to_path_buf(), root: None, sequence: AtomicU64::new(0) })
            }
            Err(error) => Err(error),
        }
    }

    fn root(&self) -> Result<&RootDir, ArchiveError> {
        let root = self.root.as_ref().ok_or_else(|| ArchiveError::Io("the audio store is absent".into()))?;
        root.revalidate().map_err(|_| ArchiveError::Escape(self.directory.clone()))?;
        Ok(root)
    }

    fn staging_name(&self) -> String {
        format!("{STAGING_PREFIX}{}-{}.m4a", std::process::id(), self.sequence.fetch_add(1, Ordering::Relaxed))
    }

    /// Mode 0600, digest and size of a file this invocation just staged.
    fn finish(&self, name: &str, kind: CloneKind) -> Result<Staged, ArchiveError> {
        let root = self.root()?;
        let path = root.path().join(name);
        let mut file = root.open_file(Path::new(name), Access::Read).map_err(|error| archive_error(&path, error))?;
        file.set_permissions(std::fs::Permissions::from_mode(0o600)).map_err(|error| io(&error))?;
        let digest = digest_open(&mut file).map_err(|error| io(&error))?;
        let size = file.metadata().map_err(|error| io(&error))?.len();
        Ok(Staged { path, digest, size, copy_on_write: kind == CloneKind::CopyOnWrite })
    }

    /// A staged file this invocation created and cannot use: keep it private
    /// for the owned cleanup and report it.
    fn owned_failure(&self, name: &str, cause: ArchiveError) -> StageFailure {
        if let Some(root) = &self.root
            && let Ok(file) = root.open_file(Path::new(name), Access::Read) {
            let _ = file.set_permissions(std::fs::Permissions::from_mode(0o600));
        }
        StageFailure { cause, owned_staging: Some(self.directory.join(name)) }
    }

    fn list(&self, keep: impl Fn(&str) -> bool) -> Result<Vec<PathBuf>, ArchiveError> {
        let Some(root) = &self.root else { return Ok(Vec::new()); };
        let names = root.names().map_err(|error| archive_error(root.path(), error))?;
        let mut paths = Vec::new();
        for name in names.into_iter().filter(|name| keep(name)) {
            if matches!(root.stat(Path::new(&name)), Ok(stat) if stat.kind == Kind::File) {
                paths.push(root.path().join(name));
            }
        }
        Ok(paths)
    }
}

impl Archive for ClonefileArchive {
    type Handle = File;

    fn stage<C>(&self, clone: C) -> Result<Staged, StageFailure>
    where
        C: FnOnce(&Path, (u64, u64), &str) -> Result<CloneKind, CloneError>,
    {
        let root = self.root().map_err(|cause| StageFailure { cause, owned_staging: None })?;
        let lost_root = || StageFailure { cause: ArchiveError::Escape(root.path().to_path_buf()), owned_staging: None };
        let name = self.staging_name();
        let identity = root.identity().map_err(|error| StageFailure {
            cause: archive_error(root.path(), error), owned_staging: None,
        })?;
        let result = match clone(root.path(), identity, &name) {
            Ok(kind) => self.finish(&name, kind).map_err(|cause| self.owned_failure(&name, cause)),
            Err(CloneError::Exists) => Err(StageFailure { cause: clone_error(CloneError::Exists), owned_staging: None }),
            Err(error) => {
                let created = matches!(root.stat(Path::new(&name)), Ok(stat) if stat.kind == Kind::File);
                if created {
                    Err(self.owned_failure(&name, clone_error(error)))
                } else {
                    Err(StageFailure { cause: clone_error(error), owned_staging: None })
                }
            }
        };
        root.revalidate().map_err(|_| lost_root())?;
        result
    }

    fn open(&self, path: &Path) -> Result<File, ArchiveError> {
        self.root()?.open_file(path, Access::Read).map_err(|error| archive_error(path, error))
    }

    fn size(&self, handle: &File) -> Result<u64, ArchiveError> {
        handle.metadata().map(|metadata| metadata.len()).map_err(|error| io(&error))
    }

    fn read_at(&self, handle: &File, offset: u64, buf: &mut [u8]) -> Result<(), ReadFailure> {
        handle.read_exact_at(buf, offset).map_err(|_| ReadFailure)
    }

    fn digest(&self, path: &Path) -> Result<Sha256Digest, ArchiveError> {
        let mut file = self.open(path)?;
        digest_open(&mut file).map_err(|error| io(&error))
    }

    fn validate_trash_path(&self, path: &Path) -> Result<(), ArchiveError> {
        self.root()?.regular(path).map_err(|error| archive_error(path, error))?;
        Ok(())
    }

    fn target(&self, name: &str) -> PathBuf {
        self.directory.join(name)
    }

    fn archived(&self) -> Result<Vec<PathBuf>, ArchiveError> {
        self.list(|name| name.ends_with(".m4a") && !name.starts_with('.'))
    }

    fn staged_leftovers(&self) -> Result<Vec<PathBuf>, ArchiveError> {
        self.list(|name| name.starts_with(STAGING_PREFIX))
    }
}
```

A clone that reports `Exists` found the name taken, so the file there is someone else's and is never
owned. Any other clone failure owns the name exactly when a regular file now stands there, which is the
byte copy that ran out of space part way; the digest, mode and size steps after a successful clone own it
unconditionally. Nothing here removes a file.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo fmt --all`

Run: `cargo test -p vpt-adapters archive`

Expected: all 10 staging tests PASS. Run `cargo clippy -p vpt-adapters --all-targets -- -D warnings` and
expect no warnings.

- [ ] **Step 5: Commit**

Run: `cargo fmt --all`

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(archive): staging below the audio root with an owned-cleanup contract"
```

______________________________________________________________________

### Task 18: Exclusive publication, verified on the platform first

The spec leaves the primitive to the plan and asks that the plan verify it before choosing it. The
primitive is `renameatx_np` with `RENAME_EXCL` relative to the archive root's descriptor, which Task 5a
wrapped as `RootDir::rename_exclusive` and whose test proves on the executor's volume that an existing
target is kept untouched and an absent one is placed. If the filesystem refuses that flag, publication
returns the typed archive failure and the staged file stays where it is for the normal owned cleanup:
there is no link-and-unlink fallback, because nothing in vpt unlinks.

**Files:**

- Create: `crates/vpt-adapters/src/archive/publish.rs`
- Modify: `crates/vpt-adapters/src/archive/mod.rs`, `crates/vpt-application/src/ports/archive.rs`,
  `crates/vpt-application/src/ports/mod.rs`

**Interfaces:**

- Consumes: `ArchiveError`, `RootDir::{open_file, rename_exclusive, sync}`.

- Produces: `vpt_application::ports::Published::{Placed(PathBuf), Exists(PathBuf)}` and, on `Archive`,
  `fn publish(&self, staged: &Path, target_name: &str) -> Result<Published, ArchiveError>` (sync the
  staged file, move it to the target without replacing one, sync the directory; a directory sync that
  fails after the move is `ArchiveError::PlacedUnsynced(target)`) and
  `fn sync_existing(&self, path: &Path) -> Result<(), ArchiveError>` (an archive file and its directory,
  for duplicate recovery); privately in the adapters module `archive::publish`,
  `exclusive(root: &RootDir, staged: &Path, target_name: &str) -> Result<Published, ArchiveError>` and
  `sync_file_and_directory(root: &RootDir, path: &Path) -> Result<(), ArchiveError>`.

- [ ] **Step 1: Write the failing tests**

Declare first: `crates/vpt-adapters/src/archive/mod.rs` gains `mod publish;` and
`crates/vpt-application/src/ports/mod.rs` adds `Published` to the archive re-export.
`crates/vpt-adapters/src/archive/publish.rs` starts as its test module alone:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::archive::ClonefileArchive;
    use std::path::PathBuf;
    use vpt_application::ports::Archive;

    fn audio() -> (tempfile::TempDir, PathBuf, RootDir) {
        let temp = tempfile::tempdir().expect("temp");
        let audio = temp.path().canonicalize().expect("canonical").join("audio");
        std::fs::create_dir(&audio).expect("audio");
        let root = RootDir::open(&audio).expect("root");
        (temp, audio, root)
    }

    #[test]
    fn publish_places_an_absent_target_and_removes_the_staged_name() {
        let (_temp, audio, root) = audio();
        let staged = audio.join(".vpt-staging-1.m4a");
        std::fs::write(&staged, b"bytes").expect("staged");
        assert_eq!(exclusive(&root, &staged, "id.m4a"), Ok(Published::Placed(audio.join("id.m4a"))));
        assert_eq!(std::fs::read(audio.join("id.m4a")).expect("read"), b"bytes");
        assert!(!staged.exists());
    }

    #[test]
    fn publish_never_replaces_an_existing_target_and_names_it() {
        let (_temp, audio, root) = audio();
        let staged = audio.join(".vpt-staging-2.m4a");
        std::fs::write(&staged, b"new").expect("staged");
        std::fs::write(audio.join("id.m4a"), b"old").expect("existing");
        assert_eq!(exclusive(&root, &staged, "id.m4a"), Ok(Published::Exists(audio.join("id.m4a"))));
        assert_eq!(std::fs::read(audio.join("id.m4a")).expect("read"), b"old");
        assert!(staged.exists(), "the staged duplicate stays for the caller to trash");
    }

    #[test]
    fn a_staged_name_outside_the_root_or_a_nested_target_is_an_escape() {
        let (temp, audio, root) = audio();
        let outside = temp.path().canonicalize().expect("canonical").join("elsewhere.m4a");
        std::fs::write(&outside, b"x").expect("outside");
        assert_eq!(exclusive(&root, &outside, "id.m4a"), Err(ArchiveError::Escape(outside)));
        let staged = audio.join(".vpt-staging-3.m4a");
        std::fs::write(&staged, b"x").expect("staged");
        assert_eq!(exclusive(&root, &staged, "sub/id.m4a"), Err(ArchiveError::Escape(audio.join("sub/id.m4a"))));
        assert!(staged.exists());
    }

    #[test]
    fn syncing_a_missing_file_is_a_sync_error_and_an_existing_one_succeeds() {
        let (_temp, audio, root) = audio();
        let missing = sync_file_and_directory(&root, &audio.join("missing.m4a"));
        assert!(matches!(missing, Err(ArchiveError::Sync(_))), "{missing:?}");
        std::fs::write(audio.join("present.m4a"), b"x").expect("present");
        assert_eq!(sync_file_and_directory(&root, &audio.join("present.m4a")), Ok(()));
    }

    #[test]
    fn the_archive_publishes_and_syncs_through_its_port() {
        let (_temp, audio, _root) = audio();
        let archive = ClonefileArchive::open(&audio).expect("archive");
        std::fs::write(audio.join(".vpt-staging-4.m4a"), b"bytes").expect("staged");
        assert_eq!(archive.publish(&audio.join(".vpt-staging-4.m4a"), "id.m4a"), Ok(Published::Placed(audio.join("id.m4a"))));
        assert_eq!(archive.sync_existing(&audio.join("id.m4a")), Ok(()));
        assert_eq!(archive.archived().expect("list"), vec![audio.join("id.m4a")]);
    }
    #[test]
    fn publication_refuses_a_replaced_root_before_returning_either_outcome() {
        for already_exists in [false, true] {
            let (_temp, audio, root) = audio();
            let staged = audio.join(".vpt-staging-race.m4a");
            std::fs::write(&staged, b"staged").expect("stage");
            if already_exists { std::fs::write(audio.join("id.m4a"), b"original").expect("target"); }
            std::fs::rename(&audio, audio.with_extension("saved")).expect("move root");
            std::fs::create_dir(&audio).expect("replacement");
            assert_eq!(exclusive(&root, &staged, "id.m4a"), Err(ArchiveError::Escape(audio.join("id.m4a"))));
            assert!(std::fs::read_dir(&audio).expect("replacement").next().is_none());
        }
    }

}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-adapters publish`

Expected: the build fails with `cannot find` for `exclusive`, `sync_file_and_directory` and `Published`,
and `no method named publish` on the archive.

- [ ] **Step 3: Write the minimal implementation**

Append to `crates/vpt-application/src/ports/archive.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Published {
    Placed(PathBuf),
    Exists(PathBuf),
}
```

and add to `Archive`:

```rust
    /// Sync the staged file, move it to `target_name` without replacing an
    /// existing target, sync the directory.
    fn publish(&self, staged: &Path, target_name: &str) -> Result<Published, ArchiveError>;
    /// Sync an archive file and its directory (duplicate recovery).
    fn sync_existing(&self, path: &Path) -> Result<(), ArchiveError>;
```

`crates/vpt-adapters/src/archive/publish.rs`, above its test module:

```rust
//! Publication never replaces an existing target. On this platform that is one
//! rename with the exclusive flag, relative to the root's descriptor; the file
//! is synced before and the directory after, so a committed row always has
//! its archive on disk.

use super::archive_error;
use crate::contained::{Access, RootDir};
use std::path::Path;
use vpt_application::ports::{ArchiveError, Published};

pub fn exclusive(root: &RootDir, staged: &Path, target_name: &str) -> Result<Published, ArchiveError> {
    let target = root.leaf(Path::new(target_name)).map_err(|error| archive_error(&root.path().join(target_name), error))?;
    sync_file(root, staged)?;
    let placed = root.rename_exclusive(staged, &target).map_err(|error| archive_error(&target, error))?;
    if !placed {
        root.revalidate().map_err(|_| ArchiveError::Escape(target.clone()))?;
        return Ok(Published::Exists(target));
    }
    root.sync().map_err(|_| ArchiveError::PlacedUnsynced(target.clone()))?;
    root.revalidate().map_err(|_| ArchiveError::Escape(target.clone()))?;
    Ok(Published::Placed(target))
}

pub fn sync_file_and_directory(root: &RootDir, path: &Path) -> Result<(), ArchiveError> {
    sync_file(root, path)?;
    root.sync().map_err(|error| ArchiveError::Sync(format!("{error:?}")))
}

fn sync_file(root: &RootDir, path: &Path) -> Result<(), ArchiveError> {
    let file = root.open_file(path, Access::Read).map_err(|error| match error {
        crate::contained::ContainedError::Io { kind, .. } => ArchiveError::Sync(kind.to_string()),
        other => archive_error(path, other),
    })?;
    file.sync_all().map_err(|error| ArchiveError::Sync(error.kind().to_string()))
}
```

and in `impl Archive for ClonefileArchive` (`archive/mod.rs`):

```rust
    fn publish(&self, staged: &Path, target_name: &str) -> Result<Published, ArchiveError> {
        publish::exclusive(self.root()?, staged, target_name)
    }

    fn sync_existing(&self, path: &Path) -> Result<(), ArchiveError> {
        publish::sync_file_and_directory(self.root()?, path)
    }
```

with `Published` added to that file's `vpt_application::ports` import.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo fmt --all`

Run: `cargo test -p vpt-adapters archive`

Expected: the 10 staging tests and the 6 publication tests PASS. Run
`cargo clippy -p vpt-adapters --all-targets -- -D warnings` and expect no warnings.

- [ ] **Step 5: Commit**

Run: `cargo fmt --all`

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(archive): exclusive publication with the file and directory synced"
```

______________________________________________________________________

### Task 19: The ingest use case, happy path

The sweep runs over real adapters in these tests (a fixture container, the clone archive and the SQLite
ledger in a temporary directory) with three small fakes for the clock, the Trash and the notifier, so the
tests exercise the code that ships. They live as integration tests of the adapters crate. The use case is
generic over its six ports; nothing in it names a handle type, and the wholeness gate reads through the
recorder's and the archive's `read_at`.

**Files:**

- Create: `crates/vpt-domain/src/notification.rs`
- Modify: `crates/vpt-domain/src/lib.rs`
- Create: `crates/vpt-application/src/ports/clock.rs`, `crates/vpt-application/src/ports/trash.rs`,
  `crates/vpt-application/src/ports/notifier.rs`, `crates/vpt-application/src/ingest/mod.rs`,
  `crates/vpt-application/src/ingest/candidate.rs`, `crates/vpt-application/src/ingest/publish.rs`
- Modify: `crates/vpt-application/src/ports/mod.rs`, `crates/vpt-application/src/lib.rs`
- Test: `crates/vpt-adapters/tests/support/mod.rs`, `crates/vpt-adapters/tests/ingest_sweep.rs`,
  `crates/vpt-adapters/tests/ingest_titles.rs`

**Interfaces:**

- Consumes: every port so far, `RecordingLedger::{seen, record_seen, commit}`, the domain gates and
  `inspect`; `RecorderStore::refresh_titles(&self) -> Result<(), RecorderError>` before each non-dry
  sweep, and `Archive::validate_trash_path(&self, path: &Path) -> Result<(), ArchiveError>` before
  cleanup.

- Produces:

  - `vpt_domain::notification::{EventKind, EventState, Notification}`;
    `EventKind::{ReviewNeeded, TranscribeFailed, IngestFailed, Deferred, BriefWritten,`
    `Retention, ConfigRefused}` with `as_str`; `EventState::{NeedsAttention, Failed, Done}` with
    `as_str`; `Notification { pub event: EventKind, pub state: EventState,`
    `pub recording: Option<RecordingId>, pub detail: String, pub counts: Vec<(String, u64)>,`
    `pub paths: Vec<(String, PathBuf)>, pub occurred_at: UtcInstant }` with constructors
    `Notification::failed(event, detail: String, at: UtcInstant)`,
    `Notification::attention(event, recording: Option<RecordingId>, detail: String,`
    `paths: Vec<(String, PathBuf)>, at: UtcInstant)`,
    `Notification::done(event, detail: String, counts: Vec<(String, u64)>, at: UtcInstant)`.
  - `vpt_application::ports::{Clock, Trash, TrashError, Notifier, DeliveryOutcome}`:
    `trait Clock { fn now(&self) -> UtcInstant; fn offset_at(&self, at: UtcInstant) -> UtcOffset; }`;
    `TrashError::{HelperAbsent, HelperVersion { found: u32 }, Failed(String), Unknown(String)}`,
    `trait Trash { fn trash(&self, path: &Path) -> Result<PathBuf, TrashError>;`
    `fn take_diagnostics(&self) -> Vec<String>; }` (default: empty);
    `DeliveryOutcome::{Delivered, Suppressed, Failed(String)}`,
    `trait Notifier { fn deliver(&self, notification: &Notification) -> DeliveryOutcome;`
    `fn take_diagnostics(&self) -> Vec<String>; }` (default: empty).
  - `vpt_application::{Ingest, Mode, IngestReport, Deferred, WouldIngest, IngestFailure, IngestError}`
    (the file `ingest/` stays private, `lib.rs` re-exports these):
    `Ingest<'a, R, A, L, C, T, N> { pub recorder: &'a R, pub archive: &'a A, pub ledger: &'a L,`
    `pub clock: &'a C, pub trash: &'a T, pub notifier: &'a N, pub settings: &'a SourceSettings }` with
    `R: RecorderStore, A: Archive, L: RecordingLedger, C: Clock, T: Trash + ?Sized, N: Notifier + ?Sized`;
    `Mode { pub dry_run: bool, pub once: Option<PathBuf> }` (`Default` is the full sweep);
    `IngestReport { pub ingested: Vec<RecordingRecord>, pub already_ingested: Vec<RecordingId>,`
    `pub recovered: Vec<RecordingId>, pub deferred: Vec<Deferred>, pub skipped: u64,`
    `pub would_ingest: Vec<WouldIngest>, pub log: Vec<String> }`;
    `Deferred { pub path: PathBuf, pub reason: DeferralReason }`;
    `WouldIngest { pub path: PathBuf, pub title: Option<String>, pub title_source: TitleOrigin }`;
    `IngestFailure::{StoreUnreadable(String), EmptyStore, NoSpace, Sync(String),`
    `ArchiveCollision { target: PathBuf, staged: Sha256Digest, existing: Sha256Digest },`
    `OnceMissing(PathBuf), PathEscape(PathBuf), HelperVersion { found: u32 }, Ledger(LedgerError), Archive(String), Recorder(String)}`
    with `fn message(&self) -> String`; `IngestError { pub failure: IngestFailure,`
    `pub completed: Vec<RecordingId>, pub log: Vec<String> }` (`completed` holds every identity the run
    recovered or ingested, deduplicated; `log` is the run's log up to the failure);
    `Ingest::run(&self, mode: &Mode) -> Result<IngestReport, Box<IngestError>>`. This task supports the
    full sweep and `once`; Task 20 adds the gates, Task 21 the duplicates, Task 22 recovery, Task 23 the
    dry run and the aborts.
  - Test support in `crates/vpt-adapters/tests/support/mod.rs`: `Fixture::new() -> Fixture` with the
    public fields `temp`, `container` (a canonical `voice-memos` directory holding `Recordings/`, the
    four Apple subdirectories each with one inner file, and a live `CloudRecordings.db` in WAL mode whose
    connection the fixture keeps open), `recordings`, `audio`, `state`; methods
    `add_recording(&self, name: &str, bytes: &[u8]) -> PathBuf` (mtime 60 s before the fixed clock),
    `set_mtime(&self, path: &Path, secs: i64)`, `add_title(&self, path: &str, label: &str)`,
    `replace_title(&self, label: &str)`, `store(&self) -> VoiceMemosStore`,
    `titled_store(&self, refresh: bool) -> VoiceMemosStore`, `archive(&self) -> ClonefileArchive`,
    `ledger(&self) -> SqliteLedger`, `settings(&self) -> SourceSettings`, `parts(&self) -> Parts`,
    `entries(&self) -> Vec<Entry>` (every entry below the container, recursively: relative path, size,
    mtime seconds and nanoseconds, flags);
    `Parts { store, archive, ledger, clock, trash, notifier, settings }` with
    `fn ingest(&self) -> RealIngest<'_>` and `fn events(&self) -> Vec<EventKind>`;
    `FixedClock { pub now: UtcInstant, pub offset: UtcOffset }` (`Copy`) and `clock() -> FixedClock` (an
    August 2026 instant an hour after `CAPTURED`, offset `-21_600`); `RecordingTrash::new(dir: PathBuf)`
    with `pub moved: RefCell<Vec<PathBuf>>` and `pub absent: Cell<bool>`;
    `RecordingNotifier(pub RefCell<Vec<Notification>>)`; `CAPTURED: i64 = 1_787_604_456`;
    `digest_of(bytes: &[u8]) -> Sha256Digest`.

- [ ] **Step 1: Write the failing tests**

`crates/vpt-adapters/tests/support/mod.rs`:

```rust
//! Real adapters over a temporary directory, plus the three fakes the sweep
//! needs for the clock, the Trash and the notifier.

#![allow(dead_code)]

use sha2::{Digest, Sha256};
use std::cell::{Cell, RefCell};
use std::os::macos::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};
use vpt_adapters::ClonefileArchive;
use vpt_adapters::RootDir;
use vpt_adapters::SqliteLedger;
use vpt_adapters::{APPLE_SUBDIRECTORIES, VoiceMemosStore};
use vpt_application::ports::{Clock, DeliveryOutcome, Notifier, Trash, TrashError};
use vpt_application::{Ingest, SourceSettings};
use vpt_domain::digest::Sha256Digest;
use vpt_domain::notification::{EventKind, Notification};
use vpt_domain::time::{UtcInstant, UtcOffset};

pub const CAPTURED: i64 = 1_787_604_456;

/// Relative path, size, mtime seconds, mtime nanoseconds, flags.
pub type Entry = (PathBuf, u64, i64, i64, u32);

pub type RealIngest<'a> = Ingest<'a, VoiceMemosStore, ClonefileArchive, SqliteLedger, FixedClock, RecordingTrash, RecordingNotifier>;

pub struct Fixture {
    pub temp: tempfile::TempDir,
    pub container: PathBuf,
    pub recordings: PathBuf,
    pub audio: PathBuf,
    pub state: PathBuf,
    live_database: rusqlite::Connection,
}

impl Fixture {
    pub fn new() -> Fixture {
        let temp = tempfile::tempdir().expect("temp");
        let base = temp.path().canonicalize().expect("canonical");
        let container = base.join("voice-memos");
        let recordings = container.join("Recordings");
        let audio = base.join("home/audio");
        let state = base.join("state/vpt");
        for dir in [&recordings, &audio, &state] {
            std::fs::create_dir_all(dir).expect("fixture dir");
        }
        for name in APPLE_SUBDIRECTORIES {
            std::fs::create_dir(recordings.join(name)).expect("apple subdirectory");
            std::fs::write(recordings.join(name).join("inner.m4a"), b"never listed").expect("inner");
        }
        let live_database = rusqlite::Connection::open(container.join("CloudRecordings.db")).expect("live db");
        live_database.pragma_update(None, "journal_mode", "WAL").expect("wal");
        live_database
            .execute_batch("CREATE TABLE ZCLOUDRECORDING (Z_PK INTEGER PRIMARY KEY, ZPATH TEXT, ZCUSTOMLABEL TEXT)")
            .expect("schema");
        Fixture { temp, container, recordings, audio, state, live_database }
    }

    /// A source file whose mtime sits 60 s before the fixed clock.
    pub fn add_recording(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.recordings.join(name);
        std::fs::write(&path, bytes).expect("recording");
        self.set_mtime(&path, clock().now.secs - 60);
        path
    }

    pub fn set_mtime(&self, path: &Path, secs: i64) {
        let at = UNIX_EPOCH + Duration::from_secs(u64::try_from(secs).expect("fixture epoch"));
        let file = std::fs::File::options().write(true).open(path).expect("open for times");
        file.set_times(std::fs::FileTimes::new().set_modified(at)).expect("mtime");
    }

    pub fn add_title(&self, path: &str, label: &str) {
        self.live_database
            .execute("INSERT INTO ZCLOUDRECORDING (ZPATH, ZCUSTOMLABEL) VALUES (?1, ?2)", [path, label])
            .expect("title row");
    }

    pub fn replace_title(&self, label: &str) {
        self.live_database.execute("UPDATE ZCLOUDRECORDING SET ZCUSTOMLABEL = ?1", [label]).expect("update title");
    }

    pub fn store(&self) -> VoiceMemosStore {
        VoiceMemosStore::open(&self.recordings).expect("store")
    }

    pub fn titled_store(&self, refresh: bool) -> VoiceMemosStore {
        self.store().with_titles(self.state.clone(), refresh)
    }

    pub fn archive(&self) -> ClonefileArchive {
        ClonefileArchive::open(&self.audio).expect("archive")
    }

    pub fn ledger(&self) -> SqliteLedger {
        SqliteLedger::open(&RootDir::open(&self.state).expect("state root")).expect("ledger")
    }

    pub fn settings(&self) -> SourceSettings {
        SourceSettings {
            recordings_dir: self.recordings.clone(),
            read_titles: false,
            quiet_period_secs: 30,
            deferral_page_threshold: 4,
            max_audio_bytes: 2_147_483_648,
        }
    }

    pub fn parts(&self) -> Parts {
        Parts {
            store: self.store(),
            archive: self.archive(),
            ledger: self.ledger(),
            clock: clock(),
            trash: RecordingTrash::new(self.temp.path().join("trash")),
            notifier: RecordingNotifier::default(),
            settings: self.settings(),
        }
    }

    /// Every entry below the container, recursively, in sorted order.
    pub fn entries(&self) -> Vec<Entry> {
        let mut entries = Vec::new();
        walk(&self.container, &self.container, &mut entries);
        entries.sort();
        entries
    }
}

fn walk(root: &Path, directory: &Path, out: &mut Vec<Entry>) {
    for entry in std::fs::read_dir(directory).expect("dir") {
        let entry = entry.expect("entry");
        let metadata = entry.metadata().expect("metadata");
        let path = entry.path();
        let relative = path.strip_prefix(root).expect("below the container").to_path_buf();
        out.push((relative, metadata.len(), metadata.st_mtime(), metadata.st_mtime_nsec(), metadata.st_flags()));
        if metadata.is_dir() {
            walk(root, &path, out);
        }
    }
}

pub fn digest_of(bytes: &[u8]) -> Sha256Digest {
    Sha256Digest(Sha256::digest(bytes).into())
}

pub struct Parts {
    pub store: VoiceMemosStore,
    pub archive: ClonefileArchive,
    pub ledger: SqliteLedger,
    pub clock: FixedClock,
    pub trash: RecordingTrash,
    pub notifier: RecordingNotifier,
    pub settings: SourceSettings,
}

impl Parts {
    pub fn ingest(&self) -> RealIngest<'_> {
        Ingest {
            recorder: &self.store,
            archive: &self.archive,
            ledger: &self.ledger,
            clock: &self.clock,
            trash: &self.trash,
            notifier: &self.notifier,
            settings: &self.settings,
        }
    }

    pub fn events(&self) -> Vec<EventKind> {
        self.notifier.0.borrow().iter().map(|notification| notification.event).collect()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FixedClock {
    pub now: UtcInstant,
    pub offset: UtcOffset,
}

impl Clock for FixedClock {
    fn now(&self) -> UtcInstant {
        self.now
    }

    fn offset_at(&self, _at: UtcInstant) -> UtcOffset {
        self.offset
    }
}

pub fn clock() -> FixedClock {
    FixedClock { now: UtcInstant { secs: CAPTURED + 3_600 }, offset: UtcOffset { secs: -21_600 } }
}

pub struct RecordingTrash {
    dir: PathBuf,
    pub moved: RefCell<Vec<PathBuf>>,
    pub absent: Cell<bool>,
}

impl RecordingTrash {
    pub fn new(dir: PathBuf) -> RecordingTrash {
        std::fs::create_dir_all(&dir).expect("trash dir");
        RecordingTrash { dir, moved: RefCell::new(vec![]), absent: Cell::new(false) }
    }
}

impl Trash for RecordingTrash {
    fn trash(&self, path: &Path) -> Result<PathBuf, TrashError> {
        if self.absent.get() {
            return Err(TrashError::HelperAbsent);
        }
        let target = self.dir.join(path.file_name().expect("name"));
        std::fs::rename(path, &target).map_err(|error| TrashError::Failed(error.kind().to_string()))?;
        self.moved.borrow_mut().push(path.to_path_buf());
        Ok(target)
    }
}

#[derive(Default)]
pub struct RecordingNotifier(pub RefCell<Vec<Notification>>);

impl Notifier for RecordingNotifier {
    fn deliver(&self, notification: &Notification) -> DeliveryOutcome {
        self.0.borrow_mut().push(notification.clone());
        DeliveryOutcome::Delivered
    }
}
```

`crates/vpt-adapters/tests/ingest_sweep.rs`:

```rust
mod support;

use std::os::unix::fs::PermissionsExt;
use support::{CAPTURED, Fixture, FixedClock, Parts, clock};
use vpt_application::Mode;
use vpt_application::ports::{Archive, RecordingLedger, StageStates, TitleOrigin};
use vpt_domain::fixtures::m4a;
use vpt_domain::time::UtcInstant;

#[test]
fn a_whole_recording_is_archived_under_its_identity_and_recorded_in_the_ledger() {
    let fixture = Fixture::new();
    let source = fixture.add_recording("20260824 144736-4F3AB19C.m4a", &m4a(CAPTURED, 612, b"audio bytes"));
    let parts = fixture.parts();

    let report = parts.ingest().run(&Mode::default()).expect("sweep");

    assert_eq!(report.ingested.len(), 1);
    let record = &report.ingested[0];
    assert!(record.id.as_str().starts_with("2026-08-24T144736-"), "{}", record.id);
    assert_eq!(record.source_path.as_deref(), Some(source.as_path()));
    assert_eq!(record.duration_secs, 612);
    assert_eq!(record.captured_at, UtcInstant { secs: CAPTURED });
    assert_eq!(record.captured_offset, clock().offset);
    assert_eq!(record.title, None);
    assert_eq!(record.title_source, TitleOrigin::Unavailable);
    assert_eq!(record.stages, StageStates::fresh());
    assert_eq!(record.audio_path, fixture.audio.join(format!("{}.m4a", record.id)));
    assert_eq!(std::fs::read(&record.audio_path).expect("archive"), m4a(CAPTURED, 612, b"audio bytes"));
    assert_eq!(std::fs::metadata(&record.audio_path).expect("meta").permissions().mode() & 0o777, 0o600);
    assert_eq!(parts.ledger.by_id(&record.id).expect("read"), Some(record.clone()));
    let row = parts.ledger.seen(&source).expect("seen").expect("row");
    assert_eq!(row.recording, Some(record.id.clone()));
    assert_eq!((row.deferral_count, row.deferral_reason, row.source_gone_at), (0, None, None));
    assert!(parts.archive.staged_leftovers().expect("leftovers").is_empty());
    assert!(parts.events().is_empty());
    assert!(parts.trash.moved.borrow().is_empty());
}

#[test]
fn a_full_sweep_with_titles_leaves_every_container_entry_with_its_size_mtime_and_flags() {
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"a"));
    fixture.add_recording("b.m4a", &m4a(CAPTURED + 60, 2, b"bb"));
    std::fs::write(fixture.recordings.join("a.waveform"), b"w").expect("sidecar");
    fixture.add_title("a.m4a", "Alpha");
    let before = fixture.entries();
    let mut parts = fixture.parts();
    parts.store = fixture.titled_store(true);

    let report = parts.ingest().run(&Mode::default()).expect("sweep");

    assert_eq!(report.ingested.len(), 2);
    assert_eq!(report.ingested[0].title.as_deref(), Some("Alpha"));
    assert_eq!(report.ingested[0].title_source, TitleOrigin::VoiceMemos);
    assert_eq!(report.ingested[1].title, None);
    assert_eq!(fixture.entries(), before);
    assert!(fixture.state.join("title-copy").join("CloudRecordings.db").exists());
}

#[test]
fn a_second_sweep_skips_the_unchanged_entries_and_refreshes_last_seen() {
    let fixture = Fixture::new();
    let source = fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"a"));
    let parts = fixture.parts();
    parts.ingest().run(&Mode::default()).expect("first");
    let later = Parts { clock: FixedClock { now: UtcInstant { secs: CAPTURED + 7_200 }, ..clock() }, ..fixture.parts() };

    let report = later.ingest().run(&Mode::default()).expect("second");

    assert!(report.ingested.is_empty());
    assert_eq!(report.skipped, 1);
    assert_eq!(later.ledger.recordings().expect("list").len(), 1);
    let row = later.ledger.seen(&source).expect("seen").expect("row");
    assert_eq!(row.last_seen, UtcInstant { secs: CAPTURED + 7_200 });
    assert_eq!(row.first_seen, UtcInstant { secs: CAPTURED + 3_600 });
}

#[test]
fn once_ingests_exactly_the_named_file() {
    let fixture = Fixture::new();
    let a = fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"a"));
    fixture.add_recording("b.m4a", &m4a(CAPTURED + 1, 1, b"b"));
    let parts = fixture.parts();

    let report = parts.ingest().run(&Mode { dry_run: false, once: Some(a.clone()) }).expect("once");

    assert_eq!(report.ingested.len(), 1);
    assert_eq!(report.ingested[0].source_path.as_deref(), Some(a.as_path()));
    assert_eq!(parts.ledger.recordings().expect("list").len(), 1);
}
```

`crates/vpt-adapters/tests/ingest_titles.rs`:

```rust
mod support;

use support::{CAPTURED, Fixture};
use vpt_adapters::TitleCopy;
use vpt_application::Mode;
use vpt_application::ports::TitleLookup;
use vpt_domain::fixtures::m4a;

#[test]
fn every_non_dry_sweep_refreshes_titles_before_candidate_work() {
    for source in [Some(m4a(CAPTURED, 1, b"audio")), Some(b"broken".to_vec()), None] {
        let fixture = Fixture::new();
        if let Some(bytes) = source { fixture.add_recording("a.m4a", &bytes); }
        fixture.add_title("a.m4a", "Alpha");
        let mut parts = fixture.parts();
        parts.store = fixture.titled_store(true);
        let _ = parts.ingest().run(&Mode::default());
        assert_eq!(TitleCopy::existing(&fixture.state).title("a.m4a"), TitleLookup::Titled("Alpha".into()));
        fixture.replace_title("Beta");
        let _ = parts.ingest().run(&Mode::default());
        assert_eq!(TitleCopy::existing(&fixture.state).title("a.m4a"), TitleLookup::Titled("Beta".into()));
    }
}

#[test]
fn ordinary_title_copy_failure_still_ingests_untitled() {
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"audio"));
    let mut parts = fixture.parts();
    parts.store = fixture.store().with_titles(fixture.state.join("absent"), true);
    let report = parts.ingest().run(&Mode::default()).expect("ingested");
    assert_eq!(report.ingested.len(), 1);
    assert_eq!(report.ingested[0].title, None);
    assert!(parts.events().is_empty());
}

```

`Parts` is a plain struct, so the third test rebuilds it around a later clock with struct update syntax
over a fresh `fixture.parts()`.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-adapters --test ingest_sweep --test ingest_titles`

Expected: the build fails with `unresolved import` for `vpt_application::Ingest` and the three new ports.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-domain/src/notification.rs`:

```rust
//! The event vpt raises: identities, counts, classes and paths, never text
//! from a recording.

use crate::identity::RecordingId;
use crate::time::UtcInstant;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    ReviewNeeded,
    TranscribeFailed,
    IngestFailed,
    Deferred,
    BriefWritten,
    Retention,
    ConfigRefused,
}

impl EventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EventKind::ReviewNeeded => "review_needed",
            EventKind::TranscribeFailed => "transcribe_failed",
            EventKind::IngestFailed => "ingest_failed",
            EventKind::Deferred => "deferred",
            EventKind::BriefWritten => "brief_written",
            EventKind::Retention => "retention",
            EventKind::ConfigRefused => "config_refused",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventState {
    NeedsAttention,
    Failed,
    Done,
}

impl EventState {
    pub fn as_str(self) -> &'static str {
        match self {
            EventState::NeedsAttention => "needs_attention",
            EventState::Failed => "failed",
            EventState::Done => "done",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    pub event: EventKind,
    pub state: EventState,
    pub recording: Option<RecordingId>,
    pub detail: String,
    pub counts: Vec<(String, u64)>,
    pub paths: Vec<(String, PathBuf)>,
    pub occurred_at: UtcInstant,
}

impl Notification {
    pub fn failed(event: EventKind, detail: String, at: UtcInstant) -> Notification {
        Notification { event, state: EventState::Failed, recording: None, detail, counts: vec![], paths: vec![], occurred_at: at }
    }

    pub fn attention(
        event: EventKind,
        recording: Option<RecordingId>,
        detail: String,
        paths: Vec<(String, PathBuf)>,
        at: UtcInstant,
    ) -> Notification {
        Notification { event, state: EventState::NeedsAttention, recording, detail, counts: vec![], paths, occurred_at: at }
    }

    pub fn done(event: EventKind, detail: String, counts: Vec<(String, u64)>, at: UtcInstant) -> Notification {
        Notification { event, state: EventState::Done, recording: None, detail, counts, paths: vec![], occurred_at: at }
    }
}
```

Add `pub mod notification;` to the domain `lib.rs`.

`crates/vpt-application/src/ports/clock.rs`:

```rust
//! Now, and the local offset at an instant.

use vpt_domain::time::{UtcInstant, UtcOffset};

pub trait Clock {
    fn now(&self) -> UtcInstant;
    fn offset_at(&self, at: UtcInstant) -> UtcOffset;
}
```

`crates/vpt-application/src/ports/trash.rs`:

```rust
//! The system Trash. `Unknown` is a reply that never came: the file may or
//! may not have moved.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrashError {
    HelperAbsent,
    HelperVersion { found: u32 },
    Failed(String),
    Unknown(String),
}

pub trait Trash {
    fn trash(&self, path: &Path) -> Result<PathBuf, TrashError>;
    fn take_diagnostics(&self) -> Vec<String> { Vec::new() }
}
```

`crates/vpt-application/src/ports/notifier.rs`:

```rust
//! Delivery of one event. A failure here never fails the work it reports on.

use vpt_domain::notification::Notification;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeliveryOutcome {
    Delivered,
    Suppressed,
    Failed(String),
}

pub trait Notifier {
    fn deliver(&self, notification: &Notification) -> DeliveryOutcome;
    fn take_diagnostics(&self) -> Vec<String> { Vec::new() }
}
```

`crates/vpt-application/src/ports/mod.rs` gains `mod clock;`, `mod notifier;`, `mod trash;` and
`pub use clock::Clock; pub use notifier::{DeliveryOutcome, Notifier};`
`pub use trash::{Trash, TrashError};`. `crates/vpt-application/src/lib.rs` gains `mod ingest;` and
`pub use ingest::{Deferred, Ingest, IngestError, IngestFailure, IngestReport, Mode, WouldIngest};`.

`crates/vpt-application/src/ingest/mod.rs`:

```rust
//! `vpt ingest`: the sweep of spec section 5, composed from the ports.

mod candidate;
mod publish;

use crate::ports::{Archive, Clock, LedgerError, Notifier, RecorderStore, RecordingLedger, RecordingRecord, TitleOrigin, Trash};
use crate::settings::SourceSettings;
use std::path::PathBuf;
use vpt_domain::digest::Sha256Digest;
use vpt_domain::identity::RecordingId;
use vpt_domain::notification::{EventKind, Notification};
use vpt_domain::sweep::DeferralReason;

pub struct Ingest<'a, R, A, L, C, T: ?Sized, N: ?Sized> {
    pub recorder: &'a R,
    pub archive: &'a A,
    pub ledger: &'a L,
    pub clock: &'a C,
    pub trash: &'a T,
    pub notifier: &'a N,
    pub settings: &'a SourceSettings,
}

/// A dry run stages, writes and notifies nothing; `once` names the one file to ingest.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Mode {
    pub dry_run: bool,
    pub once: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deferred {
    pub path: PathBuf,
    pub reason: DeferralReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WouldIngest {
    pub path: PathBuf,
    pub title: Option<String>,
    pub title_source: TitleOrigin,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct IngestReport {
    pub ingested: Vec<RecordingRecord>,
    pub already_ingested: Vec<RecordingId>,
    pub recovered: Vec<RecordingId>,
    pub deferred: Vec<Deferred>,
    pub skipped: u64,
    pub would_ingest: Vec<WouldIngest>,
    pub log: Vec<String>,
}

impl IngestReport {
    /// Every identity this run made durable, recovered first, without repeats.
    pub fn completed(&self) -> Vec<RecordingId> {
        let mut ids = self.recovered.clone();
        for record in &self.ingested {
            if !ids.contains(&record.id) {
                ids.push(record.id.clone());
            }
        }
        ids
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngestFailure {
    StoreUnreadable(String),
    EmptyStore,
    NoSpace,
    Sync(String),
    ArchiveCollision { target: PathBuf, staged: Sha256Digest, existing: Sha256Digest },
    /// A source or archive path outside its root or through a link: exit 3, `path_escape`.
    PathEscape(PathBuf),
    HelperVersion { found: u32 },
    Ledger(LedgerError),
    Archive(String),
    Recorder(String),
}

impl IngestFailure {
    pub fn message(&self) -> String {
        match self {
            IngestFailure::StoreUnreadable(detail) => format!("the recordings directory is unreadable: {detail}"),
            IngestFailure::EmptyStore => "the recordings directory holds no recording where it held some before".into(),
            IngestFailure::NoSpace => "no space left in the audio store".into(),
            IngestFailure::Sync(detail) => format!("could not sync the archive: {detail}"),
            IngestFailure::ArchiveCollision { target, .. } => {
                format!("{} exists with different content (archive_collision)", target.display())
            }
            IngestFailure::PathEscape(path) => format!("{} escapes its root (path_escape)", path.display()),
            IngestFailure::HelperVersion { found } => format!("helper major {found} is incompatible (helper_version)"),
            IngestFailure::Ledger(error) => format!("ledger failure: {error:?}"),
            IngestFailure::Archive(detail) => format!("archive failure: {detail}"),
            IngestFailure::Recorder(detail) => format!("source failure: {detail}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestError {
    pub failure: IngestFailure,
    pub completed: Vec<RecordingId>,
    pub log: Vec<String>,
}

impl<R, A, L, C, T, N> Ingest<'_, R, A, L, C, T, N>
where
    R: RecorderStore,
    A: Archive,
    L: RecordingLedger,
    C: Clock,
    T: Trash + ?Sized,
    N: Notifier + ?Sized,
{
    pub fn run(&self, mode: &Mode) -> Result<IngestReport, Box<IngestError>> {
        let mut report = IngestReport::default();
        match self.sweep(mode, &mut report) {
            Ok(()) => Ok(report),
            Err(failure) => {
                if !mode.dry_run {
                    let at = self.clock.now();
                    self.notifier.deliver(&Notification::failed(EventKind::IngestFailed, failure.message(), at));
                    report.log.extend(self.notifier.take_diagnostics());
                }
                Err(Box::new(IngestError { failure, completed: report.completed(), log: report.log }))
            }
        }
    }

    fn sweep(&self, mode: &Mode, report: &mut IngestReport) -> Result<(), IngestFailure> {
        if !mode.dry_run {
            self.recorder.refresh_titles().map_err(candidate::recorder_failure)?;
        }
        let candidates = match &mode.once {
            Some(path) => vec![self.recorder.candidate(path).map_err(candidate::recorder_failure)?],
            None => self.recorder.candidates().map_err(candidate::recorder_failure)?,
        };
        for candidate in &candidates {
            self.process(candidate, mode, report)?;
        }
        Ok(())
    }
}
```

`crates/vpt-application/src/ingest/candidate.rs`:

```rust
//! One candidate: the pre-open decision, the descriptor, then staging.

use super::{Ingest, IngestFailure, IngestReport, Mode};
use crate::ports::{Archive, Candidate, Clock, LedgerError, Notifier, RecorderError, RecorderStore, RecordingLedger, SeenRow, Trash};
use vpt_domain::sweep::{CandidateFacts, PreOpen, SeenFacts, pre_open};
use vpt_domain::time::UtcInstant;

impl<R, A, L, C, T, N> Ingest<'_, R, A, L, C, T, N>
where
    R: RecorderStore,
    A: Archive,
    L: RecordingLedger,
    C: Clock,
    T: Trash + ?Sized,
    N: Notifier + ?Sized,
{
    pub(super) fn process(&self, candidate: &Candidate, mode: &Mode, report: &mut IngestReport) -> Result<(), IngestFailure> {
        let now = self.clock.now();
        let seen = self.ledger.seen(&candidate.path).map_err(IngestFailure::Ledger)?;
        let facts = CandidateFacts { size: candidate.size, mtime: candidate.mtime, flags: candidate.flags };
        let seen_facts = seen.as_ref().map(|row| SeenFacts {
            size: row.size,
            mtime: row.mtime,
            ingested: row.recording.is_some() && row.deferral_reason.is_none(),
            deferred_size: row.deferred_size,
        });
        let seen = match pre_open(&facts, seen_facts.as_ref()) {
            PreOpen::Unchanged => match seen {
                Some(row) => return self.unchanged(row, now, mode, report),
                None => None,
            },
            PreOpen::Dataless | PreOpen::Open => seen,
        };
        let handle = self.recorder.open(&candidate.path).map_err(recorder_failure)?;
        let metadata = self.recorder.metadata(&handle).map_err(recorder_failure)?;
        self.stage_and_publish(candidate, &handle, &metadata, seen, now, mode, report)
    }

    /// An ingested entry whose triple did not move: only its liveness is refreshed.
    pub(super) fn unchanged(&self, mut row: SeenRow, now: UtcInstant, mode: &Mode, report: &mut IngestReport) -> Result<(), IngestFailure> {
        report.skipped += 1;
        if mode.dry_run {
            return Ok(());
        }
        row.last_seen = now;
        row.source_gone_at = None;
        self.ledger.record_seen(&row).map_err(IngestFailure::Ledger)
    }
}

pub(super) fn recorder_failure(error: RecorderError) -> IngestFailure {
    match error {
        RecorderError::Escape(path) | RecorderError::NotRegular(path) => IngestFailure::PathEscape(path),
        RecorderError::Unreadable(detail) => IngestFailure::StoreUnreadable(detail),
        RecorderError::NotFound(path) => IngestFailure::Recorder(format!("{} is not a recording", path.display())),
        RecorderError::Io(detail) => IngestFailure::Recorder(detail),
    }
}

pub(super) fn fresh_seen(candidate: &Candidate, now: UtcInstant) -> SeenRow {
    SeenRow {
        path: candidate.path.clone(),
        file_name: candidate.file_name.clone(),
        size: candidate.size,
        mtime: candidate.mtime,
        flags: candidate.flags,
        first_seen: now,
        last_seen: now,
        deferral_count: 0,
        deferral_reason: None,
        deferred_size: None,
        source_gone_at: None,
        recording: None,
    }
}

pub(super) fn ledger_failure(error: LedgerError) -> IngestFailure {
    IngestFailure::Ledger(error)
}
```

`crates/vpt-application/src/ingest/publish.rs`:

```rust
//! Stage, verify, hash, publish, commit; and the owned cleanup on every
//! exit before publication.

use super::candidate::{fresh_seen, ledger_failure, recorder_failure};
use super::{Ingest, IngestFailure, IngestReport, Mode};
use crate::ports::{
    Archive, ArchiveError, Candidate, Clock, LedgerCommit, Notifier, Published, RecorderStore, RecordingLedger, RecordingRecord,
    SeenRow, SourceMetadata, StageStates, TitleLookup, TitleOrigin, Trash, TrashError,
};
use std::path::Path;
use vpt_domain::container::{Container, ContainerError, inspect};
use vpt_domain::identity::RecordingId;
use vpt_domain::time::UtcInstant;

impl<R, A, L, C, T, N> Ingest<'_, R, A, L, C, T, N>
where
    R: RecorderStore,
    A: Archive,
    L: RecordingLedger,
    C: Clock,
    T: Trash + ?Sized,
    N: Notifier + ?Sized,
{
    #[allow(clippy::too_many_arguments)]
    pub(super) fn stage_and_publish(
        &self,
        candidate: &Candidate,
        handle: &R::Handle,
        metadata: &SourceMetadata,
        seen: Option<SeenRow>,
        now: UtcInstant,
        mode: &Mode,
        report: &mut IngestReport,
    ) -> Result<(), IngestFailure> {
        let staged = match self.archive.stage(|directory, directory_identity, name| self.recorder.clone_into(handle, directory, directory_identity, name)) {
            Ok(staged) => staged,
            Err(failure) => return Err(self.abandon(failure.owned_staging.as_deref(), archive_failure(failure.cause), report)),
        };
        let after = self.owned(&staged.path, self.recorder.metadata(handle).map_err(recorder_failure), report)?;
        if after != *metadata {
            self.discard(&staged.path, report)?;
            return Err(IngestFailure::Recorder("the source changed while it was staged".into()));
        }
        let container = match self.owned(&staged.path, self.inspect_archive(&staged.path), report)? {
            Ok(container) => container,
            Err(error) => {
                self.discard(&staged.path, report)?;
                return Err(IngestFailure::Archive(format!("staged container invalid: {error:?}")));
            }
        };
        let offset = self.clock.offset_at(container.creation_time);
        let Ok(id) = RecordingId::derive(container.creation_time, offset, &staged.digest) else {
            self.discard(&staged.path, report)?;
            return Err(IngestFailure::Archive("capture instant unrepresentable".into()));
        };
        if !staged.copy_on_write {
            report.log.push(format!("{}: the archive is not copy-on-write", candidate.file_name));
        }
        let _ = mode;
        match self.archive.publish(&staged.path, &format!("{id}.m4a")) {
            Ok(Published::Placed(audio_path)) => {
                let (title, title_source) = lookup_title(self.recorder.title(&candidate.file_name));
                let record = RecordingRecord {
                    id,
                    source_path: Some(candidate.path.clone()),
                    digest: staged.digest,
                    captured_at: container.creation_time,
                    captured_offset: offset,
                    duration_secs: container.duration_secs,
                    title,
                    title_source,
                    ingested_at: now,
                    audio_path,
                    stages: StageStates::fresh(),
                    audio_trashed_at: None,
                };
                let batch = LedgerCommit {
                    seen: vec![ingested_seen(candidate, seen, &record.id, now)],
                    recordings: vec![record.clone()],
                    publications: vec![],
                };
                self.ledger.commit(&batch).map_err(ledger_failure)?;
                report.ingested.push(record);
                Ok(())
            }
            Ok(Published::Exists(audio_path)) => {
                self.discard(&staged.path, report)?;
                Err(IngestFailure::Archive(format!("{} exists", audio_path.display())))
            }
            Err(ArchiveError::PlacedUnsynced(target)) => Err(IngestFailure::Sync(format!("{} placed, directory not synced", target.display()))),
            Err(error) => Err(self.abandon(Some(&staged.path), archive_failure(error), report)),
        }
    }

    /// The wholeness gate over an archive file through the archive's own reads.
    pub(super) fn inspect_archive(&self, path: &Path) -> Result<Result<Container, ContainerError>, IngestFailure> {
        let handle = self.archive.open(path).map_err(archive_failure)?;
        let len = self.archive.size(&handle).map_err(archive_failure)?;
        Ok(inspect(len, |offset, buf| self.archive.read_at(&handle, offset, buf)))
    }

    /// A step after staging: on failure the staged file is this run's to clean up.
    pub(super) fn owned<V>(&self, staged: &Path, outcome: Result<V, IngestFailure>, report: &mut IngestReport) -> Result<V, IngestFailure> {
        outcome.map_err(|failure| self.abandon(Some(staged), failure, report))
    }

    /// Trash what this run owns, then hand back the failure that ended it.
    pub(super) fn abandon(&self, owned: Option<&Path>, failure: IngestFailure, report: &mut IngestReport) -> IngestFailure {
        if let Some(path) = owned
            && let Err(cleanup) = self.discard(path, report)
        {
            report.log.push(cleanup.message());
        }
        failure
    }

    /// Move a staged file to the Trash; when that cannot happen the file
    /// stays at mode 0600 and the log says cleanup is pending.
    pub(super) fn discard(&self, staged: &Path, report: &mut IngestReport) -> Result<(), IngestFailure> {
        self.archive.validate_trash_path(staged).map_err(archive_failure)?;
        let result = self.trash.trash(staged);
        report.log.extend(self.trash.take_diagnostics());
        if let Err(error) = result {
            if let TrashError::HelperVersion { found } = error {
                report.log.push(format!("{}: staged file kept at mode 0600, cleanup pending (helper version)", staged.display()));
                return Err(IngestFailure::HelperVersion { found });
            }
            let reason = match error {
                TrashError::HelperAbsent => "helper absent",
                TrashError::HelperVersion { .. } => "helper version",
                TrashError::Failed(_) => "trash failed",
                TrashError::Unknown(_) => "trash reply unknown",
            };
            report.log.push(format!("{}: staged file kept at mode 0600, cleanup pending ({reason})", staged.display()));
        }
        Ok(())
    }
}

pub(super) fn archive_failure(error: ArchiveError) -> IngestFailure {
    match error {
        ArchiveError::NoSpace => IngestFailure::NoSpace,
        ArchiveError::Sync(detail) => IngestFailure::Sync(detail),
        ArchiveError::PlacedUnsynced(target) => IngestFailure::Sync(format!("{} placed, directory not synced", target.display())),
        ArchiveError::Escape(path) => IngestFailure::PathEscape(path),
        ArchiveError::Io(detail) => IngestFailure::Archive(detail),
    }
}

pub(super) fn lookup_title(lookup: TitleLookup) -> (Option<String>, TitleOrigin) {
    match lookup {
        TitleLookup::Titled(title) => (Some(title), TitleOrigin::VoiceMemos),
        TitleLookup::Unavailable => (None, TitleOrigin::Unavailable),
    }
}

pub(super) fn ingested_seen(candidate: &Candidate, seen: Option<SeenRow>, id: &RecordingId, now: UtcInstant) -> SeenRow {
    let mut row = seen.unwrap_or_else(|| fresh_seen(candidate, now));
    row.size = candidate.size;
    row.mtime = candidate.mtime;
    row.flags = candidate.flags;
    row.last_seen = now;
    row.deferral_count = 0;
    row.deferral_reason = None;
    row.deferred_size = None;
    row.source_gone_at = None;
    row.recording = Some(id.clone());
    row
}
```

The `let _ = mode;` line holds the parameter until Task 20 turns the two staged gates into deferrals that
honour the mode; it is deleted there. A ledger failure after `Placed` leaves the archive file at its
final name, where the next sweep's recovery (Task 22) finds it, so nothing is trashed on that path.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo fmt --all`

Run: `cargo test -p vpt-adapters --test ingest_sweep --test ingest_titles`

Expected: 4 sweep tests and 2 title-sweep tests PASS. Run
`cargo clippy --workspace --all-targets --features dev-tools -- -D warnings` and expect no warnings.

- [ ] **Step 5: Commit**

Run: `cargo fmt --all`

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(ingest): archive a whole recording under its identity"
```

______________________________________________________________________

### Task 20: The gates: deferrals, retries and the deferred page

**Files:**

- Modify: `crates/vpt-application/src/ingest/candidate.rs`,
  `crates/vpt-application/src/ingest/publish.rs`
- Test: `crates/vpt-adapters/tests/support/fakes.rs`, `crates/vpt-adapters/tests/support/mod.rs`,
  `crates/vpt-adapters/tests/ingest_gates.rs`

**Interfaces:**

- Consumes: `vpt_domain::sweep::{size_gate, rest_gate, SweepLimits, DeferralReason}`, `inspect`,
  `RecorderStore::read_at`.

- Produces: `Ingest::defer(&self, candidate: &Candidate, seen: Option<SeenRow>,`
  `reason: DeferralReason, now: UtcInstant, mode: &Mode, report: &mut IngestReport) -> Result<(),`
  `IngestFailure>` (appends to `report.deferred`; outside a dry run records the deferral with a
  saturating count and raises the `deferred` event exactly when the count crosses the threshold). Test
  support in `support/fakes.rs`: `ProbeStore<'a>::new(inner: &'a VoiceMemosStore) -> ProbeStore`
  implementing `RecorderStore` over the real store with `pub dataless: RefCell<Vec<String>>` (names
  reported with `SF_DATALESS` set, and a panic if one is opened), `pub reads: Cell<u64>` (calls to
  `read_at`), `pub grow_after_open: Cell<bool>` (the second `metadata` call reports one more byte),
  `pub extra_flags: Cell<u32>` (flags added to every observed candidate),
  `pub clone: Cell<CloneBehaviour>` with `CloneBehaviour::{Real, ByteCopy, Truncated, Exists}`.

- [ ] **Step 1: Write the failing tests**

`crates/vpt-adapters/tests/support/mod.rs` gains `pub mod fakes;` beside its `#![allow(dead_code)]`.
`crates/vpt-adapters/tests/support/fakes.rs`:

```rust
//! Recorder and archive doubles that delegate to the real adapters and inject
//! one named failure or observation each.

use std::cell::{Cell, RefCell};
use std::fs::File;
use std::io::Write;
use std::os::unix::fs::{FileExt, OpenOptionsExt};
use std::path::Path;
use vpt_adapters::VoiceMemosStore;
use vpt_application::ports::{Candidate, CloneError, CloneKind, RecorderError, RecorderStore, SourceMetadata, TitleLookup};
use vpt_domain::container::ReadFailure;
use vpt_domain::sweep::SF_DATALESS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloneBehaviour {
    Real,
    ByteCopy,
    Truncated,
    Exists,
}

pub struct ProbeStore<'a> {
    inner: &'a VoiceMemosStore,
    pub dataless: RefCell<Vec<String>>,
    pub extra_flags: Cell<u32>,
    pub reads: Cell<u64>,
    pub grow_after_open: Cell<bool>,
    pub metadata_calls: Cell<u32>,
    pub clone: Cell<CloneBehaviour>,
}

impl<'a> ProbeStore<'a> {
    pub fn new(inner: &'a VoiceMemosStore) -> ProbeStore<'a> {
        ProbeStore {
            inner,
            dataless: RefCell::new(vec![]),
            extra_flags: Cell::new(0),
            reads: Cell::new(0),
            grow_after_open: Cell::new(false),
            metadata_calls: Cell::new(0),
            clone: Cell::new(CloneBehaviour::Real),
        }
    }

    fn forced_dataless(&self, path: &Path) -> bool {
        self.dataless.borrow().iter().any(|name| path.ends_with(name))
    }
}

/// The whole source through the descriptor, shortened by `drop` bytes at the end.
fn copy_bytes(source: &File, directory: &Path, name: &str, drop: usize) -> Result<(), CloneError> {
    let size = usize::try_from(source.metadata().map_err(|e| CloneError::Io(e.to_string()))?.len()).expect("size");
    let mut bytes = vec![0u8; size];
    source.read_exact_at(&mut bytes, 0).map_err(|e| CloneError::Io(e.to_string()))?;
    bytes.truncate(size.saturating_sub(drop));
    let mut out = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(directory.join(name))
        .map_err(|e| CloneError::Io(e.to_string()))?;
    out.write_all(&bytes).map_err(|e| CloneError::Io(e.to_string()))
}

impl RecorderStore for ProbeStore<'_> {
    type Handle = File;

    fn candidates(&self) -> Result<Vec<Candidate>, RecorderError> {
        let mut candidates = self.inner.candidates()?;
        for candidate in &mut candidates {
            candidate.flags |= self.extra_flags.get();
            if self.forced_dataless(&candidate.path) {
                candidate.flags |= SF_DATALESS;
            }
        }
        Ok(candidates)
    }

    fn candidate(&self, path: &Path) -> Result<Candidate, RecorderError> {
        let mut candidate = self.inner.candidate(path)?;
        candidate.flags |= self.extra_flags.get();
        if self.forced_dataless(path) {
            candidate.flags |= SF_DATALESS;
        }
        Ok(candidate)
    }

    fn open(&self, path: &Path) -> Result<File, RecorderError> {
        assert!(!self.forced_dataless(path), "a dataless entry was opened: {}", path.display());
        self.inner.open(path)
    }

    fn metadata(&self, handle: &File) -> Result<SourceMetadata, RecorderError> {
        let mut metadata = self.inner.metadata(handle)?;
        self.metadata_calls.set(self.metadata_calls.get() + 1);
        if self.grow_after_open.get() && self.metadata_calls.get() >= 2 {
            metadata.size += 1;
        }
        Ok(metadata)
    }

    fn read_at(&self, handle: &File, offset: u64, buf: &mut [u8]) -> Result<(), ReadFailure> {
        self.reads.set(self.reads.get() + 1);
        self.inner.read_at(handle, offset, buf)
    }

    fn clone_into(&self, handle: &File, directory: &Path, directory_identity: (u64, u64), name: &str) -> Result<CloneKind, CloneError> {
        match self.clone.get() {
            CloneBehaviour::Real => self.inner.clone_into(handle, directory, directory_identity, name),
            CloneBehaviour::ByteCopy => copy_bytes(handle, directory, name, 0).map(|()| CloneKind::ByteCopy),
            CloneBehaviour::Truncated => copy_bytes(handle, directory, name, 12).map(|()| CloneKind::CopyOnWrite),
            CloneBehaviour::Exists => Err(CloneError::Exists),
        }
    }

    fn title(&self, file_name: &str) -> TitleLookup {
        self.inner.title(file_name)
    }

    fn refresh_titles(&self) -> Result<(), RecorderError> {
        self.inner.refresh_titles()
    }

    fn subdirectory_counts(&self) -> Vec<(String, Option<u64>)> {
        self.inner.subdirectory_counts()
    }
}
```

`crates/vpt-adapters/tests/ingest_gates.rs`:

```rust
mod support;

use std::os::unix::fs::PermissionsExt;
use support::fakes::{CloneBehaviour, ProbeStore};
use support::{CAPTURED, Fixture, FixedClock, Parts, clock};
use vpt_application::ports::{Archive, RecordingLedger};
use vpt_application::{Ingest, Mode};
use vpt_domain::fixtures::{box_of, m4a, mvhd_v1};
use vpt_domain::notification::EventKind;
use vpt_domain::sweep::DeferralReason;
use vpt_domain::time::UtcInstant;

fn probed(parts: &Parts, probe: &ProbeStore<'_>) -> vpt_application::IngestReport {
    let ingest = Ingest {
        recorder: probe,
        archive: &parts.archive,
        ledger: &parts.ledger,
        clock: &parts.clock,
        trash: &parts.trash,
        notifier: &parts.notifier,
        settings: &parts.settings,
    };
    ingest.run(&Mode::default()).expect("sweep")
}

#[test]
fn dataless_never_opens() {
    let fixture = Fixture::new();
    let source = fixture.add_recording("cloud.m4a", &m4a(CAPTURED, 1, b"x"));
    let parts = fixture.parts();
    let probe = ProbeStore::new(&parts.store);
    probe.dataless.borrow_mut().push("cloud.m4a".into());
    probe.extra_flags.set(0x20);

    let report = probed(&parts, &probe);

    assert_eq!(report.deferred, vec![vpt_application::Deferred { path: source.clone(), reason: DeferralReason::Dataless }]);
    let row = parts.ledger.seen(&source).expect("seen").expect("row");
    assert_eq!((row.deferral_count, row.deferral_reason), (1, Some(DeferralReason::Dataless)));
    assert_eq!(row.flags & 0x40000020, 0x40000020);
    assert!(report.ingested.is_empty());
    assert!(parts.archive.archived().expect("archived").is_empty());
    assert!(parts.events().is_empty());
}

#[test]
fn oversize_never_reads() {
    let fixture = Fixture::new();
    fixture.add_recording("big.m4a", &m4a(CAPTURED, 1, &[0u8; 4_096]));
    let mut parts = fixture.parts();
    parts.settings.max_audio_bytes = 1_000;
    let probe = ProbeStore::new(&parts.store);

    let report = probed(&parts, &probe);

    assert_eq!(report.deferred[0].reason, DeferralReason::AudioTooLarge);
    assert_eq!(probe.reads.get(), 0);
    assert!(parts.archive.staged_leftovers().expect("leftovers").is_empty());
}

#[test]
fn a_truncated_download_is_deferred_then_not_at_rest_after_growing_then_ingested_when_still() {
    let fixture = Fixture::new();
    let whole = m4a(CAPTURED, 1, b"payload");
    let source = fixture.add_recording("a.m4a", &whole[..whole.len() - 12]);
    let parts = fixture.parts();

    let first = parts.ingest().run(&Mode::default()).expect("first");
    assert_eq!(first.deferred[0].reason, DeferralReason::InvalidContainer);
    assert!(parts.archive.archived().expect("archived").is_empty(), "nothing is cloned before every gate passes");

    fixture.add_recording("a.m4a", &whole);
    let second = parts.ingest().run(&Mode::default()).expect("second");
    assert_eq!(second.deferred[0].reason, DeferralReason::NotAtRest, "the size moved since the last deferral");
    assert!(second.ingested.is_empty());
    assert_eq!(parts.ledger.seen(&source).expect("seen").map(|row| row.deferral_count), Some(2));

    let third = parts.ingest().run(&Mode::default()).expect("third");
    assert_eq!(third.ingested.len(), 1);
    assert_eq!(parts.ledger.seen(&source).expect("seen").map(|row| (row.deferral_count, row.deferred_size)), Some((0, None)));
}

#[test]
fn a_file_younger_than_the_quiet_period_is_not_at_rest() {
    let fixture = Fixture::new();
    let path = fixture.add_recording("fresh.m4a", &m4a(CAPTURED, 1, b"x"));
    fixture.set_mtime(&path, clock().now.secs - 10);
    let parts = fixture.parts();

    let report = parts.ingest().run(&Mode::default()).expect("sweep");

    assert_eq!(report.deferred[0].reason, DeferralReason::NotAtRest);
    assert!(report.ingested.is_empty());
}

#[test]
fn the_deferred_event_is_raised_once_when_the_count_crosses_the_threshold() {
    let fixture = Fixture::new();
    let whole = m4a(CAPTURED, 1, b"payload");
    let source = fixture.add_recording("a.m4a", &whole[..whole.len() - 12]);
    let mut parts = fixture.parts();
    parts.settings.deferral_page_threshold = 2;

    for _ in 0..3 {
        parts.ingest().run(&Mode::default()).expect("sweep");
    }

    assert_eq!(parts.events(), vec![EventKind::Deferred]);
    assert_eq!(parts.ledger.seen(&source).expect("seen").map(|row| row.deferral_count), Some(3));
}

#[test]
fn an_edited_recording_is_deferred_while_fresh_then_ingested_as_a_second_recording() {
    let fixture = Fixture::new();
    let source = fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"first take"));
    let parts = fixture.parts();
    let first = parts.ingest().run(&Mode::default()).expect("first");
    let original = first.ingested[0].id.clone();
    std::fs::write(&source, m4a(CAPTURED + 30, 2, b"second take")).expect("edit");
    fixture.set_mtime(&source, clock().now.secs - 5);

    let second = parts.ingest().run(&Mode::default()).expect("second");
    assert_eq!(second.deferred[0].reason, DeferralReason::NotAtRest);
    assert_eq!(parts.ledger.recordings().expect("list").len(), 1);

    let later = Parts { clock: FixedClock { now: UtcInstant { secs: clock().now.secs + 60 }, ..clock() }, ..fixture.parts() };
    let third = later.ingest().run(&Mode::default()).expect("third");
    assert_eq!(third.ingested.len(), 1);
    assert_ne!(third.ingested[0].id, original);
    assert_eq!(later.ledger.recordings().expect("list").len(), 2);
    assert_eq!(later.ledger.seen(&source).expect("seen").and_then(|row| row.recording), Some(third.ingested[0].id.clone()));
}

#[test]
fn source_change_discards_stage() {
    let fixture = Fixture::new();
    let source = fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"moving"));
    let parts = fixture.parts();
    let probe = ProbeStore::new(&parts.store);
    probe.grow_after_open.set(true);

    let report = probed(&parts, &probe);

    assert_eq!(report.deferred[0].reason, DeferralReason::ChangedDuringRead);
    assert_eq!(parts.trash.moved.borrow().len(), 1, "the staged clone went to the Trash");
    assert!(parts.archive.staged_leftovers().expect("leftovers").is_empty());
    assert!(parts.archive.archived().expect("archived").is_empty());
    assert_eq!(parts.ledger.seen(&source).expect("seen").and_then(|row| row.recording), None);
    assert!(parts.events().is_empty());
}

#[test]
fn invalid_stage_is_trashed() {
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"whole at the source"));
    let parts = fixture.parts();
    let probe = ProbeStore::new(&parts.store);
    probe.clone.set(CloneBehaviour::Truncated);

    let report = probed(&parts, &probe);

    assert_eq!(report.deferred[0].reason, DeferralReason::InvalidContainer);
    assert_eq!(parts.trash.moved.borrow().len(), 1);
    assert!(parts.archive.staged_leftovers().expect("leftovers").is_empty());
    assert!(parts.ledger.recordings().expect("list").is_empty());
    assert!(parts.events().is_empty());
}

#[test]
fn absent_trash_preserves_private_stage() {
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"whole at the source"));
    let parts = fixture.parts();
    parts.trash.absent.set(true);
    let probe = ProbeStore::new(&parts.store);
    probe.clone.set(CloneBehaviour::Truncated);

    let report = probed(&parts, &probe);

    assert_eq!(report.deferred[0].reason, DeferralReason::InvalidContainer);
    let leftovers = parts.archive.staged_leftovers().expect("leftovers");
    assert_eq!(leftovers.len(), 1);
    assert_eq!(std::fs::metadata(&leftovers[0]).expect("meta").permissions().mode() & 0o777, 0o600);
    assert!(report.log.iter().any(|line| line.contains("cleanup pending")), "{:?}", report.log);
    assert!(parts.ledger.recordings().expect("list").is_empty());
    assert!(parts.events().is_empty());
}

#[test]
fn incompatible_cleanup_helper_refuses_without_losing_diagnostics_or_staging() {
    struct Incompatible;
    impl vpt_application::ports::Trash for Incompatible {
        fn trash(&self, _path: &std::path::Path) -> Result<std::path::PathBuf, vpt_application::ports::TrashError> {
            Err(vpt_application::ports::TrashError::HelperVersion { found: 7 })
        }
        fn take_diagnostics(&self) -> Vec<String> {
            vec!["unknown additive helper field: capabilities".into()]
        }
    }
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"whole at source"));
    let parts = fixture.parts();
    let probe = ProbeStore::new(&parts.store);
    probe.clone.set(CloneBehaviour::Truncated);
    let ingest = Ingest {
        recorder: &probe, archive: &parts.archive, ledger: &parts.ledger,
        clock: &parts.clock, trash: &Incompatible, notifier: &parts.notifier,
        settings: &parts.settings,
    };
    let error = ingest.run(&Mode::default()).expect_err("helper version refusal");
    assert_eq!(error.failure, vpt_application::IngestFailure::HelperVersion { found: 7 });
    assert!(error.completed.is_empty());
    assert!(error.log.iter().any(|line| line == "unknown additive helper field: capabilities"));
    assert_eq!(parts.archive.staged_leftovers().expect("staging").len(), 1);
    assert!(parts.ledger.recordings().expect("rows").is_empty());
}

#[test]
fn a_capture_instant_with_no_four_digit_year_is_deferred_as_an_invalid_container() {
    let fixture = Fixture::new();
    let mut bytes = box_of(b"ftyp", b"M4A ");
    bytes.extend(box_of(b"mdat", b"audio"));
    bytes.extend(box_of(b"moov", &mvhd_v1(253_402_300_800, 1)));
    fixture.add_recording("far.m4a", &bytes);
    let mut parts = fixture.parts();
    parts.clock.offset = vpt_domain::time::UtcOffset { secs: 0 };

    let report = parts.ingest().run(&Mode::default()).expect("sweep");

    assert_eq!(report.deferred[0].reason, DeferralReason::InvalidContainer);
    assert_eq!(parts.trash.moved.borrow().len(), 1);
    assert!(parts.ledger.recordings().expect("list").is_empty());
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-adapters --test ingest_gates`

Expected: `dataless_never_opens` PANICS in the probe (the entry is opened); `oversize_never_reads`, the
truncated-download test, the quiet-period test and the edited-recording test FAIL because the entries are
ingested or fail outright; the threshold test FAILS with no events; `source_change_discards_stage` and
`invalid_stage_is_trashed` FAIL with a `Recorder` or `Archive` error where a deferral is expected; the
absent-trash and far-future tests FAIL the same way.

- [ ] **Step 3: Write the minimal implementation**

Replace `process` in `crates/vpt-application/src/ingest/candidate.rs` and add `defer` to the same `impl`
block:

```rust
    pub(super) fn process(&self, candidate: &Candidate, mode: &Mode, report: &mut IngestReport) -> Result<(), IngestFailure> {
        let now = self.clock.now();
        let seen = self.ledger.seen(&candidate.path).map_err(IngestFailure::Ledger)?;
        let facts = CandidateFacts { size: candidate.size, mtime: candidate.mtime, flags: candidate.flags };
        let seen_facts = seen.as_ref().map(|row| SeenFacts {
            size: row.size,
            mtime: row.mtime,
            ingested: row.recording.is_some() && row.deferral_reason.is_none(),
            deferred_size: row.deferred_size,
        });
        let seen = match pre_open(&facts, seen_facts.as_ref()) {
            PreOpen::Dataless => return self.defer(candidate, seen, DeferralReason::Dataless, now, mode, report),
            PreOpen::Unchanged => match seen {
                Some(row) => return self.unchanged(row, now, mode, report),
                None => None,
            },
            PreOpen::Open => seen,
        };
        let handle = self.recorder.open(&candidate.path).map_err(recorder_failure)?;
        let metadata = self.recorder.metadata(&handle).map_err(recorder_failure)?;
        let limits = SweepLimits { max_audio_bytes: self.settings.max_audio_bytes, quiet_period_secs: self.settings.quiet_period_secs };
        if let Err(reason) = size_gate(metadata.size, &limits) {
            return self.defer(candidate, seen, reason, now, mode, report);
        }
        if inspect(metadata.size, |offset, buf| self.recorder.read_at(&handle, offset, buf)).is_err() {
            return self.defer(candidate, seen, DeferralReason::InvalidContainer, now, mode, report);
        }
        if let Err(reason) = rest_gate(metadata.mtime, metadata.size, now, seen_facts.as_ref(), &limits) {
            return self.defer(candidate, seen, reason, now, mode, report);
        }
        self.stage_and_publish(candidate, &handle, &metadata, seen, now, mode, report)
    }

    /// Record one deferral and page exactly when the count crosses the threshold.
    pub(super) fn defer(
        &self,
        candidate: &Candidate,
        seen: Option<SeenRow>,
        reason: DeferralReason,
        now: UtcInstant,
        mode: &Mode,
        report: &mut IngestReport,
    ) -> Result<(), IngestFailure> {
        report.deferred.push(super::Deferred { path: candidate.path.clone(), reason });
        if mode.dry_run {
            return Ok(());
        }
        let mut row = seen.unwrap_or_else(|| fresh_seen(candidate, now));
        row.size = candidate.size;
        row.mtime = candidate.mtime;
        row.flags = candidate.flags;
        row.last_seen = now;
        row.source_gone_at = None;
        let previous = row.deferral_count;
        row.deferral_count = previous.saturating_add(1);
        row.deferral_reason = Some(reason);
        row.deferred_size = Some(candidate.size);
        self.ledger.record_seen(&row).map_err(IngestFailure::Ledger)?;
        let threshold = self.settings.deferral_page_threshold;
        if previous < threshold && row.deferral_count >= threshold {
            self.notifier.deliver(&Notification::attention(
                EventKind::Deferred,
                None,
                format!("deferred {} times: {}", row.deferral_count, reason.as_str()),
                vec![("source".into(), candidate.path.clone())],
                now,
            ));
            report.log.extend(self.notifier.take_diagnostics());
        }
        Ok(())
    }
```

with these imports in `candidate.rs` replacing the earlier `vpt_domain::sweep` line:

```rust
use vpt_domain::container::inspect;
use vpt_domain::notification::{EventKind, Notification};
use vpt_domain::sweep::{CandidateFacts, DeferralReason, PreOpen, SeenFacts, SweepLimits, pre_open, rest_gate, size_gate};
```

In `publish.rs` the two staged gates and the identity become deferrals. Replace the block from
`let after = ...` to the `let Ok(id) = ...` binding with:

```rust
        let after = self.owned(&staged.path, self.recorder.metadata(handle).map_err(recorder_failure), report)?;
        if after != *metadata {
            self.discard(&staged.path, report)?;
            return self.defer(candidate, seen, DeferralReason::ChangedDuringRead, now, mode, report);
        }
        let container = match self.owned(&staged.path, self.inspect_archive(&staged.path), report)? {
            Ok(container) => container,
            Err(_) => {
                self.discard(&staged.path, report)?;
                return self.defer(candidate, seen, DeferralReason::InvalidContainer, now, mode, report);
            }
        };
        let offset = self.clock.offset_at(container.creation_time);
        let Ok(id) = RecordingId::derive(container.creation_time, offset, &staged.digest) else {
            self.discard(&staged.path, report)?;
            return self.defer(candidate, seen, DeferralReason::InvalidContainer, now, mode, report);
        };
```

delete the `let _ = mode;` line, and add `use vpt_domain::sweep::DeferralReason;` to its imports. A
capture instant whose local year has no four-digit form is the container's fault, so it defers as
`invalid_container` (spec section 4.3 derives the identity from the container alone).

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo fmt --all`

Run: `cargo test -p vpt-adapters --test ingest_gates --test ingest_sweep`

Expected: all gate and cleanup tests PASS. Run
`cargo clippy --workspace --all-targets --features dev-tools -- -D warnings` and expect no warnings.

- [ ] **Step 5: Commit**

Run: `cargo fmt --all`

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(ingest): the sweep gates defer, retry and page at the threshold"
```

______________________________________________________________________

### Task 21: Duplicates, recovery and the archive collision

**Files:**

- Modify: `crates/vpt-application/src/ingest/publish.rs`
- Test: `crates/vpt-adapters/tests/support/fakes.rs`, `crates/vpt-adapters/tests/ingest_duplicates.rs`

**Interfaces:**

- Consumes: `Archive::{digest, sync_existing}`, `RecordingLedger::{by_digest, by_id, commit,`
  `set_source_path}`.

- Produces: `Ingest::known_bytes` and `Ingest::resolve_existing` (private to the module). Test support in
  `support/fakes.rs`: `FaultyArchive<'a>::new(inner: &'a ClonefileArchive) -> FaultyArchive` implementing
  `Archive` over the real archive with `pub fault: Cell<ArchiveFault>` and
  `pub reject_trash: Cell<bool>`, `ArchiveFault::{None, NoSpaceAfterCreate, FileSync, DirectorySync}`.

- [ ] **Step 1: Write the failing tests**

Append to `crates/vpt-adapters/tests/support/fakes.rs`:

```rust
use vpt_adapters::{ClonefileArchive, STAGING_PREFIX};
use vpt_application::ports::{Archive, ArchiveError, Published, StageFailure, Staged};
use vpt_domain::digest::Sha256Digest;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFault {
    None,
    NoSpaceAfterCreate,
    FileSync,
    DirectorySync,
}

pub struct FaultyArchive<'a> {
    inner: &'a ClonefileArchive,
    pub fault: Cell<ArchiveFault>,
    pub reject_trash: Cell<bool>,
}

impl<'a> FaultyArchive<'a> {
    pub fn new(inner: &'a ClonefileArchive) -> FaultyArchive<'a> {
        FaultyArchive { inner, fault: Cell::new(ArchiveFault::None), reject_trash: Cell::new(false) }
    }
}

impl Archive for FaultyArchive<'_> {
    type Handle = File;

    fn stage<C>(&self, clone: C) -> Result<Staged, StageFailure>
    where
        C: FnOnce(&Path, (u64, u64), &str) -> Result<CloneKind, CloneError>,
    {
        if self.fault.get() != ArchiveFault::NoSpaceAfterCreate {
            return self.inner.stage(clone);
        }
        let path = self.inner.target(&format!("{STAGING_PREFIX}{}-fault.m4a", std::process::id()));
        let mut partial = std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&path).expect("partial staging");
        partial.write_all(b"partial").expect("partial bytes");
        Err(StageFailure { cause: ArchiveError::NoSpace, owned_staging: Some(path) })
    }

    fn open(&self, path: &Path) -> Result<File, ArchiveError> {
        self.inner.open(path)
    }

    fn size(&self, handle: &File) -> Result<u64, ArchiveError> {
        self.inner.size(handle)
    }

    fn read_at(&self, handle: &File, offset: u64, buf: &mut [u8]) -> Result<(), ReadFailure> {
        self.inner.read_at(handle, offset, buf)
    }

    fn digest(&self, path: &Path) -> Result<Sha256Digest, ArchiveError> {
        self.inner.digest(path)
    }

    fn validate_trash_path(&self, path: &Path) -> Result<(), ArchiveError> {
        if self.reject_trash.get() { return Err(ArchiveError::Escape(path.to_path_buf())); }
        self.inner.validate_trash_path(path)
    }

    fn target(&self, name: &str) -> std::path::PathBuf {
        self.inner.target(name)
    }

    fn archived(&self) -> Result<Vec<std::path::PathBuf>, ArchiveError> {
        self.inner.archived()
    }

    fn staged_leftovers(&self) -> Result<Vec<std::path::PathBuf>, ArchiveError> {
        self.inner.staged_leftovers()
    }

    fn publish(&self, staged: &Path, target_name: &str) -> Result<Published, ArchiveError> {
        match self.fault.get() {
            ArchiveFault::FileSync => Err(ArchiveError::Sync("staged file".into())),
            ArchiveFault::DirectorySync => {
                self.inner.publish(staged, target_name)?;
                Err(ArchiveError::PlacedUnsynced(self.inner.target(target_name)))
            }
            ArchiveFault::None | ArchiveFault::NoSpaceAfterCreate => self.inner.publish(staged, target_name),
        }
    }

    fn sync_existing(&self, path: &Path) -> Result<(), ArchiveError> {
        self.inner.sync_existing(path)
    }
}
```

`crates/vpt-adapters/tests/ingest_duplicates.rs`:

```rust
mod support;

use std::os::unix::fs::PermissionsExt;
use support::fakes::{ArchiveFault, CloneBehaviour, FaultyArchive, ProbeStore};
use support::{CAPTURED, Fixture, Parts};
use vpt_application::ports::{Archive, RecordingLedger};
use vpt_application::{Ingest, IngestError, IngestFailure, IngestReport, Mode};
use vpt_domain::fixtures::m4a;
use vpt_domain::notification::EventKind;

fn lose_the_ledger(fixture: &Fixture) {
    for name in ["vpt.db", "vpt.db-wal", "vpt.db-shm"] {
        let _ = std::fs::remove_file(fixture.state.join(name));
    }
}

fn with_archive(parts: &Parts, archive: &FaultyArchive<'_>) -> Result<IngestReport, Box<IngestError>> {
    let ingest = Ingest {
        recorder: &parts.store,
        archive,
        ledger: &parts.ledger,
        clock: &parts.clock,
        trash: &parts.trash,
        notifier: &parts.notifier,
        settings: &parts.settings,
    };
    ingest.run(&Mode::default())
}

#[test]
fn the_same_bytes_at_a_second_path_are_one_recording_whose_source_path_moves() {
    let fixture = Fixture::new();
    fixture.add_recording("first.m4a", &m4a(CAPTURED, 1, b"same"));
    let parts = fixture.parts();
    parts.ingest().run(&Mode::default()).expect("first");
    std::fs::rename(fixture.recordings.join("first.m4a"), fixture.recordings.join("renamed.m4a")).expect("rename");

    let report = parts.ingest().run(&Mode::default()).expect("second");

    assert_eq!(report.already_ingested.len(), 1);
    assert!(report.ingested.is_empty());
    let record = parts.ledger.recordings().expect("list").remove(0);
    assert_eq!(record.source_path, Some(fixture.recordings.join("renamed.m4a")));
    assert_eq!(parts.archive.archived().expect("archived").len(), 1);
    assert!(parts.archive.staged_leftovers().expect("leftovers").is_empty());
    assert_eq!(parts.trash.moved.borrow().len(), 1, "the staged duplicate went to the Trash");
    assert!(parts.events().is_empty());
}

#[test]
fn an_archive_without_a_ledger_row_is_recovered_on_publish_and_reported_as_already_ingested() {
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"orphan"));
    let first = fixture.parts().ingest().run(&Mode::default()).expect("first");
    let id = first.ingested[0].id.clone();
    lose_the_ledger(&fixture);
    let parts = fixture.parts();

    let report = parts.ingest().run(&Mode::default()).expect("second");

    assert!(report.already_ingested.contains(&id) || report.recovered.contains(&id), "{report:?}");
    assert!(parts.ledger.by_id(&id).expect("read").is_some());
    assert_eq!(parts.archive.archived().expect("archived").len(), 1);
    assert!(parts.archive.staged_leftovers().expect("leftovers").is_empty());
}

#[test]
fn a_target_with_different_bytes_is_an_archive_collision_and_nothing_is_replaced() {
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"real"));
    let first = fixture.parts().ingest().run(&Mode::default()).expect("first");
    let record = first.ingested[0].clone();
    std::fs::write(&record.audio_path, b"tampered").expect("tamper the archive in the test only");
    lose_the_ledger(&fixture);
    let parts = fixture.parts();

    let error = parts.ingest().run(&Mode::default()).unwrap_err();

    assert!(matches!(error.failure, IngestFailure::ArchiveCollision { .. }), "{error:?}");
    assert_eq!(std::fs::read(&record.audio_path).expect("read"), b"tampered");
    assert!(parts.archive.staged_leftovers().expect("leftovers").is_empty());
    assert_eq!(parts.trash.moved.borrow().len(), 1);
    assert!(parts.ledger.recordings().expect("list").is_empty());
    assert_eq!(parts.events(), vec![EventKind::IngestFailed]);
}

#[test]
fn exdev_uses_bounded_copy() {
    let fixture = Fixture::new();
    let bytes = m4a(CAPTURED, 1, b"other volume");
    fixture.add_recording("a.m4a", &bytes);
    let parts = fixture.parts();
    let probe = ProbeStore::new(&parts.store);
    probe.clone.set(CloneBehaviour::ByteCopy);
    let ingest = Ingest {
        recorder: &probe,
        archive: &parts.archive,
        ledger: &parts.ledger,
        clock: &parts.clock,
        trash: &parts.trash,
        notifier: &parts.notifier,
        settings: &parts.settings,
    };

    let report = ingest.run(&Mode::default()).expect("sweep");

    assert_eq!(report.ingested.len(), 1);
    let record = &report.ingested[0];
    assert_eq!(std::fs::read(&record.audio_path).expect("archive"), bytes);
    assert_eq!(std::fs::metadata(&record.audio_path).expect("meta").permissions().mode() & 0o777, 0o600);
    assert!(report.log.iter().any(|line| line.contains("not copy-on-write")), "{:?}", report.log);
    assert!(parts.archive.staged_leftovers().expect("leftovers").is_empty());
    assert!(parts.events().is_empty());
}

#[test]
fn enospc_aborts_without_row() {
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"a"));
    let parts = fixture.parts();
    let archive = FaultyArchive::new(&parts.archive);
    archive.fault.set(ArchiveFault::NoSpaceAfterCreate);

    let error = with_archive(&parts, &archive).unwrap_err();

    assert_eq!(error.failure, IngestFailure::NoSpace);
    assert!(parts.ledger.recordings().expect("list").is_empty());
    assert_eq!(parts.trash.moved.borrow().len(), 1, "the partial staging this run created was trashed");
    assert!(parts.archive.staged_leftovers().expect("leftovers").is_empty());
    assert_eq!(parts.events(), vec![EventKind::IngestFailed]);
}

#[test]
fn stage_sync_failure_commits_no_row() {
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"a"));
    let parts = fixture.parts();
    let archive = FaultyArchive::new(&parts.archive);
    archive.fault.set(ArchiveFault::FileSync);

    let error = with_archive(&parts, &archive).unwrap_err();

    assert_eq!(error.failure, IngestFailure::Sync("staged file".into()));
    assert!(parts.ledger.recordings().expect("list").is_empty());
    assert!(parts.archive.archived().expect("archived").is_empty());
    assert_eq!(parts.trash.moved.borrow().len(), 1);
    assert!(parts.archive.staged_leftovers().expect("leftovers").is_empty());
    assert_eq!(parts.events(), vec![EventKind::IngestFailed]);
}

#[test]
fn directory_sync_failure_commits_no_row() {
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"a"));
    let parts = fixture.parts();
    let archive = FaultyArchive::new(&parts.archive);
    archive.fault.set(ArchiveFault::DirectorySync);

    let error = with_archive(&parts, &archive).unwrap_err();

    assert!(matches!(error.failure, IngestFailure::Sync(_)), "{error:?}");
    assert!(parts.ledger.recordings().expect("list").is_empty());
    assert_eq!(parts.archive.archived().expect("archived").len(), 1, "the placed file stays for recovery");
    assert!(parts.trash.moved.borrow().is_empty());
    assert!(parts.archive.staged_leftovers().expect("leftovers").is_empty());
    assert_eq!(parts.events(), vec![EventKind::IngestFailed]);
}
#[test]
fn a_known_digest_keeps_its_identity_when_the_new_zone_cannot_form_an_id() {
    let fixture = Fixture::new();
    let mut bytes = vpt_domain::fixtures::box_of(b"ftyp", b"M4A ");
    bytes.extend(vpt_domain::fixtures::box_of(b"mdat", b"audio"));
    bytes.extend(vpt_domain::fixtures::box_of(b"moov", &vpt_domain::fixtures::mvhd_v1(253_402_300_800, 1)));
    let source = fixture.add_recording("first.m4a", &bytes);
    let mut parts = fixture.parts();
    parts.clock.offset = vpt_domain::time::UtcOffset { secs: -3_600 };
    let first = parts.ingest().run(&Mode::default()).expect("first");
    let id = first.ingested[0].id.clone();
    std::fs::rename(source, fixture.recordings.join("renamed.m4a")).expect("rename");
    parts.clock.offset = vpt_domain::time::UtcOffset { secs: 0 };
    let report = parts.ingest().run(&Mode::default()).expect("known bytes");
    assert_eq!(report.already_ingested, vec![id]);
    assert!(report.deferred.is_empty());
    assert_eq!(parts.ledger.recordings().expect("list").len(), 1);
}

#[test]
fn invalid_staging_is_never_sent_to_trash_when_archive_validation_refuses_it() {
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"whole source"));
    let parts = fixture.parts();
    let probe = ProbeStore::new(&parts.store);
    probe.clone.set(CloneBehaviour::Truncated);
    let archive = FaultyArchive::new(&parts.archive);
    archive.reject_trash.set(true);
    let error = Ingest {
        recorder: &probe, archive: &archive, ledger: &parts.ledger, clock: &parts.clock,
        trash: &parts.trash, notifier: &parts.notifier, settings: &parts.settings,
    }.run(&Mode::default()).expect_err("path refused");
    assert!(matches!(error.failure, IngestFailure::PathEscape(_)));
    assert!(parts.trash.moved.borrow().is_empty());
    assert_eq!(parts.archive.staged_leftovers().expect("staging").len(), 1);
}

```

The recovery and collision tests delete database files, which the spec forbids vpt itself from doing; the
test harness may, because it is simulating a lost ledger inside a temporary directory the test owns.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-adapters --test ingest_duplicates`

Expected: the rename test FAILS (the sweep reports `Archive("... exists")` because the same bytes derive
the same identity); the recovery test FAILS the same way; the collision test FAILS because the failure is
`Archive`, not `ArchiveCollision`; the four fault tests PASS already on the Task 19 cleanup contract and
stay as regression guards.

- [ ] **Step 3: Write the minimal implementation**

In `crates/vpt-application/src/ingest/publish.rs`, remove the earlier `let offset` and `let Ok(id)` block
added by Task 20. Insert this complete block after the copy-on-write diagnostic and before
`archive.publish`; known digests return before any timezone-dependent identity derivation:

```rust
        let existing = self.owned(&staged.path, self.ledger.by_digest(&staged.digest).map_err(ledger_failure), report)?;
        if let Some(existing) = existing {
            return self.known_bytes(candidate, existing, &staged, seen, now, report);
        }
        let offset = self.clock.offset_at(container.creation_time);
        let Ok(id) = RecordingId::derive(container.creation_time, offset, &staged.digest) else {
            self.discard(&staged.path, report)?;
            return self.defer(candidate, seen, DeferralReason::InvalidContainer, now, mode, report);
        };
```

Replace the `Published::Exists` arm:

```rust
            Ok(Published::Exists(audio_path)) => {
                self.resolve_existing(candidate, id, container, offset, &staged, audio_path, seen, now, report)
            }
```

and add the two methods to the `impl` block:

```rust
    /// Known bytes at a new path: the recording keeps its identity, its
    /// source path follows, the staged duplicate goes to the Trash.
    fn known_bytes(
        &self,
        candidate: &Candidate,
        existing: RecordingRecord,
        staged: &Staged,
        seen: Option<SeenRow>,
        now: UtcInstant,
        report: &mut IngestReport,
    ) -> Result<(), IngestFailure> {
        if existing.source_path.as_deref() != Some(candidate.path.as_path()) {
            let moved = self.ledger.set_source_path(&existing.id, &candidate.path).map_err(ledger_failure);
            self.owned(&staged.path, moved, report)?;
            report.log.push(format!("{}: known bytes at a new path, source path updated", existing.id));
        }
        let row = ingested_seen(candidate, seen, &existing.id, now);
        self.owned(&staged.path, self.ledger.record_seen(&row).map_err(ledger_failure), report)?;
        self.discard(&staged.path, report)?;
        report.already_ingested.push(existing.id);
        Ok(())
    }

    /// `<id>.m4a` already exists: verify it byte for byte, sync it, recover
    /// any missing rows, trash the staged duplicate.
    #[allow(clippy::too_many_arguments)]
    fn resolve_existing(
        &self,
        candidate: &Candidate,
        id: RecordingId,
        container: Container,
        offset: UtcOffset,
        staged: &Staged,
        audio_path: PathBuf,
        seen: Option<SeenRow>,
        now: UtcInstant,
        report: &mut IngestReport,
    ) -> Result<(), IngestFailure> {
        let existing = self.owned(&staged.path, self.archive.digest(&audio_path).map_err(archive_failure), report)?;
        let valid = self.owned(&staged.path, self.inspect_archive(&audio_path), report)?.is_ok();
        if existing != staged.digest || !valid {
            self.discard(&staged.path, report)?;
            return Err(IngestFailure::ArchiveCollision { target: audio_path, staged: staged.digest, existing });
        }
        self.owned(&staged.path, self.archive.sync_existing(&audio_path).map_err(archive_failure), report)?;
        let known = self.owned(&staged.path, self.ledger.by_id(&id).map_err(ledger_failure), report)?;
        let row = ingested_seen(candidate, seen, &id, now);
        if known.is_none() {
            let (title, title_source) = lookup_title(self.recorder.title(&candidate.file_name));
            let record = RecordingRecord {
                id: id.clone(),
                source_path: Some(candidate.path.clone()),
                digest: staged.digest,
                captured_at: container.creation_time,
                captured_offset: offset,
                duration_secs: container.duration_secs,
                title,
                title_source,
                ingested_at: now,
                audio_path,
                stages: StageStates::fresh(),
                audio_trashed_at: None,
            };
            let batch = LedgerCommit { recordings: vec![record], seen: vec![row], publications: vec![] };
            self.owned(&staged.path, self.ledger.commit(&batch).map_err(ledger_failure), report)?;
            report.recovered.push(id.clone());
        } else {
            self.owned(&staged.path, self.ledger.record_seen(&row).map_err(ledger_failure), report)?;
        }
        self.discard(&staged.path, report)?;
        report.already_ingested.push(id);
        Ok(())
    }
```

with these imports added to `publish.rs`: `Staged` in the `crate::ports` list, `use std::path::PathBuf;`
(beside `Path`), and `use vpt_domain::time::UtcOffset;` beside `UtcInstant`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo fmt --all`

Run: `cargo test -p vpt-adapters --test ingest_duplicates`

Expected: 9 duplicate and cleanup tests PASS. Run
`cargo clippy --workspace --all-targets --features dev-tools -- -D warnings` and expect no warnings.

- [ ] **Step 5: Commit**

Run: `cargo fmt --all`

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(ingest): duplicates recover their rows and a collision refuses"
```

______________________________________________________________________

### Task 22: Vanished sources and orphaned archives

An archive file with no ledger row is recovered at the start of every sweep, its source present or not.
Its identity is the file name's own: the hash half must equal the digest of the whole file, and the
timestamp half, read back as a civil time, must sit within a day of the container's capture instant,
which yields the zone the file was named in. The machine's current zone plays no part, so an archive made
in another zone recovers with the offset it was made in.

**Files:**

- Create: `crates/vpt-application/src/ingest/recovery.rs`
- Modify: `crates/vpt-application/src/ingest/mod.rs`
- Test: `crates/vpt-adapters/tests/ingest_recovery.rs`

**Interfaces:**

- Consumes: `RecordingLedger::{seen_all, record_seen, by_id, commit}`,
  `Archive::{archived, open, size, read_at, digest, sync_existing}`,
  `vpt_domain::identity::parse_local_timestamp`, `Civil::instant`.

- Produces: `Ingest::recover_orphans` and `Ingest::mark_gone` (private), and the free function
  `recovery::archived_offset(id: &RecordingId, captured: UtcInstant) -> Option<UtcOffset>`.

- [ ] **Step 1: Write the failing tests**

`crates/vpt-adapters/tests/ingest_recovery.rs`:

```rust
mod support;

use support::{CAPTURED, Fixture, clock, digest_of};
use vpt_application::Mode;
use vpt_application::ports::{Archive, RecordingLedger};
use vpt_domain::fixtures::m4a;
use vpt_domain::identity::RecordingId;
use vpt_domain::time::{UtcInstant, UtcOffset};

#[test]
fn a_recording_whose_source_disappeared_is_marked_gone_and_its_clone_stays() {
    let fixture = Fixture::new();
    let source = fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"a"));
    let parts = fixture.parts();
    let first = parts.ingest().run(&Mode::default()).expect("first");
    fixture.add_recording("survivor.m4a", &m4a(CAPTURED + 1, 1, b"survivor"));
    std::fs::remove_file(&source).expect("the operator deleted the memo; test setup only");

    parts.ingest().run(&Mode::default()).expect("second");

    let row = parts.ledger.seen(&source).expect("seen").expect("row");
    assert_eq!(row.source_gone_at, Some(clock().now));
    assert!(first.ingested[0].audio_path.exists());
    assert!(parts.events().is_empty());
}

#[test]
fn an_archive_with_no_ledger_row_is_recovered_at_the_start_of_the_sweep_in_the_zone_it_was_named_in() {
    let fixture = Fixture::new();
    let bytes = m4a(CAPTURED, 7, b"orphan");
    let digest = digest_of(&bytes);
    let elsewhere = UtcOffset { secs: 19_800 };
    let id = RecordingId::derive(UtcInstant { secs: CAPTURED }, elsewhere, &digest).expect("id");
    std::fs::write(fixture.audio.join(format!("{id}.m4a")), &bytes).expect("orphan archive");
    let parts = fixture.parts();

    let report = parts.ingest().run(&Mode::default()).expect("sweep");

    assert_eq!(report.recovered, vec![id.clone()]);
    let record = parts.ledger.by_id(&id).expect("read").expect("recovered");
    assert_eq!(record.source_path, None);
    assert_eq!(record.duration_secs, 7);
    assert_eq!(record.digest, digest);
    assert_eq!(record.captured_at, UtcInstant { secs: CAPTURED });
    assert_eq!(record.captured_offset, elsewhere);
    assert_eq!(record.audio_path, fixture.audio.join(format!("{id}.m4a")));
}

#[test]
fn an_orphan_named_in_the_local_zone_recovers_with_that_offset_too() {
    let fixture = Fixture::new();
    let bytes = m4a(CAPTURED, 3, b"local orphan");
    let id = RecordingId::derive(UtcInstant { secs: CAPTURED }, clock().offset, &digest_of(&bytes)).expect("id");
    std::fs::write(fixture.audio.join(format!("{id}.m4a")), &bytes).expect("orphan archive");
    let parts = fixture.parts();

    let report = parts.ingest().run(&Mode::default()).expect("sweep");

    assert_eq!(report.recovered, vec![id.clone()]);
    assert_eq!(parts.ledger.by_id(&id).expect("read").expect("recovered").captured_offset, UtcOffset { secs: -21_600 });
}

#[test]
fn an_archive_file_whose_name_does_not_match_its_bytes_is_logged_and_left_alone() {
    let fixture = Fixture::new();
    std::fs::write(fixture.audio.join("2026-08-24T144736-000000000000.m4a"), m4a(CAPTURED, 1, b"x")).expect("wrong hash");
    let far = RecordingId::derive(UtcInstant { secs: CAPTURED + 200_000 }, clock().offset, &digest_of(&m4a(CAPTURED, 1, b"y"))).expect("id");
    std::fs::write(fixture.audio.join(format!("{far}.m4a")), m4a(CAPTURED, 1, b"y")).expect("wrong timestamp");
    let parts = fixture.parts();

    let report = parts.ingest().run(&Mode::default()).expect("sweep");

    assert!(report.recovered.is_empty());
    assert!(report.log.iter().any(|line| line.contains("does not match its bytes")), "{:?}", report.log);
    assert!(report.log.iter().any(|line| line.contains("capture instant")), "{:?}", report.log);
    assert!(parts.ledger.recordings().expect("list").is_empty());
    assert_eq!(parts.archive.archived().expect("archived").len(), 2);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-adapters --test ingest_recovery`

Expected: the gone test FAILS on `source_gone_at == None`; the two orphan tests FAIL on an empty
`recovered`; the mismatch test FAILS on an empty log.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-application/src/ingest/recovery.rs`:

```rust
//! Before the candidates: recover archive files with no row, and after them:
//! mark the sources that vanished.

use super::publish::archive_failure;
use super::{Ingest, IngestFailure, IngestReport};
use crate::ports::{Archive, Candidate, Clock, LedgerCommit, Notifier, RecorderStore, RecordingLedger, RecordingRecord, StageStates, TitleOrigin, Trash};
use vpt_domain::identity::{RecordingId, parse_local_timestamp};
use vpt_domain::time::{UtcInstant, UtcOffset};

impl<R, A, L, C, T, N> Ingest<'_, R, A, L, C, T, N>
where
    R: RecorderStore,
    A: Archive,
    L: RecordingLedger,
    C: Clock,
    T: Trash + ?Sized,
    N: Notifier + ?Sized,
{
    pub(super) fn recover_orphans(&self, report: &mut IngestReport) -> Result<(), IngestFailure> {
        for path in self.archive.archived().map_err(archive_failure)? {
            let name = path.file_stem().and_then(|stem| stem.to_str()).unwrap_or_default();
            let Ok(named) = RecordingId::parse(name) else {
                report.log.push(format!("{}: not an archive name, left alone", path.display()));
                continue;
            };
            if self.ledger.by_id(&named).map_err(IngestFailure::Ledger)?.is_some() {
                continue;
            }
            let container = match self.inspect_archive(&path)? {
                Ok(container) => container,
                Err(error) => {
                    report.log.push(format!("{}: container invalid ({error:?}), left alone", path.display()));
                    continue;
                }
            };
            let digest = self.archive.digest(&path).map_err(archive_failure)?;
            if digest.hash12() != named.hash12() {
                report.log.push(format!("{}: name does not match its bytes, left alone", path.display()));
                continue;
            }
            let Some(offset) = archived_offset(&named, container.creation_time) else {
                report.log.push(format!("{}: name does not match its capture instant, left alone", path.display()));
                continue;
            };
            let record = RecordingRecord {
                id: named.clone(),
                source_path: None,
                digest,
                captured_at: container.creation_time,
                captured_offset: offset,
                duration_secs: container.duration_secs,
                title: None,
                title_source: TitleOrigin::Unavailable,
                ingested_at: self.clock.now(),
                audio_path: path.clone(),
                stages: StageStates::fresh(),
                audio_trashed_at: None,
            };
            self.archive.sync_existing(&path).map_err(archive_failure)?;
            let batch = LedgerCommit { recordings: vec![record], ..LedgerCommit::default() };
            self.ledger.commit(&batch).map_err(IngestFailure::Ledger)?;
            report.log.push(format!("{named}: archive recovered into the ledger"));
            report.recovered.push(named);
        }
        Ok(())
    }

    pub(super) fn mark_gone(&self, candidates: &[Candidate], report: &mut IngestReport) -> Result<(), IngestFailure> {
        let now = self.clock.now();
        for mut row in self.ledger.seen_all().map_err(IngestFailure::Ledger)? {
            let present = candidates.iter().any(|candidate| candidate.path == row.path);
            if row.recording.is_some() && row.source_gone_at.is_none() && !present {
                row.source_gone_at = Some(now);
                self.ledger.record_seen(&row).map_err(IngestFailure::Ledger)?;
                report.log.push(format!("{}: source gone", row.path.display()));
            }
        }
        Ok(())
    }
}

/// The zone an archive was named in: its local timestamp read as a civil
/// time, against the container's instant, when the difference is under a day.
pub(super) fn archived_offset(id: &RecordingId, captured: UtcInstant) -> Option<UtcOffset> {
    let civil = parse_local_timestamp(id.local_timestamp()).ok()?;
    let as_utc = civil.instant(UtcOffset { secs: 0 })?;
    let offset = i32::try_from(i128::from(as_utc.secs) - i128::from(captured.secs)).ok()?;
    (offset.unsigned_abs() < 86_400).then_some(UtcOffset { secs: offset })
}
```

In `crates/vpt-application/src/ingest/mod.rs`, add `mod recovery;` beside the other two and make `sweep`
call both ends:

```rust
    fn sweep(&self, mode: &Mode, report: &mut IngestReport) -> Result<(), IngestFailure> {
        if !mode.dry_run {
            self.recorder.refresh_titles().map_err(candidate::recorder_failure)?;
        }
        if !mode.dry_run {
            self.recover_orphans(report)?;
        }
        let candidates = match &mode.once {
            Some(path) => vec![self.recorder.candidate(path).map_err(candidate::recorder_failure)?],
            None => self.recorder.candidates().map_err(candidate::recorder_failure)?,
        };
        for candidate in &candidates {
            self.process(candidate, mode, report)?;
        }
        if mode.once.is_none() && !mode.dry_run {
            self.mark_gone(&candidates, report)?;
        }
        Ok(())
    }
```

`inspect_archive` and `archive_failure` are the ones `publish.rs` already defines.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo fmt --all`

Run: `cargo test -p vpt-adapters --test 'ingest_*'`

Expected: all PASS. Run `cargo clippy --workspace --all-targets --features dev-tools -- -D warnings` and
expect no warnings.

- [ ] **Step 5: Commit**

Run: `cargo fmt --all`

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(ingest): mark vanished sources and recover orphaned archives"
```

______________________________________________________________________

### Task 23: Dry run, `--once`, and the aborts

**Files:**

- Modify: `crates/vpt-application/src/ingest/mod.rs`, `crates/vpt-application/src/ingest/candidate.rs`,
  `crates/vpt-application/src/ports/recorder.rs`, `crates/vpt-adapters/src/voice_memos/store.rs`,
  `crates/vpt-adapters/tests/support/fakes.rs`
- Test: `crates/vpt-adapters/tests/ingest_modes.rs`

**Interfaces:**

- Produces on `RecorderStore`:
  `fn digest(&self, handle: &Self::Handle) -> Result<Sha256Digest, RecorderError>`; dry-run hashes the
  validated source descriptor, rechecks its metadata, and looks up its current digest. Extend
  `ports/recorder.rs`, `voice_memos/store.rs` and the existing `ProbeStore` forwarding implementation.

- Consumes: `Mode { dry_run, once }` from Task 19, `RecordingLedger::{seen_all, by_digest}`.

- Produces: `Ingest::would_ingest(&self, candidate: &Candidate, digest: &Sha256Digest,`
  `report: &mut IngestReport) -> Result<(), IngestFailure>` (private); the empty-store abort; the dry
  run's rule set: every gate runs, nothing is staged, recorded, refreshed or notified, a known
  recording's title comes from the ledger through `by_digest` and a ledger failure there fails the run.

- [ ] **Step 1: Write the failing tests**

`crates/vpt-adapters/tests/ingest_modes.rs`:

```rust
mod support;

use std::os::macos::fs::MetadataExt;
use support::{CAPTURED, Fixture, Parts, clock, digest_of};
use vpt_application::ports::{Archive, RecordingLedger, TitleOrigin};
use vpt_application::{IngestFailure, Mode};
use vpt_domain::fixtures::m4a;
use vpt_domain::identity::RecordingId;
use vpt_domain::notification::EventKind;
use vpt_domain::time::UtcInstant;

const DRY: Mode = Mode { dry_run: true, once: None };

fn copy_snapshot(fixture: &Fixture) -> Vec<(String, Vec<u8>, i64, i64, u32)> {
    let dir = fixture.state.join("title-copy");
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .expect("copy dir")
        .map(|entry| {
            let entry = entry.expect("entry");
            let metadata = entry.metadata().expect("metadata");
            let bytes = std::fs::read(entry.path()).expect("bytes");
            (entry.file_name().to_string_lossy().into_owned(), bytes, metadata.st_mtime(), metadata.st_mtime_nsec(), metadata.st_mode())
        })
        .collect();
    entries.sort();
    entries
}

#[test]
fn a_dry_run_runs_every_gate_and_writes_nothing_durable() {
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"a"));
    let whole = m4a(CAPTURED, 1, b"payload");
    fixture.add_recording("broken.m4a", &whole[..whole.len() - 12]);
    let orphan = m4a(CAPTURED, 2, b"orphan");
    let orphan_id = RecordingId::derive(UtcInstant { secs: CAPTURED }, clock().offset, &digest_of(&orphan)).expect("id");
    std::fs::write(fixture.audio.join(format!("{orphan_id}.m4a")), &orphan).expect("orphan");
    let mut parts = fixture.parts();
    parts.store = fixture.titled_store(false);

    let report = parts.ingest().run(&DRY).expect("dry run");

    assert_eq!(report.would_ingest.len(), 1);
    assert_eq!(report.would_ingest[0].path, fixture.recordings.join("a.m4a"));
    assert_eq!(report.would_ingest[0].title_source, TitleOrigin::Unavailable);
    assert_eq!(report.deferred.len(), 1);
    assert!(report.recovered.is_empty(), "a dry run recovers nothing");
    assert_eq!(parts.archive.archived().expect("archived").len(), 1, "the orphan is untouched");
    assert!(parts.archive.staged_leftovers().expect("leftovers").is_empty());
    assert!(parts.ledger.seen_all().expect("seen").is_empty(), "a dry run records no deferral");
    assert!(parts.ledger.recordings().expect("list").is_empty());
    assert!(parts.events().is_empty());
    assert!(!fixture.state.join("title-copy").exists());
}

#[test]
fn a_dry_run_does_not_apply_the_previous_recording_title_to_edited_bytes() {
    let fixture = Fixture::new();
    let source = fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"a"));
    fixture.add_title("a.m4a", "Alpha");
    let mut parts = fixture.parts();
    parts.store = fixture.titled_store(true);
    parts.ingest().run(&Mode::default()).expect("a real sweep records the title");
    fixture.add_title("b.m4a", "Beta, never copied");
    std::fs::write(&source, m4a(CAPTURED + 30, 1, b"edited")).expect("edit");
    fixture.set_mtime(&source, clock().now.secs - 60);
    let before = copy_snapshot(&fixture);
    let dry = Parts { store: fixture.titled_store(false), ..fixture.parts() };

    let report = dry.ingest().run(&DRY).expect("dry run");

    assert_eq!(report.would_ingest.len(), 1);
    assert_eq!(report.would_ingest[0].title, None);
    assert_eq!(report.would_ingest[0].title_source, TitleOrigin::Unavailable);
    assert_eq!(copy_snapshot(&fixture), before);
    assert_eq!(dry.ledger.recordings().expect("list").len(), 1);
    assert!(dry.events().is_empty());
}

#[test]
fn a_dry_run_on_an_unreadable_source_fails_without_an_event() {
    let fixture = Fixture::new();
    let parts = fixture.parts();
    std::fs::remove_dir_all(&fixture.recordings).expect("test setup only");

    let error = parts.ingest().run(&DRY).unwrap_err();

    assert!(matches!(error.failure, IngestFailure::StoreUnreadable(_)), "{error:?}");
    assert!(parts.events().is_empty());
}

#[test]
fn once_runs_the_gates_and_touches_nothing_else() {
    let fixture = Fixture::new();
    let whole = m4a(CAPTURED, 1, b"payload");
    let broken = fixture.add_recording("broken.m4a", &whole[..whole.len() - 12]);
    fixture.add_recording("other.m4a", &m4a(CAPTURED + 1, 1, b"other"));
    let parts = fixture.parts();

    let report = parts.ingest().run(&Mode { dry_run: false, once: Some(broken.clone()) }).expect("once");

    assert_eq!(report.deferred.len(), 1);
    assert!(report.ingested.is_empty());
    assert_eq!(parts.ledger.seen_all().expect("seen").len(), 1);
    let dry_once = parts.ingest().run(&Mode { dry_run: true, once: Some(fixture.recordings.join("other.m4a")) }).expect("dry once");
    assert_eq!(dry_once.would_ingest.len(), 1);
    assert!(parts.ledger.recordings().expect("list").is_empty());
}

#[test]
fn an_unreadable_recordings_directory_aborts_with_one_event() {
    let fixture = Fixture::new();
    let parts = fixture.parts();
    std::fs::remove_dir_all(&fixture.recordings).expect("test setup only");

    let error = parts.ingest().run(&Mode::default()).unwrap_err();

    assert!(matches!(error.failure, IngestFailure::StoreUnreadable(_)));
    assert_eq!(parts.events(), vec![EventKind::IngestFailed]);
}

#[test]
fn an_empty_store_that_held_recordings_before_is_the_silent_failure_and_aborts() {
    let fixture = Fixture::new();
    let a = fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"a"));
    let parts = fixture.parts();
    parts.ingest().run(&Mode::default()).expect("first");
    std::fs::remove_file(&a).expect("test setup only");

    let error = parts.ingest().run(&Mode::default()).unwrap_err();

    assert_eq!(error.failure, IngestFailure::EmptyStore);
    assert!(error.completed.is_empty());
    assert_eq!(parts.ledger.seen(&a).expect("seen").and_then(|row| row.source_gone_at), Some(clock().now));
    assert_eq!(parts.events(), vec![EventKind::IngestFailed]);
}

#[test]
fn a_fresh_empty_store_is_not_a_failure() {
    let fixture = Fixture::new();
    let parts = fixture.parts();
    assert!(parts.ingest().run(&Mode::default()).is_ok());
    assert!(parts.events().is_empty());
}

#[test]
fn a_failure_after_an_ingestion_lists_the_completed_identities() {
    let fixture = Fixture::new();
    let a = m4a(CAPTURED, 1, b"a");
    let b = m4a(CAPTURED + 5, 1, b"b");
    fixture.add_recording("a.m4a", &a);
    fixture.add_recording("b.m4a", &b);
    let b_id = RecordingId::derive(UtcInstant { secs: CAPTURED + 5 }, clock().offset, &digest_of(&b)).expect("id");
    std::fs::write(fixture.audio.join(format!("{b_id}.m4a")), b"tampered").expect("collision setup");
    let parts = fixture.parts();

    let error = parts.ingest().run(&Mode::default()).unwrap_err();

    assert!(matches!(error.failure, IngestFailure::ArchiveCollision { .. }), "{error:?}");
    let a_id = RecordingId::derive(UtcInstant { secs: CAPTURED }, clock().offset, &digest_of(&a)).expect("id");
    assert_eq!(error.completed, vec![a_id]);
}

#[test]
fn a_recovery_before_a_failing_candidate_stays_in_completed() {
    let fixture = Fixture::new();
    let orphan = m4a(CAPTURED, 2, b"orphan");
    let orphan_id = RecordingId::derive(UtcInstant { secs: CAPTURED }, clock().offset, &digest_of(&orphan)).expect("id");
    std::fs::write(fixture.audio.join(format!("{orphan_id}.m4a")), &orphan).expect("orphan");
    let b = m4a(CAPTURED + 5, 1, b"b");
    fixture.add_recording("b.m4a", &b);
    let b_id = RecordingId::derive(UtcInstant { secs: CAPTURED + 5 }, clock().offset, &digest_of(&b)).expect("id");
    std::fs::write(fixture.audio.join(format!("{b_id}.m4a")), b"tampered").expect("collision setup");
    let parts = fixture.parts();

    let error = parts.ingest().run(&Mode::default()).unwrap_err();

    assert!(matches!(error.failure, IngestFailure::ArchiveCollision { .. }), "{error:?}");
    assert_eq!(error.completed, vec![orphan_id.clone()]);
    assert!(parts.ledger.by_id(&orphan_id).expect("read").is_some());
}
#[test]
fn dry_run_finds_the_stored_title_for_known_bytes_at_a_new_path() {
    let fixture = Fixture::new();
    let source = fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"audio"));
    fixture.add_title("a.m4a", "Alpha");
    let mut parts = fixture.parts();
    parts.store = fixture.titled_store(true);
    parts.ingest().run(&Mode::default()).expect("first");
    let renamed = fixture.recordings.join("renamed.m4a");
    std::fs::rename(source, &renamed).expect("rename");
    let before = copy_snapshot(&fixture);
    let dry = Parts { store: fixture.titled_store(false), ..fixture.parts() };
    let report = dry.ingest().run(&DRY).expect("dry");
    assert_eq!(report.would_ingest.len(), 1);
    assert_eq!(report.would_ingest[0].path, renamed);
    assert_eq!(report.would_ingest[0].title.as_deref(), Some("Alpha"));
    assert_eq!(report.would_ingest[0].title_source, TitleOrigin::VoiceMemos);
    assert_eq!(copy_snapshot(&fixture), before);
    assert!(dry.events().is_empty());
}

#[test]
fn dry_run_defers_a_descriptor_that_changes_while_hashing() {
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"audio"));
    let parts = fixture.parts();
    let probe = support::fakes::ProbeStore::new(&parts.store);
    probe.grow_after_open.set(true);
    let report = vpt_application::Ingest {
        recorder: &probe, archive: &parts.archive, ledger: &parts.ledger, clock: &parts.clock,
        trash: &parts.trash, notifier: &parts.notifier, settings: &parts.settings,
    }.run(&DRY).expect("dry");
    assert!(report.would_ingest.is_empty());
    assert_eq!(report.deferred[0].reason, vpt_domain::sweep::DeferralReason::ChangedDuringRead);
    assert!(parts.ledger.seen_all().expect("seen").is_empty());
    assert!(parts.archive.staged_leftovers().expect("staging").is_empty());
    assert!(parts.events().is_empty());
}

```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-adapters --test ingest_modes`

Expected: the unreadable dry-run event test is an existing regression guard; the new once, empty-store
and current-digest dry-run cases establish this task's red result. Run every command independently so one
expected failure cannot skip the next command.

- [ ] **Step 3: Write the minimal implementation**

In `crates/vpt-application/src/ingest/mod.rs`, `sweep` becomes:

```rust
    fn sweep(&self, mode: &Mode, report: &mut IngestReport) -> Result<(), IngestFailure> {
        if !mode.dry_run {
            self.recorder.refresh_titles().map_err(candidate::recorder_failure)?;
        }
        if !mode.dry_run {
            self.recover_orphans(report)?;
        }
        let candidates = match &mode.once {
            Some(path) => vec![self.recorder.candidate(path).map_err(candidate::recorder_failure)?],
            None => self.recorder.candidates().map_err(candidate::recorder_failure)?,
        };
        let full_sweep = mode.once.is_none();
        if full_sweep && candidates.is_empty() && !self.ledger.seen_all().map_err(IngestFailure::Ledger)?.is_empty() {
            if !mode.dry_run {
                self.mark_gone(&candidates, report)?;
            }
            return Err(IngestFailure::EmptyStore);
        }
        for candidate in &candidates {
            self.process(candidate, mode, report)?;
        }
        if full_sweep && !mode.dry_run {
            self.mark_gone(&candidates, report)?;
        }
        Ok(())
    }
```

Add `use vpt_domain::digest::Sha256Digest;` to `ports/recorder.rs`, `voice_memos/store.rs`, and
`ingest/candidate.rs`. The Task 21 fake already imports it. Add this method to `RecorderStore`:

```rust
fn digest(&self, handle: &Self::Handle) -> Result<Sha256Digest, RecorderError>;
```

In `impl RecorderStore for VoiceMemosStore`:

```rust
fn digest(&self, handle: &File) -> Result<Sha256Digest, RecorderError> {
    let mut file = handle.try_clone().map_err(|error| RecorderError::Io(error.kind().to_string()))?;
    std::io::Seek::rewind(&mut file).map_err(|error| RecorderError::Io(error.kind().to_string()))?;
    crate::digest_open(&mut file).map_err(|error| RecorderError::Io(error.kind().to_string()))
}
```

In `impl RecorderStore for ProbeStore<'_>`:

```rust
fn digest(&self, handle: &File) -> Result<Sha256Digest, RecorderError> {
    self.inner.digest(handle)
}
```

In `crates/vpt-application/src/ingest/candidate.rs`, `process` gains the dry-run exit between the rest
gate and staging:

```rust
        if let Err(reason) = rest_gate(metadata.mtime, metadata.size, now, seen_facts.as_ref(), &limits) {
            return self.defer(candidate, seen, reason, now, mode, report);
        }
        if mode.dry_run {
            let digest = self.recorder.digest(&handle).map_err(recorder_failure)?;
            let after = self.recorder.metadata(&handle).map_err(recorder_failure)?;
            if after != metadata {
                return self.defer(candidate, seen, DeferralReason::ChangedDuringRead, now, mode, report);
            }
            return self.would_ingest(candidate, &digest, report);
        }
        self.stage_and_publish(candidate, &handle, &metadata, seen, now, mode, report)
```

and the `impl` block gains:

```rust
    /// Titles belong to the current content digest, regardless of its source path.
    pub(super) fn would_ingest(&self, candidate: &Candidate, digest: &Sha256Digest, report: &mut IngestReport) -> Result<(), IngestFailure> {
        let known = self.ledger.by_digest(digest).map_err(IngestFailure::Ledger)?;
        report.would_ingest.push(super::WouldIngest {
            path: candidate.path.clone(),
            title: known.as_ref().and_then(|record| record.title.clone()),
            title_source: known.map_or(TitleOrigin::Unavailable, |record| record.title_source),
        });
        Ok(())
    }
```

with `TitleOrigin` added to the `crate::ports` import of `candidate.rs`. `defer` and `unchanged` already
honour `mode.dry_run` (Tasks 19 and 20), `run` already withholds the failure event on a dry run (Task
19). The composition root (Task 27) attaches no title-copy capability during a dry run; titles come only
from the existing ledger by the current digest, leaving the private copy untouched.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo fmt --all`

Run: `cargo test -p vpt-adapters`

Expected: all PASS, every earlier ingest test included. Run
`cargo clippy --workspace --all-targets --features dev-tools -- -D warnings` and expect no warnings.

- [ ] **Step 5: Commit**

Run: `cargo fmt --all`

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(ingest): dry run, --once and the abort conditions"
```

______________________________________________________________________

### Task 24: Bounded process execution and the fake engine

Every command vpt spawns runs from its argv directly, in its own process group, with stdin, stdout and
stderr handled concurrently under one deadline; at the deadline or on interruption the group is
terminated, force-killed after the grace period and reaped. The private executor takes its cancellation,
clock, waiting and grace as controls: production hands it the process-wide interrupt flag, the monotonic
clock, a real sleep and the one-second grace; each test hands it its own, so no test touches the
production flag or asserts elapsed wall time. A readiness marker gates each virtual deadline or
interrupt. Group-cleanup tests use zero grace; the grace-specific test advances a private clock through
one second without sleeping. Every test child has a cleared environment and temporary HOME,
XDG_STATE_HOME and VPT_CONFIG. The 750 ms watchdog only bounds a broken fixture and fails the test if
used. The fake engine is the child every later integration test spawns.

**Files:**

- Create: `crates/vpt-adapters/src/spawn.rs`, `crates/vpt-adapters/src/spawn/tests.rs`,
  `crates/vpt-adapters/src/spawn/tests/support.rs`
- Modify: `crates/vpt-adapters/src/lib.rs`
- Create: `crates/vpt/src/bin/vpt-fake-engine.rs` (replacing the stand-in)

**Interfaces:**

- Consumes: nothing.

- Produces, through curated root reexports from the private adapters module `spawn`:
  `run(argv: &[String], stdin: &[u8], deadline: Duration, output_limit: usize) ->`
  `Result<Outcome, SpawnError>`,
  `run_with_env(argv: &[String], stdin: &[u8], deadline: Duration, output_limit: usize,`
  `env: &[(&str, &str)]) -> Result<Outcome, SpawnError>`,
  `Outcome { pub status: Status, pub stdout: Vec<u8>, pub stderr: Vec<u8>, pub group: libc::pid_t }`,
  `Status::{Exited(i32), Signaled(i32), DeadlineExceeded, Interrupted}`,
  `SpawnError::{NotFound(String), Io(String)}`, `install_interrupt_handlers()`,
  `OUTPUT_LIMIT: usize = 65_536`, `GRACE: Duration = 1 s`; privately,
  `Controls<I, K, W> { interrupted: I, now: K, wait: W, grace: Duration }` and
  `execute(command: Command, stdin, deadline, output_limit, controls)` and `interrupted() -> bool`. The
  direct child's status is provisional until the stdin writer and both drains finish. A deadline or
  interruption before that point overrides the recorded exit status. Termination reaps the leader while
  checking the whole group: only group disappearance ends grace early. Remaining members receive KILL
  after grace, and the direct child is reaped on every return path, including a wait error.

- The fake engine `vpt-fake-engine`: `--version` prints
  `{"schema":"vpt.helper/1","version":"<VPT_FAKE_VERSION or 1.0.0>"}`; `notify --title <t> --body <b>`
  appends `notify\t<t>\t<b>\n` to `$VPT_FAKE_LOG` when set and prints `{"posted":true}`; `trash <path>`
  renames the path into `$VPT_FAKE_TRASH` and prints `{"trashed":"<path>"}`; `command-sink [args...]`
  reads stdin to the end, appends `command-sink\t<args joined by space>\t<stdin>\n` to `$VPT_FAKE_LOG`
  and exits with `$VPT_FAKE_EXIT` (default 0); with `VPT_FAKE_HANG=1` every subcommand sleeps for 60
  seconds before answering.

- [ ] **Step 1: Write the failing tests**

`crates/vpt-adapters/src/lib.rs` gains these curated reexports; `spawn` remains private:

```rust
mod spawn;
pub use spawn::{
    GRACE, OUTPUT_LIMIT, Outcome, SpawnError, Status, install_interrupt_handlers, run, run_with_env,
};
```

Create `crates/vpt-adapters/src/spawn.rs` with this registration before running red:

```rust
#[cfg(test)]
mod tests;
```

`crates/vpt-adapters/src/spawn/tests.rs`:

```rust
mod support;
use super::*;
use std::cell::Cell;
use support::{OwnedGroup, command, controls, group_is_gone, pending, wait_for_leader};

#[test]
fn the_exit_code_and_both_streams_are_returned() {
    let temp = tempfile::tempdir().expect("temp");
    let child = command(
        temp.path(),
        "/bin/sh",
        &["-c", "printf out; printf err >&2; exit 3"],
    );
    let outcome = execute(
        child,
        b"",
        Duration::from_millis(750),
        OUTPUT_LIMIT,
        controls(),
    )
    .expect("ran");
    assert_eq!(outcome.status, Status::Exited(3));
    assert_eq!(outcome.stdout, b"out");
    assert_eq!(outcome.stderr, b"err");
    assert!(group_is_gone(outcome.group));
}

#[test]
fn stdin_is_written_and_a_missing_executable_is_not_found() {
    let temp = tempfile::tempdir().expect("temp");
    let child = command(temp.path(), "/bin/cat", &[]);
    let outcome = execute(
        child,
        b"hello",
        Duration::from_millis(750),
        OUTPUT_LIMIT,
        controls(),
    )
    .expect("ran");
    assert_eq!(outcome.stdout, b"hello");
    let missing = temp.path().join("missing-program");
    let child = command(temp.path(), missing.to_str().expect("path"), &[]);
    assert!(matches!(
        execute(child, b"", Duration::ZERO, OUTPUT_LIMIT, controls()),
        Err(SpawnError::NotFound(_))
    ));
}

#[test]
fn output_is_bounded_while_the_child_is_still_drained_to_completion() {
    let temp = tempfile::tempdir().expect("temp");
    let child = command(
        temp.path(),
        "/bin/sh",
        &["-c", "/usr/bin/yes | /usr/bin/head -c 200000"],
    );
    let outcome = execute(child, b"", Duration::from_millis(750), 1_000, controls()).expect("ran");
    assert_eq!(outcome.stdout.len(), 1_000);
    assert_eq!(outcome.status, Status::Exited(0));
}

#[test]
fn the_deadline_terminates_the_whole_process_group_and_reaps_it() {
    assert_eq!(pending(false, false).status, Status::DeadlineExceeded);
}

#[test]
fn an_interrupt_terminates_the_child_and_reports_interrupted() {
    assert_eq!(pending(false, true).status, Status::Interrupted);
}

#[test]
fn a_descendant_holding_stdout_does_not_outlive_the_deadline() {
    let outcome = pending(true, false);
    assert_eq!(outcome.status, Status::DeadlineExceeded);
    assert_eq!(outcome.stdout, b"{}");
}

#[test]
fn interruption_overrides_an_exited_leader_while_a_descendant_holds_stdout() {
    assert_eq!(pending(true, true).status, Status::Interrupted);
}

#[test]
fn a_descendant_that_closed_every_stream_lets_the_run_finish_with_the_child() {
    let temp = tempfile::tempdir().expect("temp");
    let child = command(
        temp.path(),
        "/bin/sh",
        &[
            "-c",
            "(exec >/dev/null 2>&1 </dev/null; exec /bin/sleep 30) & printf done",
        ],
    );
    let outcome = execute(
        child,
        b"",
        Duration::from_millis(750),
        OUTPUT_LIMIT,
        controls(),
    )
    .expect("ran");
    assert_eq!(outcome.status, Status::Exited(0));
    assert_eq!(outcome.stdout, b"done");
    assert!(group_is_gone(outcome.group));
}

#[test]
fn a_surviving_descendant_receives_the_full_grace_after_the_leader_exits() {
    let temp = tempfile::tempdir().expect("temp");
    let mut builder = command(
        temp.path(),
        "/bin/sh",
        &["-c", "trap '' TERM; /bin/sleep 30 &"],
    );
    let mut child = OwnedGroup(
        builder
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
            .expect("fixture group"),
    );
    wait_for_leader(&mut child.0);
    let group = child.0.id() as libc::pid_t;
    let start = Instant::now();
    let elapsed = Cell::new(Duration::ZERO);
    let controls = Controls {
        interrupted: || false,
        now: || start + elapsed.get(),
        wait: |duration| {
            elapsed.set(elapsed.get() + duration);
            std::thread::yield_now();
        },
        grace: GRACE,
    };
    terminate(group, &mut child.0, &controls);
    assert_eq!(elapsed.get(), GRACE);
    assert!(group_is_gone(group));
}

#[test]
fn test_children_receive_only_the_fixture_environment() {
    let temp = tempfile::tempdir().expect("temp");
    let child = command(temp.path(), "/usr/bin/env", &[]);
    let outcome = execute(
        child,
        b"",
        Duration::from_millis(750),
        OUTPUT_LIMIT,
        controls(),
    )
    .expect("ran");
    let text = String::from_utf8(outcome.stdout).expect("environment");
    let mut names: Vec<_> = text
        .lines()
        .map(|line| line.split_once('=').expect("entry").0)
        .collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "HOME",
            "PATH",
            "VPT_CONFIG",
            "XDG_CONFIG_HOME",
            "XDG_DATA_HOME",
            "XDG_STATE_HOME"
        ]
    );
    assert!(text.contains(&format!("HOME={}\n", temp.path().display())));
}
```

`crates/vpt-adapters/src/spawn/tests/support.rs`:

```rust
use super::super::*;
use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::sync::{Arc, atomic::AtomicBool, mpsc};

const WATCHDOG: Duration = Duration::from_millis(750);

pub(super) fn command(temp: &Path, program: &str, arguments: &[&str]) -> Command {
    let mut command = Command::new(program);
    command
        .args(arguments)
        .env_clear()
        .current_dir(temp)
        .env("HOME", temp)
        .env("XDG_CONFIG_HOME", temp.join("config"))
        .env("XDG_DATA_HOME", temp.join("data"))
        .env("XDG_STATE_HOME", temp.join("state"))
        .env("VPT_CONFIG", temp.join("config.toml"))
        .env("PATH", "/usr/bin:/bin");
    command
}

pub(super) fn controls() -> Controls<impl Fn() -> bool, impl Fn() -> Instant, impl Fn(Duration)> {
    Controls {
        interrupted: || false,
        now: Instant::now,
        wait: |_| std::thread::yield_now(),
        grace: Duration::ZERO,
    }
}

pub(super) fn group_is_gone(group: libc::pid_t) -> bool {
    let until = Instant::now() + WATCHDOG;
    loop {
        // SAFETY: signal zero checks existence without delivering a signal.
        if unsafe { libc::killpg(group, 0) } == -1
            && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
        {
            return true;
        }
        if Instant::now() >= until {
            return false;
        }
        std::thread::yield_now();
    }
}

pub(super) struct Watchdog {
    stop: Option<mpsc::Sender<()>>,
    worker: Option<std::thread::JoinHandle<()>>,
    pub fired: Arc<AtomicBool>,
}

impl Watchdog {
    pub fn start(group_file: PathBuf) -> Self {
        let (stop, receiver) = mpsc::channel();
        let fired = Arc::new(AtomicBool::new(false));
        let flag = fired.clone();
        let worker = std::thread::spawn(move || {
            if receiver.recv_timeout(WATCHDOG) == Err(mpsc::RecvTimeoutError::Timeout) {
                flag.store(true, Ordering::SeqCst);
                if let Some(group) = std::fs::read_to_string(group_file)
                    .ok()
                    .and_then(|text| text.parse::<libc::pid_t>().ok())
                    .filter(|group| *group > 1)
                {
                    // SAFETY: the fixture writes its own newly created process group.
                    unsafe {
                        libc::killpg(group, libc::SIGKILL);
                    }
                }
            }
        });
        Self {
            stop: Some(stop),
            worker: Some(worker),
            fired,
        }
    }
}

impl Drop for Watchdog {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(worker) = self.worker.take() {
            worker.join().expect("watchdog");
        }
    }
}

pub(super) fn pending(leader_exits: bool, interrupt: bool) -> Outcome {
    let temp = tempfile::tempdir().expect("temp");
    let ready = temp.path().join("ready");
    let group_file = temp.path().join("group");
    let script = if leader_exits {
        "printf '%s' \"$$\" > \"$VPT_TEST_GROUP\"; parent=$$; \
        (while kill -0 \"$parent\" 2>/dev/null; do :; done; \
        printf ready > \"$VPT_TEST_READY\"; exec /bin/sleep 30) & printf '{}'; exit 0"
    } else {
        "printf '%s' \"$$\" > \"$VPT_TEST_GROUP\"; /bin/sleep 30 & \
        printf ready > \"$VPT_TEST_READY\"; wait"
    };
    let mut child = command(temp.path(), "/bin/sh", &["-c", script]);
    child
        .env("VPT_TEST_READY", &ready)
        .env("VPT_TEST_GROUP", &group_file);
    let start = Instant::now();
    let elapsed = Cell::new(Duration::ZERO);
    let cancelled = Cell::new(false);
    let watchdog = Watchdog::start(group_file);
    let controls = Controls {
        interrupted: || cancelled.get(),
        now: || start + elapsed.get(),
        wait: |_| {
            if ready.exists() {
                if interrupt {
                    cancelled.set(true);
                } else {
                    elapsed.set(Duration::from_secs(1));
                }
            }
            if watchdog.fired.load(Ordering::SeqCst) {
                elapsed.set(Duration::from_secs(1));
            }
            std::thread::yield_now();
        },
        grace: Duration::ZERO,
    };
    let outcome = execute(child, b"", Duration::from_secs(1), OUTPUT_LIMIT, controls).expect("ran");
    assert!(
        !watchdog.fired.load(Ordering::SeqCst),
        "process fixture exceeded its watchdog: group={} ready={} status={:?}",
        outcome.group,
        ready.exists(),
        outcome.status
    );
    assert!(ready.exists(), "fixture readiness was not observed");
    assert!(
        group_is_gone(outcome.group),
        "fixture group survived cleanup"
    );
    outcome
}

pub(super) struct OwnedGroup(pub Child);

impl Drop for OwnedGroup {
    fn drop(&mut self) {
        let group = self.0.id() as libc::pid_t;
        // SAFETY: this guard owns the fixture's process group.
        unsafe {
            libc::killpg(group, libc::SIGKILL);
        }
        let _ = self.0.wait();
    }
}

pub(super) fn wait_for_leader(child: &mut Child) {
    let until = Instant::now() + WATCHDOG;
    loop {
        if child.try_wait().expect("leader status").is_some() {
            return;
        }
        assert!(Instant::now() < until, "fixture leader did not exit");
        std::thread::yield_now();
    }
}
```

The shell fixtures are confined to the tests. Descendants use `exec` after publishing readiness, so
termination cannot race another fork of a pipe-holding child. The group-existence assertion polls to
allow asynchronous reaping; it never equates any arbitrary signal error with group disappearance.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-adapters spawn`

Expected: the build fails with `cannot find` for `execute`, `Controls`, `Outcome`, `Status`,
`SpawnError`, `OUTPUT_LIMIT` and the imported process types. Confirm the private child test module is
registered before accepting this compile failure.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-adapters/src/spawn.rs`, above its test module:

```rust
//! Bounded process execution: argv, own process group, one deadline for
//! writing, draining and waiting, group termination, reaping.

use std::io::{Read, Write};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub const OUTPUT_LIMIT: usize = 65_536;
pub const GRACE: Duration = Duration::from_secs(1);
const TICK: Duration = Duration::from_millis(20);

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, PartialEq, Eq)]
pub enum Status {
    Exited(i32),
    Signaled(i32),
    DeadlineExceeded,
    Interrupted,
}

#[derive(Debug)]
pub struct Outcome {
    pub status: Status,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub group: libc::pid_t,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SpawnError {
    NotFound(String),
    Io(String),
}

/// What the executor consults: cancellation, the clock, how it waits, and
/// how long a terminated group gets before it is killed.
struct Controls<I, K, W> {
    interrupted: I,
    now: K,
    wait: W,
    grace: Duration,
}

extern "C" fn on_signal(_signal: libc::c_int) {
    INTERRUPTED.store(true, Ordering::SeqCst);
}

/// Called once at startup by the command crate.
pub fn install_interrupt_handlers() {
    for signal in [libc::SIGINT, libc::SIGTERM, libc::SIGHUP] {
        // SAFETY: the handler only stores into an atomic, which is async-signal-safe.
        unsafe {
            libc::signal(
                signal,
                on_signal as extern "C" fn(libc::c_int) as libc::sighandler_t,
            );
        }
    }
}

fn interrupted() -> bool {
    INTERRUPTED.load(Ordering::SeqCst)
}

pub fn run(
    argv: &[String],
    stdin: &[u8],
    deadline: Duration,
    output_limit: usize,
) -> Result<Outcome, SpawnError> {
    run_with_env(argv, stdin, deadline, output_limit, &[])
}

/// `run` with extra environment for the child; tests use it to steer the fake engine.
pub fn run_with_env(
    argv: &[String],
    stdin: &[u8],
    deadline: Duration,
    output_limit: usize,
    env: &[(&str, &str)],
) -> Result<Outcome, SpawnError> {
    let controls = Controls {
        interrupted,
        now: Instant::now,
        wait: std::thread::sleep,
        grace: GRACE,
    };
    let (program, arguments) = argv
        .split_first()
        .ok_or_else(|| SpawnError::Io("empty argv".into()))?;
    let mut command = Command::new(program);
    command.args(arguments).envs(env.iter().copied());
    execute(command, stdin, deadline, output_limit, controls)
}

fn execute<I, K, W>(
    mut command: Command,
    stdin: &[u8],
    deadline: Duration,
    output_limit: usize,
    controls: Controls<I, K, W>,
) -> Result<Outcome, SpawnError>
where
    I: Fn() -> bool,
    K: Fn() -> Instant,
    W: Fn(Duration),
{
    let program = command.get_program().to_string_lossy().into_owned();
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => SpawnError::NotFound(program),
            _ => SpawnError::Io(error.kind().to_string()),
        })?;
    let group = child.id() as libc::pid_t;
    let input = stdin.to_vec();
    let mut stdin_pipe = child.stdin.take();
    let writer = std::thread::spawn(move || {
        if let Some(mut pipe) = stdin_pipe.take() {
            let _ = pipe.write_all(&input);
        }
    });
    let stdout_pipe = child.stdout.take();
    let stderr_pipe = child.stderr.take();
    let out = std::thread::spawn(move || drain(stdout_pipe, output_limit));
    let err = std::thread::spawn(move || drain(stderr_pipe, output_limit));
    let started = (controls.now)();
    let mut status = None;
    let result = loop {
        if status.is_none() {
            match child.try_wait() {
                Ok(Some(exit)) => status = Some(status_of(exit)),
                Ok(None) => {}
                Err(error) => break Err(SpawnError::Io(error.kind().to_string())),
            }
        }
        let drained = writer.is_finished() && out.is_finished() && err.is_finished();
        if let (Some(_), true) = (&status, drained) {
            break Ok(());
        }
        if (controls.interrupted)() {
            status = Some(Status::Interrupted);
            break Ok(());
        }
        if (controls.now)().duration_since(started) >= deadline {
            status = Some(Status::DeadlineExceeded);
            break Ok(());
        }
        (controls.wait)(TICK);
    };
    terminate(group, &mut child, &controls);
    let _ = writer.join();
    let stdout = out.join().unwrap_or_default();
    let stderr = err.join().unwrap_or_default();
    result.map(|()| Outcome {
        status: status.unwrap_or(Status::Exited(-1)),
        stdout,
        stderr,
        group,
    })
}

fn status_of(exit: std::process::ExitStatus) -> Status {
    match (exit.code(), exit.signal()) {
        (Some(code), _) => Status::Exited(code),
        (None, Some(signal)) => Status::Signaled(signal),
        (None, None) => Status::Exited(-1),
    }
}

fn drain(pipe: Option<impl Read>, limit: usize) -> Vec<u8> {
    let Some(mut pipe) = pipe else {
        return Vec::new();
    };
    let mut kept = Vec::new();
    let mut buffer = [0u8; 8_192];
    while let Ok(read) = pipe.read(&mut buffer) {
        if read == 0 {
            break;
        }
        let room = limit.saturating_sub(kept.len());
        kept.extend_from_slice(&buffer[..read.min(room)]);
    }
    kept
}

/// TERM to whatever is left of the group, the grace period, KILL, then reap
/// the direct child. Harmless on a group that has already gone.
fn terminate<I, K, W>(group: libc::pid_t, child: &mut Child, controls: &Controls<I, K, W>)
where
    K: Fn() -> Instant,
    W: Fn(Duration),
{
    // SAFETY: `group` is the child's own process group, created by process_group(0).
    unsafe {
        libc::killpg(group, libc::SIGTERM);
    }
    let started = (controls.now)();
    while (controls.now)().duration_since(started) < controls.grace {
        let _ = child.try_wait();
        // SAFETY: signal zero checks the owned process group's existence.
        if unsafe { libc::killpg(group, 0) } == -1
            && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
        {
            break;
        }
        (controls.wait)(TICK);
    }
    // SAFETY: as above; a group that already exited makes killpg fail harmlessly.
    unsafe {
        libc::killpg(group, libc::SIGKILL);
    }
    let _ = child.wait();
}

```

The recorded exit status remains provisional while any pipe worker is unfinished. Interruption and the
deadline override that status; a complete child result keeps its own exit or signal. Every exit cleans
the owned process group. Reaping the leader does not end a surviving descendant's grace.

`crates/vpt/src/bin/vpt-fake-engine.rs`:

```rust
//! The fake helper and fake command the tests spawn. Behind `dev-tools`.

use std::io::{Read, Write};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if std::env::var("VPT_FAKE_HANG").as_deref() == Ok("1") {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
    let code = match args.first().map(String::as_str) {
        Some("--version") => version(),
        Some("notify") => notify(&args[1..]),
        Some("trash") => trash(args.get(1).map(String::as_str)),
        Some("command-sink") => command_sink(&args[1..]),
        _ => {
            eprintln!("vpt-fake-engine: unknown subcommand");
            2
        }
    };
    std::process::exit(code);
}

fn log(line: &str) {
    if let Ok(path) = std::env::var("VPT_FAKE_LOG")
        && let Ok(mut file) = std::fs::OpenOptions::new().append(true).create(true).open(path) {
        let _ = writeln!(file, "{line}");
    }
}

fn version() -> i32 {
    let version = std::env::var("VPT_FAKE_VERSION").unwrap_or_else(|_| "1.0.0".into());
    println!("{{\"schema\":\"vpt.helper/1\",\"version\":\"{version}\"}}");
    0
}

fn notify(args: &[String]) -> i32 {
    let value = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned().unwrap_or_default();
    log(&format!("notify\t{}\t{}", value("--title"), value("--body")));
    println!("{{\"posted\":true}}");
    0
}

fn trash(path: Option<&str>) -> i32 {
    let Some(path) = path else { return 2 };
    let Ok(dir) = std::env::var("VPT_FAKE_TRASH") else {
        eprintln!("VPT_FAKE_TRASH unset");
        return 1;
    };
    let name = std::path::Path::new(path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    if std::fs::rename(path, std::path::Path::new(&dir).join(name)).is_err() {
        return 1;
    }
    log(&format!("trash\t{path}"));
    println!("{{\"trashed\":{}}}", serde_json::Value::String(path.to_owned()));
    0
}

fn command_sink(args: &[String]) -> i32 {
    let mut stdin = String::new();
    let _ = std::io::stdin().read_to_string(&mut stdin);
    log(&format!("command-sink\t{}\t{}", args.join(" "), stdin.replace('\n', "\\n")));
    std::env::var("VPT_FAKE_EXIT").ok().and_then(|v| v.parse().ok()).unwrap_or(0)
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo fmt --all`

Run: `cargo test -p vpt-adapters spawn && cargo build -p vpt --features dev-tools`

Expected: 10 tests PASS; the fake engine builds. Run `cargo fmt --all`, then `cargo fmt --all -- --check`
and `cargo clippy -p vpt-adapters --all-targets -- -D warnings`.

Run `cargo +nightly test -p vpt-adapters spawn -- -Z unstable-options --report-time` and record each test
below one second. Restore each of these mutants after its named test fails: retain `Exited(0)` on
deadline, retain it on interruption, break grace when only the leader exits, or signal only the child pid
instead of its process group. Rerun the unmutated stable suite after all four.

- [ ] **Step 5: Commit**

Run: `cargo fmt --all`

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(adapters): bounded process execution and the fake engine"
```

______________________________________________________________________

### Task 25: The helper client and the bounded document reader

**Files:**

- Create: `crates/vpt-protocol/src/limits.rs`, `crates/vpt-protocol/src/limits/visitor.rs`,
  `crates/vpt-protocol/src/limits/tests.rs`, `crates/vpt-protocol/src/helper.rs`.
- Create: `crates/vpt-adapters/src/helper.rs`, `crates/vpt-adapters/src/helper/reply.rs`,
  `crates/vpt-adapters/src/helper/tests.rs`.
- Modify: `crates/vpt-protocol/src/lib.rs`, `crates/vpt-adapters/src/lib.rs`,
  `crates/vpt-adapters/Cargo.toml`, `crates/vpt/src/lib.rs`, `crates/vpt/src/commands/version.rs`.
- Test: `crates/vpt/tests/helper_client.rs`, `crates/vpt/tests/version.rs`,
  `crates/vpt/tests/support/mod.rs`.

**Interfaces:**

- Consumes:
  `spawn::run_with_env(argv: &[String], stdin: &[u8], deadline: Duration, output_limit: usize, env: &[(&str, &str)]) -> Result<spawn::Outcome, spawn::SpawnError>`;
  `Outcome { status: Status, stdout: Vec<u8>, stderr: Vec<u8>, group: libc::pid_t }`;
  `Status::{Exited(i32), Signaled(i32), DeadlineExceeded, Interrupted}`;
  `SpawnError::{NotFound(String), Io(String)}`;
  `Trash::trash(&self, path: &Path) -> Result<PathBuf, TrashError>` and
  `Trash::take_diagnostics(&self) -> Vec<String>`;
  `TrashError::{HelperAbsent, HelperVersion { found: u32 }, Failed(String), Unknown(String)}`.

- Consumes: `config::load_file(path: &Path) -> Result<toml::Table, ConfigError>`,
  `config::from_table(table: &toml::Table, home: &Path) -> Result<Settings, ConfigError>`;
  `Settings::helper_path: PathBuf`;
  `Environment::{config_path(&self) -> PathBuf, home_dir(&self) -> PathBuf}`;
  `Invocation::config: Option<PathBuf>`.

- Produces protocol root exports:
  `Limits { pub bytes: usize, pub depth: usize, pub text_chars: usize, pub array_len: usize }`,
  `Limits::incoming() -> Limits`,
  `LimitViolation::{Bytes(usize), Depth(usize), Text(usize), Array(usize), Syntax(String)}`,
  `read_bounded(bytes: &[u8], limits: &Limits) -> Result<serde_json::Value, LimitViolation>`. Check depth
  before descending; refuse array entry `array_len + 1` before decoding or appending it; validate decoded
  strings and keys before retaining them; finish with `Deserializer::end()`.

- Produces protocol root exports: `HELPER_SCHEMA: &str = "vpt.helper/1"`,
  `HelperVersion { pub schema: String, pub version: String }`,
  `HelperVersion::major(&self) -> Option<u32>`, `Posted { pub posted: bool }`,
  `Trashed { pub trashed: String }`, each reply implementing `Deserialize`.

- Produces adapter root exports: `HelperClient::new(path: PathBuf) -> HelperClient`,
  `HelperClient::with_deadline(self, deadline: Duration) -> HelperClient`,
  `HelperClient::version(&self) -> Result<String, HelperError>`,
  `HelperClient::version_with_env(&self, env: &[(&str, &str)]) -> Result<String, HelperError>`,
  `HelperClient::notify(&self, title: &str, body: &str) -> Result<(), HelperError>`,
  `HelperClient::notify_with_env(&self, title: &str, body: &str, env: &[(&str, &str)]) -> Result<(), HelperError>`,
  `HelperClient::trash_with_env(&self, path: &Path, env: &[(&str, &str)]) -> Result<PathBuf, TrashError>`,
  `HelperClient::take_diagnostics(&self) -> Vec<String>`, `impl Trash for HelperClient`,
  `CALL_DEADLINE: Duration = Duration::from_secs(5)`, `BUILT_AGAINST_MAJOR: u32 = 1`,
  `HelperError::{Absent, MajorMismatch { found: u32 }, Failed(String), Unknown(String)}`. The first
  notify or Trash request validates the configured helper. Only successful compatibility is cached.
  Unknown additive field names enter the drainable diagnostics queue; values never do. A caller keeps the
  client and test environment fixed for one command.

- Produces
  `commands::version::{run(environment: &Environment, config: Option<&Path>) -> Outcome, document(helper_version: Option<&str>) -> serde_json::Value, human(helper_version: Option<&str>) -> String}`.
  Version uses the selected configuration, including `--config` and `VPT_CONFIG`; only an absent
  configuration permits the default `vpt-macos`. An unreadable or invalid existing configuration, or an
  unidentified selected helper, produces `helper_version: null` with exit 0.

- Test support: `FAKE_ENGINE: &str`, `Sandbox::install_fake_helper(&self)`,
  `Sandbox::fake_log(&self) -> PathBuf`. Every integration call selects the fake or a missing sandbox
  path.

- [ ] **Step 1: Register and write the failing tests**

Add `serde = "1.0.229"` under `[dependencies]` in `crates/vpt-adapters/Cargo.toml`. The helper decoder
names serde's deserialization trait directly.

Add these private modules and curated exports to `crates/vpt-protocol/src/lib.rs`:

```rust
mod limits;
mod helper;
pub use limits::{LimitViolation, Limits, read_bounded};
pub use helper::{HELPER_SCHEMA, HelperVersion, Posted, Trashed};
```

Add to `crates/vpt-adapters/src/lib.rs`:

```rust
mod helper;
pub use helper::{BUILT_AGAINST_MAJOR, CALL_DEADLINE, HelperClient, HelperError};
```

Create `crates/vpt-protocol/src/limits.rs` and `crates/vpt-adapters/src/helper.rs` with this test
declaration in each:

```rust
#[cfg(test)]
mod tests;
```

Create `crates/vpt-protocol/src/helper.rs` as an empty file. The unresolved reply exports participate in
the red build.

`crates/vpt-protocol/src/limits/tests.rs`:

```rust
use super::*;

#[test]
fn valid_document_and_unicode_text_at_the_limit_parse() {
    assert_eq!(
        read_bounded(br#"{"version":"1.0.0"}"#, &Limits::incoming()).expect("document")["version"],
        "1.0.0"
    );
    let limits = Limits {
        text_chars: 2,
        ..Limits::incoming()
    };
    assert!(read_bounded(br#"["\u00e9\u00e9"]"#, &limits).is_ok());
}

#[test]
fn byte_limit_includes_the_sentinel_and_trailing_whitespace() {
    let limits = Limits {
        bytes: 2,
        ..Limits::incoming()
    };
    assert!(read_bounded(b"{}", &limits).is_ok());
    assert_eq!(read_bounded(b"{} ", &limits), Err(LimitViolation::Bytes(3)));
}

#[test]
fn depth_is_checked_before_reading_children() {
    let limits = Limits {
        depth: 1,
        ..Limits::incoming()
    };
    assert!(read_bounded(b"[1]", &limits).is_ok());
    assert_eq!(
        read_bounded(b"[[broken", &limits),
        Err(LimitViolation::Depth(2))
    );
    assert!(read_bounded(br#"{"t":"[[[[[[[[[[[["}"#, &limits).is_ok());
}

#[test]
fn decoded_text_and_keys_are_checked_before_retaining_them() {
    let limits = Limits {
        text_chars: 2,
        ..Limits::incoming()
    };
    assert_eq!(
        read_bounded(br#"["\u00e9\u00e9\u00e9"]"#, &limits),
        Err(LimitViolation::Text(3))
    );
    assert_eq!(
        read_bounded(br#"{"abc":null}"#, &limits),
        Err(LimitViolation::Text(3))
    );
}

#[test]
fn array_overflow_is_refused_before_decoding_the_extra_element() {
    let limits = Limits {
        array_len: 2,
        ..Limits::incoming()
    };
    assert!(read_bounded(b"[1,2]", &limits).is_ok());
    assert_eq!(
        read_bounded(b"[1,2,{broken", &limits),
        Err(LimitViolation::Array(3))
    );
    let zero = Limits {
        array_len: 0,
        ..Limits::incoming()
    };
    assert!(read_bounded(b"[]", &zero).is_ok());
    assert_eq!(
        read_bounded(b"[true]", &zero),
        Err(LimitViolation::Array(1))
    );
}

#[test]
fn malformed_and_trailing_documents_have_fixed_diagnostics() {
    for bytes in [b"{".as_slice(), b"{} {}", b"CANARY-RAW-JSON"] {
        assert_eq!(
            read_bounded(bytes, &Limits::incoming()),
            Err(LimitViolation::Syntax("invalid JSON document".into()))
        );
    }
}
```

`crates/vpt-adapters/src/helper/tests.rs`:

```rust
use super::*;
use serde_json::json;

fn client() -> HelperClient {
    HelperClient::new(PathBuf::from("/sandbox/fake-helper"))
}

fn version() -> Value {
    json!({"schema":"vpt.helper/1", "version":"1.0.0"})
}

#[test]
fn compatibility_is_checked_once_before_notify_and_trash() {
    let client = client();
    let calls = RefCell::new(Vec::new());
    let call = |argv: &[String]| {
        calls.borrow_mut().push(argv[0].clone());
        Ok(match argv[0].as_str() {
            "--version" => version(),
            "notify" => json!({"posted":true}),
            "trash" => json!({"trashed":argv[1]}),
            _ => panic!("unexpected operation"),
        })
    };
    assert_eq!(client.notify_using("title", "body", &call), Ok(()));
    let path = Path::new("/sandbox/file");
    assert_eq!(client.trash_using(path, &call), Ok(path.into()));
    assert_eq!(*calls.borrow(), ["--version", "notify", "trash"]);
}

#[test]
fn incompatible_version_never_receives_notify_or_trash() {
    let client = client();
    let calls = RefCell::new(Vec::new());
    let call = |argv: &[String]| {
        calls.borrow_mut().push(argv[0].clone());
        Ok(json!({"schema":"vpt.helper/1", "version":"2.3.0"}))
    };
    assert_eq!(
        client.notify_using("title", "body", &call),
        Err(HelperError::MajorMismatch { found: 2 })
    );
    assert_eq!(
        client.trash_using(Path::new("/sandbox/file"), &call),
        Err(TrashError::HelperVersion { found: 2 })
    );
    assert_eq!(*calls.borrow(), ["--version", "--version"]);
}

#[test]
fn trash_confirms_the_exact_requested_path() {
    let client = client();
    let call = |argv: &[String]| {
        Ok(if argv[0] == "--version" {
            version()
        } else {
            json!({"trashed":"/sandbox/another"})
        })
    };
    assert_eq!(
        client.trash_using(Path::new("/sandbox/file"), &call),
        Err(TrashError::Unknown(
            "the helper trash reply names another path".into()
        ))
    );
}

#[test]
fn additive_names_survive_all_replies_without_their_values() {
    let client = client();
    let call = |argv: &[String]| {
        let mut value = match argv[0].as_str() {
            "--version" => version(),
            "notify" => json!({"posted":true}),
            _ => json!({"trashed":argv[1]}),
        };
        value["capabilities"] = json!("CANARY-ADDITIVE-VALUE");
        Ok(value)
    };
    client.notify_using("title", "body", &call).expect("notify");
    client
        .trash_using(Path::new("/sandbox/file"), &call)
        .expect("trash");
    assert_eq!(
        client.take_diagnostics(),
        vec!["unknown additive helper field: capabilities"; 3]
    );
    assert!(client.take_diagnostics().is_empty());
}

#[test]
fn wrong_types_and_schemas_never_quote_child_values() {
    let client = client();
    for schema in [json!("CANARY-SCHEMA"), json!({"private":"CANARY-SCHEMA"})] {
        let value = json!({"schema":schema, "version":"1.0.0"});
        assert_eq!(
            client.version_using(&|_| Ok(value.clone())),
            Err(HelperError::Unknown("invalid helper schema".into()))
        );
    }
    assert_eq!(
        client.version_using(&|_| Ok(json!({"schema":"vpt.helper/9", "version":"CANARY"}))),
        Err(HelperError::Unknown(
            "unsupported helper schema major 9".into()
        ))
    );
    for value in [json!("1.0.0+CANARY\nSECRET"), json!({"private":"CANARY"})] {
        assert_eq!(
            client.version_using(&|_| Ok(json!({"schema":"vpt.helper/1", "version":value}))),
            Err(HelperError::Unknown(
                "invalid helper version document".into()
            ))
        );
    }
    let notify = |argv: &[String]| {
        Ok(if argv[0] == "--version" {
            version()
        } else {
            json!({"posted":"CANARY-NOTIFY"})
        })
    };
    assert_eq!(
        client.notify_using("title", "body", &notify),
        Err(HelperError::Unknown(
            "invalid helper notify document".into()
        ))
    );
    assert_eq!(
        client.trash_using(Path::new("/sandbox/file"), &|_| Ok(
            json!({"trashed":["CANARY-TRASH"]})
        )),
        Err(TrashError::Unknown("invalid helper trash document".into()))
    );
}

#[test]
fn oversized_valid_prefix_retains_the_rejection_sentinel() {
    let client = client();
    let result = client.call_using(&["--version".into()], &[], |_, _, _, limit, _| {
        assert_eq!(limit, 65_537);
        let mut stdout = serde_json::to_vec(&version()).expect("json");
        stdout.resize(65_540, b' ');
        stdout.truncate(limit);
        Ok(spawn::Outcome {
            status: Status::Exited(0),
            stdout,
            stderr: vec![],
            group: 0,
        })
    });
    assert_eq!(
        result,
        Err(HelperError::Unknown(
            "invalid or oversized helper document".into()
        ))
    );
}

#[test]
fn spawn_and_timeout_errors_are_fixed_and_absence_is_typed() {
    let client = client();
    assert_eq!(
        client.call_using(&[], &[], |_, _, _, _, _| Err(SpawnError::Io(
            "CANARY".into()
        ))),
        Err(HelperError::Failed("the helper could not start".into()))
    );
    assert_eq!(
        client.call_using(&[], &[], |_, _, _, _, _| Err(SpawnError::NotFound(
            "CANARY".into()
        ))),
        Err(HelperError::Absent)
    );
    assert_eq!(
        client.call_using(&[], &[], |_, _, _, _, _| Ok(spawn::Outcome {
            status: Status::DeadlineExceeded,
            stdout: b"CANARY".to_vec(),
            stderr: vec![],
            group: 0,
        })),
        Err(HelperError::Unknown(
            "the helper exceeded its deadline".into()
        ))
    );
}
```

`crates/vpt/tests/helper_client.rs`:

```rust
mod support;

use support::Sandbox;
use vpt_adapters::{BUILT_AGAINST_MAJOR, HelperClient, HelperError};
use vpt_application::ports::{Trash, TrashError};

#[test]
fn helper_version_and_major_refusal_are_read_from_the_fake() {
    let sandbox = Sandbox::new("helper-version");
    sandbox.install_fake_helper();
    let path = sandbox.path().join("bin/vpt-macos");
    assert_eq!(
        HelperClient::new(path.clone()).version().expect("version"),
        "1.0.0"
    );
    assert_eq!(BUILT_AGAINST_MAJOR, 1);
    assert_eq!(
        HelperClient::new(path).version_with_env(&[("VPT_FAKE_VERSION", "2.3.0")]),
        Err(HelperError::MajorMismatch { found: 2 })
    );
}

#[test]
fn absent_helper_has_a_typed_error_without_touching_the_target() {
    let sandbox = Sandbox::new("helper-absent");
    let client = HelperClient::new(sandbox.path().join("missing-helper"));
    assert_eq!(client.version(), Err(HelperError::Absent));
    assert_eq!(
        client.trash(&sandbox.path().join("victim")),
        Err(TrashError::HelperAbsent)
    );
    assert!(!sandbox.fake_log().exists());
}

#[test]
fn trash_uses_the_fake_and_confirms_the_same_requested_path() {
    let sandbox = Sandbox::new("helper-trash");
    sandbox.install_fake_helper();
    let victim = sandbox.path().join("victim.txt");
    std::fs::write(&victim, b"fixture").expect("victim");
    let client = HelperClient::new(sandbox.path().join("bin/vpt-macos"));
    let trash = sandbox.path().join("trash");
    assert_eq!(
        client.trash_with_env(
            &victim,
            &[("VPT_FAKE_TRASH", trash.to_str().expect("utf8"))]
        ),
        Ok(victim.clone())
    );
    assert!(!victim.exists());
    assert_eq!(
        std::fs::read(trash.join("victim.txt")).expect("fake trash"),
        b"fixture"
    );
}

#[test]
fn mismatched_helper_does_not_move_a_file() {
    let sandbox = Sandbox::new("helper-refused-trash");
    sandbox.install_fake_helper();
    let victim = sandbox.path().join("victim.txt");
    std::fs::write(&victim, b"fixture").expect("victim");
    let client = HelperClient::new(sandbox.path().join("bin/vpt-macos"));
    let trash = sandbox.path().join("trash");
    assert_eq!(
        client.trash_with_env(
            &victim,
            &[
                ("VPT_FAKE_VERSION", "2.0.0"),
                ("VPT_FAKE_TRASH", trash.to_str().expect("utf8"))
            ]
        ),
        Err(TrashError::HelperVersion { found: 2 })
    );
    assert_eq!(std::fs::read(&victim).expect("unmoved"), b"fixture");
    assert!(!trash.join("victim.txt").exists());
}
```

Append these tests to `crates/vpt/tests/version.rs`, whose existing imports provide `Sandbox`, `run` and
`stdout`:

```rust
#[test]
fn version_uses_path_only_when_the_selected_configuration_is_absent() {
    let sandbox = Sandbox::new("version-fallback");
    sandbox.install_fake_helper();
    let output = run(sandbox.vpt().args(["--version", "--json"]));
    assert_eq!(output.status.code(), Some(0));
    let document: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("json");
    assert_eq!(document["helper_version"], "1.0.0");
}

#[test]
fn version_honours_environment_config_and_explicit_override() {
    let sandbox = Sandbox::new("version-selected");
    sandbox.install_fake_helper();
    std::fs::create_dir_all(sandbox.config_path().parent().expect("parent")).expect("config dir");
    let missing = sandbox.path().join("missing-helper");
    let settings = format!(
        "config_version = 1\n[helper]\npath = {}\n",
        serde_json::to_string(&missing.to_str().expect("utf8")).expect("path")
    );
    std::fs::write(sandbox.config_path(), settings).expect("selected config");
    let configured = run(sandbox.vpt().args(["--version", "--json"]));
    assert_eq!(configured.status.code(), Some(0));
    let document: serde_json::Value = serde_json::from_str(&stdout(&configured)).expect("json");
    assert!(
        document["helper_version"].is_null(),
        "the configured helper must win over PATH"
    );

    let alternate = sandbox.path().join("alternate.toml");
    let helper = sandbox.path().join("bin/vpt-macos");
    let settings = format!(
        "config_version = 1\n[helper]\npath = {}\n",
        serde_json::to_string(&helper.to_str().expect("utf8")).expect("path")
    );
    std::fs::write(&alternate, settings).expect("alternate config");
    let output = run(sandbox
        .vpt()
        .args(["--version", "--json", "--config"])
        .arg(&alternate));
    assert_eq!(output.status.code(), Some(0));
    let document: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("json");
    assert_eq!(document["helper_version"], "1.0.0");
}

#[test]
fn invalid_existing_configuration_keeps_version_success_without_a_path_fallback() {
    let sandbox = Sandbox::new("version-invalid-config");
    sandbox.install_fake_helper();
    std::fs::create_dir_all(sandbox.config_path().parent().expect("parent")).expect("config dir");
    std::fs::write(sandbox.config_path(), "CANARY-INVALID-CONFIG\n[").expect("invalid config");
    let output = run(sandbox.vpt().args(["--version", "--json"]));
    assert_eq!(output.status.code(), Some(0));
    let text = stdout(&output);
    let document: serde_json::Value = serde_json::from_str(&text).expect("json");
    assert!(document["helper_version"].is_null());
    assert!(!text.contains("CANARY"));
}
```

Add this support to `crates/vpt/tests/support/mod.rs`:

```rust
pub const FAKE_ENGINE: &str = env!("CARGO_BIN_EXE_vpt-fake-engine");

impl Sandbox {
    pub fn install_fake_helper(&self) {
        std::os::unix::fs::symlink(FAKE_ENGINE, self.root.join("bin/vpt-macos")).expect("fake helper");
        std::fs::create_dir_all(self.root.join("trash")).expect("fake trash");
    }

    pub fn fake_log(&self) -> PathBuf {
        self.root.join("fake.log")
    }
}
```

In `Sandbox::vpt`, replace the final `.env("PATH", self.root.join("bin"));` with these chained calls,
keeping its existing session isolation and cleared environment:

```rust
            .env("PATH", self.root.join("bin"))
            .env("VPT_FAKE_TRASH", self.root.join("trash"))
            .env("VPT_FAKE_LOG", self.fake_log());
```

- [ ] **Step 2: Run each red command**

`cargo test -p vpt-protocol limits`

`cargo test -p vpt-adapters helper`

`cargo test -p vpt --features dev-tools --test helper_client --test version`

Expected: the registered tests fail to compile because the bounded reader and helper client are absent.
After the client exists, the version selection tests remain red until the command uses the selected
configuration. The new test modules must be compiled and selected. Zero selected tests or a successful
command does not satisfy this step.

- [ ] **Step 3: Implement the bounded reader, helper client and version command**

`crates/vpt-protocol/src/limits.rs`:

```rust
mod visitor;

use serde::de::DeserializeSeed;
use serde_json::Value;
use std::cell::RefCell;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub bytes: usize,
    pub depth: usize,
    pub text_chars: usize,
    pub array_len: usize,
}

impl Limits {
    pub fn incoming() -> Self {
        Self {
            bytes: 65_536,
            depth: 8,
            text_chars: 65_536,
            array_len: 256,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LimitViolation {
    Bytes(usize),
    Depth(usize),
    Text(usize),
    Array(usize),
    Syntax(String),
}

pub fn read_bounded(bytes: &[u8], limits: &Limits) -> Result<Value, LimitViolation> {
    if bytes.len() > limits.bytes {
        return Err(LimitViolation::Bytes(bytes.len()));
    }
    let context = visitor::Context {
        limits,
        violation: RefCell::new(None),
    };
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    let value = visitor::Node {
        context: &context,
        depth: 0,
        overflow: None,
    }
    .deserialize(&mut decoder)
    .and_then(|value| decoder.end().map(|()| value));
    value.map_err(|_| {
        context
            .violation
            .into_inner()
            .unwrap_or_else(|| LimitViolation::Syntax("invalid JSON document".into()))
    })
}

#[cfg(test)]
mod tests;
```

`crates/vpt-protocol/src/limits/visitor.rs`:

```rust
use super::{LimitViolation, Limits};
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};
use std::cell::RefCell;
use std::fmt;

pub(super) struct Context<'a> {
    pub limits: &'a Limits,
    pub violation: RefCell<Option<LimitViolation>>,
}

impl Context<'_> {
    fn reject<E: de::Error>(&self, violation: LimitViolation) -> E {
        *self.violation.borrow_mut() = Some(violation);
        E::custom("document limit exceeded")
    }

    fn text<E: de::Error>(&self, value: &str) -> Result<(), E> {
        let length = value.chars().count();
        if length > self.limits.text_chars {
            return Err(self.reject(LimitViolation::Text(length)));
        }
        Ok(())
    }
}

pub(super) struct Node<'a, 'b> {
    pub context: &'a Context<'b>,
    pub depth: usize,
    pub overflow: Option<usize>,
}

impl<'de> DeserializeSeed<'de> for Node<'_, '_> {
    type Value = Value;

    fn deserialize<D: de::Deserializer<'de>>(self, decoder: D) -> Result<Value, D::Error> {
        if let Some(length) = self.overflow {
            return Err(self.context.reject(LimitViolation::Array(length)));
        }
        decoder.deserialize_any(self)
    }
}

impl Node<'_, '_> {
    fn enter<E: de::Error>(&self) -> Result<usize, E> {
        let depth = self.depth + 1;
        if depth > self.context.limits.depth {
            return Err(self.context.reject(LimitViolation::Depth(depth)));
        }
        Ok(depth)
    }
}

impl<'de> Visitor<'de> for Node<'_, '_> {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a bounded JSON value")
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Value, E> {
        Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("invalid JSON number"))
    }

    fn visit_unit<E: de::Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Value, E> {
        self.context.text(value)?;
        Ok(Value::String(value.to_owned()))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Value, E> {
        self.context.text(&value)?;
        Ok(Value::String(value))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Value, A::Error> {
        let depth = self.enter()?;
        let mut items = Vec::new();
        loop {
            let overflow =
                (items.len() >= self.context.limits.array_len).then_some(items.len() + 1);
            let next = sequence.next_element_seed(Node {
                context: self.context,
                depth,
                overflow,
            })?;
            match next {
                Some(value) => items.push(value),
                None => return Ok(Value::Array(items)),
            }
        }
    }

    fn visit_map<A: MapAccess<'de>>(self, mut object: A) -> Result<Value, A::Error> {
        let depth = self.enter()?;
        let mut fields = Map::new();
        while let Some(key) = object.next_key_seed(Key(self.context))? {
            let value = object.next_value_seed(Node {
                context: self.context,
                depth,
                overflow: None,
            })?;
            fields.insert(key, value);
        }
        Ok(Value::Object(fields))
    }
}

struct Key<'a, 'b>(&'a Context<'b>);

impl<'de> DeserializeSeed<'de> for Key<'_, '_> {
    type Value = String;

    fn deserialize<D: de::Deserializer<'de>>(self, decoder: D) -> Result<String, D::Error> {
        decoder.deserialize_str(self)
    }
}

impl<'de> Visitor<'de> for Key<'_, '_> {
    type Value = String;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a bounded object key")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<String, E> {
        self.0.text(value)?;
        Ok(value.to_owned())
    }
}
```

`crates/vpt-protocol/src/helper.rs`:

```rust
use serde::Deserialize;

pub const HELPER_SCHEMA: &str = "vpt.helper/1";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct HelperVersion {
    pub schema: String,
    pub version: String,
}

impl HelperVersion {
    pub fn major(&self) -> Option<u32> {
        let mut core = self.version.as_str();
        if let Some((prefix, build)) = core.split_once('+') {
            if !identifiers(build, false) {
                return None;
            }
            core = prefix;
        }
        if let Some((prefix, pre)) = core.split_once('-') {
            if !identifiers(pre, true) {
                return None;
            }
            core = prefix;
        }
        let parts: Vec<_> = core.split('.').collect();
        if parts.len() != 3
            || parts.iter().any(|part| {
                part.is_empty()
                    || !part.bytes().all(|byte| byte.is_ascii_digit())
                    || (part.len() > 1 && part.starts_with('0'))
            })
        {
            return None;
        }
        parts[0].parse().ok()
    }
}

fn identifiers(value: &str, prerelease: bool) -> bool {
    value.split('.').all(|part| {
        !part.is_empty()
            && part
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            && !(prerelease
                && part.len() > 1
                && part.starts_with('0')
                && part.bytes().all(|byte| byte.is_ascii_digit()))
    })
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Posted {
    pub posted: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Trashed {
    pub trashed: String,
}
```

`crates/vpt-adapters/src/helper.rs`:

```rust
mod reply;
use crate::spawn::{self, SpawnError, Status};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::time::Duration;
use vpt_application::ports::{Trash, TrashError};
use vpt_protocol::{HelperVersion, Limits, Posted, Trashed, read_bounded};

pub const CALL_DEADLINE: Duration = Duration::from_secs(5);
pub const BUILT_AGAINST_MAJOR: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HelperError {
    Absent,
    MajorMismatch { found: u32 },
    Failed(String),
    Unknown(String),
}

pub struct HelperClient {
    path: PathBuf,
    deadline: Duration,
    version: RefCell<Option<String>>,
    diagnostics: RefCell<Vec<String>>,
}

impl HelperClient {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            deadline: CALL_DEADLINE,
            version: RefCell::new(None),
            diagnostics: RefCell::new(vec![]),
        }
    }

    pub fn with_deadline(mut self, deadline: Duration) -> Self {
        self.deadline = deadline;
        self
    }

    pub fn take_diagnostics(&self) -> Vec<String> {
        std::mem::take(&mut *self.diagnostics.borrow_mut())
    }

    pub fn version(&self) -> Result<String, HelperError> {
        self.version_with_env(&[])
    }

    pub fn version_with_env(&self, env: &[(&str, &str)]) -> Result<String, HelperError> {
        self.version_using(&|argv| self.call(argv, env))
    }

    fn version_using(
        &self,
        call: &impl Fn(&[String]) -> Result<Value, HelperError>,
    ) -> Result<String, HelperError> {
        if let Some(version) = self.version.borrow().clone() {
            return Ok(version);
        }
        let value = call(&["--version".into()])?;
        reply::schema(&value)?;
        let document: HelperVersion = self.decode(
            value,
            &["schema", "version"],
            "invalid helper version document",
        )?;
        let major = document
            .major()
            .ok_or_else(|| HelperError::Unknown("invalid helper version document".into()))?;
        if major != BUILT_AGAINST_MAJOR {
            return Err(HelperError::MajorMismatch { found: major });
        }
        *self.version.borrow_mut() = Some(document.version.clone());
        Ok(document.version)
    }

    pub fn notify(&self, title: &str, body: &str) -> Result<(), HelperError> {
        self.notify_with_env(title, body, &[])
    }

    pub fn notify_with_env(
        &self,
        title: &str,
        body: &str,
        env: &[(&str, &str)],
    ) -> Result<(), HelperError> {
        self.notify_using(title, body, &|argv| self.call(argv, env))
    }

    fn notify_using(
        &self,
        title: &str,
        body: &str,
        call: &impl Fn(&[String]) -> Result<Value, HelperError>,
    ) -> Result<(), HelperError> {
        self.version_using(call)?;
        let argv = ["notify", "--title", title, "--body", body].map(str::to_owned);
        let reply: Posted =
            self.decode(call(&argv)?, &["posted"], "invalid helper notify document")?;
        if reply.posted {
            Ok(())
        } else {
            Err(HelperError::Failed("the helper did not post".into()))
        }
    }

    pub fn trash_with_env(&self, path: &Path, env: &[(&str, &str)]) -> Result<PathBuf, TrashError> {
        self.trash_using(path, &|argv| self.call(argv, env))
    }

    fn trash_using(
        &self,
        path: &Path,
        call: &impl Fn(&[String]) -> Result<Value, HelperError>,
    ) -> Result<PathBuf, TrashError> {
        self.version_using(call).map_err(reply::trash_error)?;
        let path_text = path
            .to_str()
            .ok_or_else(|| TrashError::Failed("the Trash path is not UTF-8".into()))?;
        let argv = ["trash".to_owned(), path_text.to_owned()];
        let reply: Trashed = self
            .decode(
                call(&argv).map_err(reply::trash_error)?,
                &["trashed"],
                "invalid helper trash document",
            )
            .map_err(reply::trash_error)?;
        if Path::new(&reply.trashed) != path {
            return Err(TrashError::Unknown(
                "the helper trash reply names another path".into(),
            ));
        }
        Ok(path.to_path_buf())
    }

    fn decode<T: DeserializeOwned>(
        &self,
        value: Value,
        supported: &[&str],
        invalid: &str,
    ) -> Result<T, HelperError> {
        let fields = value
            .as_object()
            .ok_or_else(|| HelperError::Unknown(invalid.into()))?;
        self.diagnostics.borrow_mut().extend(
            fields
                .keys()
                .filter(|key| !supported.contains(&key.as_str()))
                .map(|key| {
                    format!(
                        "unknown additive helper field: {}",
                        key.chars()
                            .flat_map(char::escape_default)
                            .collect::<String>()
                    )
                }),
        );
        serde_json::from_value(value).map_err(|_| HelperError::Unknown(invalid.into()))
    }

    fn call(&self, argv: &[String], env: &[(&str, &str)]) -> Result<Value, HelperError> {
        self.call_using(argv, env, spawn::run_with_env)
    }

    fn call_using(
        &self,
        argv: &[String],
        env: &[(&str, &str)],
        run: impl FnOnce(
            &[String],
            &[u8],
            Duration,
            usize,
            &[(&str, &str)],
        ) -> Result<spawn::Outcome, SpawnError>,
    ) -> Result<Value, HelperError> {
        let path = self
            .path
            .to_str()
            .ok_or_else(|| HelperError::Failed("the helper path is not UTF-8".into()))?;
        let mut full = vec![path.to_owned()];
        full.extend_from_slice(argv);
        let limits = Limits::incoming();
        let outcome = run(
            &full,
            b"",
            self.deadline,
            limits.bytes.saturating_add(1),
            env,
        )
        .map_err(|error| match error {
            SpawnError::NotFound(_) => HelperError::Absent,
            SpawnError::Io(_) => HelperError::Failed("the helper could not start".into()),
        })?;
        match outcome.status {
            Status::Exited(0) => read_bounded(&outcome.stdout, &limits)
                .map_err(|_| HelperError::Unknown("invalid or oversized helper document".into())),
            Status::Exited(code) => Err(HelperError::Failed(format!("the helper exited {code}"))),
            Status::Signaled(signal) => Err(HelperError::Failed(format!(
                "the helper died on signal {signal}"
            ))),
            Status::DeadlineExceeded => Err(HelperError::Unknown(
                "the helper exceeded its deadline".into(),
            )),
            Status::Interrupted => Err(HelperError::Unknown("interrupted".into())),
        }
    }
}

impl Trash for HelperClient {
    fn trash(&self, path: &Path) -> Result<PathBuf, TrashError> {
        self.trash_with_env(path, &[])
    }

    fn take_diagnostics(&self) -> Vec<String> {
        HelperClient::take_diagnostics(self)
    }
}

#[cfg(test)]
mod tests;
```

`crates/vpt-adapters/src/helper/reply.rs`:

```rust
use super::HelperError;
use serde_json::Value;
use vpt_application::ports::TrashError;
use vpt_protocol::HELPER_SCHEMA;

pub(super) fn schema(value: &Value) -> Result<(), HelperError> {
    let Some(schema) = value.get("schema").and_then(Value::as_str) else {
        return Err(HelperError::Unknown("invalid helper schema".into()));
    };
    if schema == HELPER_SCHEMA {
        return Ok(());
    }
    let major = schema
        .strip_prefix("vpt.helper/")
        .filter(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
        .and_then(|part| part.parse::<u32>().ok());
    match major {
        Some(found) => Err(HelperError::Unknown(format!(
            "unsupported helper schema major {found}"
        ))),
        None => Err(HelperError::Unknown("invalid helper schema".into())),
    }
}

pub(super) fn trash_error(error: HelperError) -> TrashError {
    match error {
        HelperError::Absent => TrashError::HelperAbsent,
        HelperError::MajorMismatch { found } => TrashError::HelperVersion { found },
        HelperError::Failed(detail) => TrashError::Failed(detail),
        HelperError::Unknown(detail) => TrashError::Unknown(detail),
    }
}
```

`crates/vpt/src/commands/version.rs`:

```rust
use crate::cli::output::Outcome;
use crate::compose::Environment;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use vpt_adapters::HelperClient;
use vpt_adapters::config::{ConfigError, from_table, load_file};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn document(helper_version: Option<&str>) -> Value {
    json!({"schema":"vpt.result/1", "command":"version", "version":VERSION, "helper_version":helper_version})
}

pub fn human(helper_version: Option<&str>) -> String {
    match helper_version {
        Some(helper) => format!("vpt {VERSION} (helper {helper})\n"),
        None => format!("vpt {VERSION} (helper absent)\n"),
    }
}

pub fn run(environment: &Environment, config: Option<&Path>) -> Outcome {
    let selected = config
        .map(Path::to_path_buf)
        .unwrap_or_else(|| environment.config_path());
    let helper_path = match load_file(&selected) {
        Ok(table) => from_table(&table, &environment.home_dir())
            .ok()
            .map(|settings| settings.helper_path),
        Err(ConfigError::Missing(_)) => Some(PathBuf::from("vpt-macos")),
        Err(_) => None,
    };
    let helper = helper_path.map(HelperClient::new);
    let version = helper.as_ref().and_then(|helper| helper.version().ok());
    let diagnostics = helper
        .as_ref()
        .map(HelperClient::take_diagnostics)
        .unwrap_or_default();
    let mut result = document(version.as_deref());
    let mut text = human(version.as_deref());
    if !diagnostics.is_empty() {
        result["diagnostics"] = json!(diagnostics);
        for diagnostic in &diagnostics {
            text.push_str(diagnostic);
            text.push('\n');
        }
    }
    Outcome::Success {
        document: result,
        human: text,
    }
}
```

In `crates/vpt/src/lib.rs`, use this dispatch arm with its existing `environment`:

```rust
        Verb::Version => commands::version::run(&environment, invocation.config.as_deref()),
```

In `crates/vpt/src/lib.rs`, insert this as the first statement of `pub fn run() -> !`, before reading
arguments or constructing the environment. Retain it in every later replacement of that function:

```rust
    vpt_adapters::install_interrupt_handlers();
```

Commands collect `Trash::take_diagnostics` after cleanup or reconciliation. `HelperVersion { found }`
remains typed through the application and maps to exit 3, rule `helper_version`, when Trash is required.

- [ ] **Step 4: Verify green and the guards**

Run: `cargo fmt --all`

Run `cargo test --workspace --features dev-tools`, `cargo fmt --all -- --check` and
`cargo clippy --workspace --all-targets --features dev-tools -- -D warnings`. Expected: all tests pass.
Reader and client unit tests use private injected operations; integration tests spawn only the
development fake. No helper test uses wall-clock assertions or a real helper.

Mutation-check the array overflow guard, the depth guard, the extra stdout byte, the compatibility
preflight and the returned Trash path check. Verify each changed line before running its named test; each
mutant must fail, then restore the implementation and rerun green. Count physical lines after rustfmt;
the private child files keep every handwritten Rust file below 500 lines.

- [ ] **Step 5: Commit**

Run: `cargo fmt --all`

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(helper): validate replies and cache compatible helper versions"
```

______________________________________________________________________

### Task 26: Notifications: `vpt.event/1` and the three modes

**Files:**

- Create: `crates/vpt-protocol/src/event.rs`.
- Create: `crates/vpt-adapters/src/notify/mod.rs`, `crates/vpt-adapters/src/notify/tests.rs`,
  `crates/vpt-adapters/src/notify/delivery.rs`, `crates/vpt-adapters/src/notify/delivery/tests.rs`.
- Modify: `crates/vpt-protocol/src/lib.rs`, `crates/vpt-adapters/src/lib.rs`.
- Test: `crates/vpt/tests/notify.rs`.

**Interfaces:**

- Consumes:
  `Notification { pub event: EventKind, pub state: EventState, pub recording: Option<RecordingId>, pub detail: String, pub counts: Vec<(String, u64)>, pub paths: Vec<(String, PathBuf)>, pub occurred_at: UtcInstant }`;
  `EventKind::as_str(self) -> &'static str`, `EventState::as_str(self) -> &'static str`,
  `RecordingId::as_str(&self) -> &str`, `UtcInstant::rfc3339(self) -> String`.

- Consumes: `Notifier::deliver(&self, notification: &Notification) -> DeliveryOutcome`,
  `Notifier::take_diagnostics(&self) -> Vec<String>`,
  `DeliveryOutcome::{Delivered, Suppressed, Failed(String)}`;
  `HelperClient::notify_with_env(&self, title: &str, body: &str, env: &[(&str, &str)]) -> Result<(), HelperError>`,
  `HelperClient::take_diagnostics(&self) -> Vec<String>`;
  `spawn::run_with_env(argv: &[String], stdin: &[u8], deadline: Duration, output_limit: usize, env: &[(&str, &str)]) -> Result<spawn::Outcome, spawn::SpawnError>`.

- Produces protocol root exports: `EVENT_SCHEMA: &str = "vpt.event/1"`,
  `EventDocument { pub schema: String, pub event: String, pub state: String, pub recording: Option<String>, pub detail: String, pub counts: serde_json::Map<String, serde_json::Value>, pub paths: serde_json::Map<String, serde_json::Value>, pub occurred_at: String }`,
  implementing `Serialize`.

- Produces adapter root exports: `notification_document(notification: &Notification) -> EventDocument`,
  `notification_tokens(notification: &Notification, argv: &[String]) -> Vec<String>`,
  `DesktopNotifier::new(helper: HelperClient) -> DesktopNotifier`,
  `DesktopNotifier::with_env(self, env: Vec<(String, String)>) -> DesktopNotifier`,
  `CommandNotifier::new(argv: Vec<String>, fallback: DesktopNotifier) -> CommandNotifier`,
  `CommandNotifier::with_env(self, env: Vec<(String, String)>) -> CommandNotifier`, `OffNotifier`,
  `NOTIFY_DEADLINE: Duration = Duration::from_secs(5)`. All three implement `Notifier`. The desktop and
  command implementations retain diagnostics until drained. An absent desktop helper disables subsequent
  attempts for that command and records exactly `desktop notifications disabled: helper absent` once.
  Command failure is recorded before one fallback attempt, including its exit code when available.
  Delivery never changes the work's exit. Production command paths drain diagnostics on success and
  failure into their final result or error.

- [ ] **Step 1: Register and write the failing tests**

Add this private module and its curated exports to the adapters crate root:

```rust
mod notify;
pub use notify::{CommandNotifier, DesktopNotifier, NOTIFY_DEADLINE, OffNotifier,
    document as notification_document, tokens as notification_tokens};
```

Create `crates/vpt-adapters/src/notify/mod.rs` with:

```rust
mod delivery;
pub use delivery::{CommandNotifier, DesktopNotifier, NOTIFY_DEADLINE, OffNotifier};

#[cfg(test)]
mod tests;
```

Create `crates/vpt-adapters/src/notify/delivery.rs` with:

```rust
#[cfg(test)]
mod tests;
```

`crates/vpt-adapters/src/notify/tests.rs`:

```rust
use super::*;
use std::path::PathBuf;
use vpt_domain::identity::RecordingId;
use vpt_domain::notification::{EventKind, Notification};
use vpt_domain::time::UtcInstant;

fn review_needed() -> Notification {
    Notification::attention(
        EventKind::ReviewNeeded,
        Some(RecordingId::parse("2026-08-24T144736-4f3ab19c02de").expect("id")),
        "6 spans to review".into(),
        vec![("note".into(), PathBuf::from("/sandbox/transcripts/a.md"))],
        UtcInstant {
            secs: 1_789_599_200,
        },
    )
}

#[test]
fn event_document_contains_the_spec_fields_and_correct_utc_timestamp() {
    let mut notification = review_needed();
    notification.counts = vec![("numeric".into(), 3), ("diagnostics".into(), 0)];
    let json = serde_json::to_value(document(&notification)).expect("json");
    assert_eq!(json["schema"], "vpt.event/1");
    assert_eq!(json["event"], "review_needed");
    assert_eq!(json["state"], "needs_attention");
    assert_eq!(json["recording"], "2026-08-24T144736-4f3ab19c02de");
    assert_eq!(json["counts"]["numeric"], 3);
    assert_eq!(json["paths"]["note"], "/sandbox/transcripts/a.md");
    assert_eq!(json["occurred_at"], "2026-09-16T22:53:20Z");
}

#[test]
fn all_tokens_are_replaced_and_inserted_values_are_never_scanned() {
    let mut notification = review_needed();
    notification.detail = "use {count}".into();
    notification.counts = vec![("numeric".into(), 3), ("other".into(), 2)];
    assert_eq!(tokens(&notification, &["{detail}".into()]), ["use {count}"]);
    let argv = [
        "notify-command",
        "{event}/{state}/{id}/{count}/{path}",
        "é{unknown}{detail}",
    ]
    .map(str::to_owned);
    assert_eq!(
        tokens(&notification, &argv),
        [
            "notify-command",
            "review_needed/needs_attention/2026-08-24T144736-4f3ab19c02de/5//sandbox/transcripts/a.md",
            "é{unknown}use {count}"
        ]
    );
    let bare = Notification::failed(
        EventKind::IngestFailed,
        "boom".into(),
        UtcInstant { secs: 0 },
    );
    assert_eq!(
        tokens(&bare, &["{id}".into(), "{path}".into(), "{detail}".into()]),
        ["", "", "boom"]
    );
}
```

`crates/vpt-adapters/src/notify/delivery/tests.rs`:

```rust
use super::*;
use std::path::PathBuf;
use vpt_domain::notification::EventKind;
use vpt_domain::time::UtcInstant;

fn event() -> Notification {
    Notification::failed(
        EventKind::IngestFailed,
        "unreadable source".into(),
        UtcInstant { secs: 7 },
    )
}

fn desktop() -> DesktopNotifier {
    DesktopNotifier::new(HelperClient::new(PathBuf::from("/sandbox/fake-helper")))
}

#[test]
fn absent_desktop_helper_is_reported_once_and_subsequent_attempts_are_suppressed() {
    let notifier = desktop();
    let attempts = Cell::new(0);
    for _ in 0..2 {
        assert_eq!(
            notifier.deliver_using(|| {
                attempts.set(attempts.get() + 1);
                Err(HelperError::Absent)
            }),
            DeliveryOutcome::Suppressed
        );
    }
    assert_eq!(attempts.get(), 1);
    assert_eq!(
        notifier.take_diagnostics(),
        ["desktop notifications disabled: helper absent"]
    );
    assert!(notifier.take_diagnostics().is_empty());
}

#[test]
fn failed_command_is_recorded_before_one_fallback_and_keeps_its_status() {
    let notifier = CommandNotifier::new(vec!["notify-command".into()], desktop());
    let attempts = Cell::new(0);
    let result = notifier.deliver_using(
        &event(),
        |_, _, _, _, _| {
            Ok(spawn::Outcome {
                status: Status::Exited(7),
                stdout: b"CANARY-STDOUT".to_vec(),
                stderr: b"CANARY-STDERR".to_vec(),
                group: 0,
            })
        },
        || {
            attempts.set(attempts.get() + 1);
            assert_eq!(*notifier.diagnostics.borrow(), ["notify command exited 7"]);
            notifier.fallback.deliver_using(|| Err(HelperError::Absent))
        },
    );
    assert_eq!(attempts.get(), 1);
    assert_eq!(
        result,
        DeliveryOutcome::Failed("notify command exited 7".into())
    );
    assert_eq!(
        notifier.take_diagnostics(),
        [
            "notify command exited 7",
            "desktop notifications disabled: helper absent"
        ]
    );
}

#[test]
fn command_sends_substituted_argv_and_event_stdin_together_without_fallback() {
    let notifier = CommandNotifier::new(vec!["notify-command".into(), "{event}".into()], desktop());
    let result = notifier.deliver_using(
        &event(),
        |argv, stdin, deadline, limit, _| {
            assert_eq!(argv, ["notify-command", "ingest_failed"]);
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(stdin).expect("event")["schema"],
                "vpt.event/1"
            );
            assert_eq!(deadline, Duration::from_secs(5));
            assert_eq!(limit, spawn::OUTPUT_LIMIT);
            Ok(spawn::Outcome {
                status: Status::Exited(0),
                stdout: vec![],
                stderr: vec![],
                group: 0,
            })
        },
        || panic!("unexpected fallback"),
    );
    assert_eq!(result, DeliveryOutcome::Delivered);
    assert!(notifier.take_diagnostics().is_empty());
}

#[test]
fn spawn_failure_diagnostics_never_copy_untrusted_details() {
    let notifier = CommandNotifier::new(vec![], desktop());
    let result = notifier.deliver_using(
        &event(),
        |_, _, _, _, _| Err(SpawnError::Io("CANARY".into())),
        || DeliveryOutcome::Delivered,
    );
    assert_eq!(
        result,
        DeliveryOutcome::Failed("notify command could not start".into())
    );
    assert_eq!(
        notifier.take_diagnostics(),
        ["notify command could not start"]
    );
}

#[test]
fn incompatible_desktop_helper_is_diagnostic_and_off_has_no_diagnostics() {
    let notifier = desktop();
    assert!(matches!(
        notifier.deliver_using(|| Err(HelperError::MajorMismatch { found: 2 })),
        DeliveryOutcome::Failed(_)
    ));
    assert_eq!(
        notifier.take_diagnostics(),
        ["desktop notification refused: helper major version 2"]
    );
    assert_eq!(OffNotifier.deliver(&event()), DeliveryOutcome::Suppressed);
    assert!(OffNotifier.take_diagnostics().is_empty());
}
```

`crates/vpt/tests/notify.rs`:

```rust
mod support;

use std::path::Path;
use support::{FAKE_ENGINE, Sandbox};
use vpt_adapters::{CommandNotifier, DesktopNotifier, HelperClient};
use vpt_application::ports::{DeliveryOutcome, Notifier};
use vpt_domain::notification::{EventKind, Notification};
use vpt_domain::time::UtcInstant;

fn event() -> Notification {
    Notification::failed(
        EventKind::IngestFailed,
        "the recordings directory is unreadable".into(),
        UtcInstant { secs: 7 },
    )
}

fn lines(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

fn env(sandbox: &Sandbox) -> Vec<(String, String)> {
    vec![(
        "VPT_FAKE_LOG".into(),
        sandbox.fake_log().to_string_lossy().into_owned(),
    )]
}

#[test]
fn desktop_posts_through_the_fake_and_retains_absence_diagnostics() {
    let sandbox = Sandbox::new("notify-desktop");
    sandbox.install_fake_helper();
    let notifier = DesktopNotifier::new(HelperClient::new(sandbox.path().join("bin/vpt-macos")))
        .with_env(env(&sandbox));
    assert_eq!(notifier.deliver(&event()), DeliveryOutcome::Delivered);
    assert_eq!(
        lines(&sandbox.fake_log()),
        ["notify\tvpt: ingest_failed\tthe recordings directory is unreadable"]
    );
    let absent = DesktopNotifier::new(HelperClient::new(sandbox.path().join("missing")));
    assert_eq!(absent.deliver(&event()), DeliveryOutcome::Suppressed);
    assert_eq!(absent.deliver(&event()), DeliveryOutcome::Suppressed);
    assert_eq!(
        absent.take_diagnostics(),
        ["desktop notifications disabled: helper absent"]
    );
}

#[test]
fn command_gets_tokens_and_json_and_nonzero_falls_back_once() {
    let sandbox = Sandbox::new("notify-command");
    sandbox.install_fake_helper();
    let desktop = DesktopNotifier::new(HelperClient::new(sandbox.path().join("bin/vpt-macos")));
    let argv = [
        FAKE_ENGINE,
        "command-sink",
        "--event",
        "{event}",
        "--state",
        "{state}",
    ]
    .map(str::to_owned)
    .to_vec();
    let mut child_env = env(&sandbox);
    child_env.push(("VPT_FAKE_EXIT".into(), "7".into()));
    let notifier = CommandNotifier::new(argv, desktop).with_env(child_env);
    assert_eq!(
        notifier.deliver(&event()),
        DeliveryOutcome::Failed("notify command exited 7".into())
    );
    assert_eq!(notifier.take_diagnostics(), ["notify command exited 7"]);
    let lines = lines(&sandbox.fake_log());
    assert_eq!(lines.len(), 2);
    let fields: Vec<_> = lines[0].splitn(3, '\t').collect();
    assert_eq!(fields[0], "command-sink");
    assert_eq!(fields[1], "--event ingest_failed --state failed");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(fields[2]).expect("event")["schema"],
        "vpt.event/1"
    );
    assert_eq!(
        lines[1],
        "notify\tvpt: ingest_failed\tthe recordings directory is unreadable"
    );
}
```

- [ ] **Step 2: Run each red command**

`cargo test -p vpt-adapters notify`

`cargo test -p vpt --features dev-tools --test notify`

Expected: compile errors name the absent document conversion, token substitution and notifier types. The
new test modules must be compiled and selected. Zero selected tests or a successful command does not
satisfy this step.

- [ ] **Step 3: Implement event encoding, token substitution and delivery**

Add `mod event; pub use event::{EVENT_SCHEMA, EventDocument};` to the protocol crate root.

`crates/vpt-protocol/src/event.rs`:

```rust
use serde::Serialize;
use serde_json::{Map, Value};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct EventDocument {
    pub schema: String,
    pub event: String,
    pub state: String,
    pub recording: Option<String>,
    pub detail: String,
    pub counts: Map<String, Value>,
    pub paths: Map<String, Value>,
    pub occurred_at: String,
}

pub const EVENT_SCHEMA: &str = "vpt.event/1";
```

`crates/vpt-adapters/src/notify/mod.rs`:

```rust
mod delivery;
pub use delivery::{CommandNotifier, DesktopNotifier, NOTIFY_DEADLINE, OffNotifier};

use serde_json::{Map, Value};
use vpt_domain::notification::Notification;
use vpt_protocol::{EVENT_SCHEMA, EventDocument};

pub fn document(notification: &Notification) -> EventDocument {
    let counts = notification
        .counts
        .iter()
        .map(|(key, count)| (key.clone(), Value::from(*count)))
        .collect::<Map<_, _>>();
    let paths = notification
        .paths
        .iter()
        .map(|(key, path)| {
            (
                key.clone(),
                Value::String(path.to_string_lossy().into_owned()),
            )
        })
        .collect::<Map<_, _>>();
    EventDocument {
        schema: EVENT_SCHEMA.into(),
        event: notification.event.as_str().into(),
        state: notification.state.as_str().into(),
        recording: notification
            .recording
            .as_ref()
            .map(|id| id.as_str().to_owned()),
        detail: notification.detail.clone(),
        counts,
        paths,
        occurred_at: notification.occurred_at.rfc3339(),
    }
}

pub fn tokens(notification: &Notification, argv: &[String]) -> Vec<String> {
    let count = notification
        .counts
        .iter()
        .map(|(_, count)| u128::from(*count))
        .sum::<u128>()
        .to_string();
    let path = notification
        .paths
        .first()
        .map(|(_, path)| path.to_string_lossy().into_owned())
        .unwrap_or_default();
    let id = notification
        .recording
        .as_ref()
        .map(|id| id.as_str())
        .unwrap_or_default();
    let replacements = [
        ("{event}", notification.event.as_str()),
        ("{state}", notification.state.as_str()),
        ("{id}", id),
        ("{detail}", &notification.detail),
        ("{count}", &count),
        ("{path}", &path),
    ];
    argv.iter()
        .map(|word| {
            let mut output = String::new();
            let mut remaining = word.as_str();
            while !remaining.is_empty() {
                if let Some((token, value)) = replacements
                    .iter()
                    .find(|(token, _)| remaining.starts_with(token))
                {
                    output.push_str(value);
                    remaining = &remaining[token.len()..];
                } else if let Some(character) = remaining.chars().next() {
                    output.push(character);
                    remaining = &remaining[character.len_utf8()..];
                }
            }
            output
        })
        .collect()
}

#[cfg(test)]
mod tests;
```

`crates/vpt-adapters/src/notify/delivery.rs`:

```rust
use super::{document, tokens};
use crate::helper::{HelperClient, HelperError};
use crate::spawn::{self, SpawnError, Status};
use std::cell::{Cell, RefCell};
use std::time::Duration;
use vpt_application::ports::{DeliveryOutcome, Notifier};
use vpt_domain::notification::Notification;

pub const NOTIFY_DEADLINE: Duration = Duration::from_secs(5);

pub struct DesktopNotifier {
    helper: HelperClient,
    env: Vec<(String, String)>,
    disabled: Cell<bool>,
    diagnostics: RefCell<Vec<String>>,
}

impl DesktopNotifier {
    pub fn new(helper: HelperClient) -> Self {
        Self {
            helper,
            env: vec![],
            disabled: Cell::new(false),
            diagnostics: RefCell::new(vec![]),
        }
    }

    pub fn with_env(mut self, env: Vec<(String, String)>) -> Self {
        self.env = env;
        self
    }

    fn deliver_using(&self, call: impl FnOnce() -> Result<(), HelperError>) -> DeliveryOutcome {
        if self.disabled.get() {
            return DeliveryOutcome::Suppressed;
        }
        let result = call();
        self.diagnostics
            .borrow_mut()
            .extend(self.helper.take_diagnostics());
        match result {
            Ok(()) => DeliveryOutcome::Delivered,
            Err(HelperError::Absent) => {
                self.disabled.set(true);
                self.diagnostics
                    .borrow_mut()
                    .push("desktop notifications disabled: helper absent".into());
                DeliveryOutcome::Suppressed
            }
            Err(HelperError::MajorMismatch { found }) => {
                let detail = format!("desktop notification refused: helper major version {found}");
                self.diagnostics.borrow_mut().push(detail.clone());
                DeliveryOutcome::Failed(detail)
            }
            Err(HelperError::Failed(detail) | HelperError::Unknown(detail)) => {
                self.diagnostics.borrow_mut().push(detail.clone());
                DeliveryOutcome::Failed(detail)
            }
        }
    }
}

fn borrowed(env: &[(String, String)]) -> Vec<(&str, &str)> {
    env.iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect()
}

impl Notifier for DesktopNotifier {
    fn deliver(&self, notification: &Notification) -> DeliveryOutcome {
        let title = format!("vpt: {}", notification.event.as_str());
        self.deliver_using(|| {
            self.helper
                .notify_with_env(&title, &notification.detail, &borrowed(&self.env))
        })
    }

    fn take_diagnostics(&self) -> Vec<String> {
        std::mem::take(&mut *self.diagnostics.borrow_mut())
    }
}

pub struct CommandNotifier {
    argv: Vec<String>,
    fallback: DesktopNotifier,
    env: Vec<(String, String)>,
    diagnostics: RefCell<Vec<String>>,
}

impl CommandNotifier {
    pub fn new(argv: Vec<String>, fallback: DesktopNotifier) -> Self {
        Self {
            argv,
            fallback,
            env: vec![],
            diagnostics: RefCell::new(vec![]),
        }
    }

    pub fn with_env(mut self, env: Vec<(String, String)>) -> Self {
        self.fallback.env = env.clone();
        self.env = env;
        self
    }

    fn deliver_using(
        &self,
        notification: &Notification,
        run: impl FnOnce(
            &[String],
            &[u8],
            Duration,
            usize,
            &[(&str, &str)],
        ) -> Result<spawn::Outcome, SpawnError>,
        fallback: impl FnOnce() -> DeliveryOutcome,
    ) -> DeliveryOutcome {
        let argv = tokens(notification, &self.argv);
        let result = match serde_json::to_vec(&document(notification)) {
            Ok(body) => run(
                &argv,
                &body,
                NOTIFY_DEADLINE,
                spawn::OUTPUT_LIMIT,
                &borrowed(&self.env),
            ),
            Err(_) => return self.failed("notify event could not be encoded".into(), fallback),
        };
        let detail = match result {
            Ok(outcome) => match outcome.status {
                Status::Exited(0) => return DeliveryOutcome::Delivered,
                Status::Exited(code) => format!("notify command exited {code}"),
                Status::Signaled(signal) => format!("notify command died on signal {signal}"),
                Status::DeadlineExceeded => "notify command exceeded its deadline".into(),
                Status::Interrupted => "notify command interrupted".into(),
            },
            Err(_) => "notify command could not start".into(),
        };
        self.failed(detail, fallback)
    }

    fn failed(
        &self,
        detail: String,
        fallback: impl FnOnce() -> DeliveryOutcome,
    ) -> DeliveryOutcome {
        self.diagnostics.borrow_mut().push(detail.clone());
        let _ = fallback();
        self.diagnostics
            .borrow_mut()
            .extend(self.fallback.take_diagnostics());
        DeliveryOutcome::Failed(detail)
    }
}

impl Notifier for CommandNotifier {
    fn deliver(&self, notification: &Notification) -> DeliveryOutcome {
        self.deliver_using(notification, spawn::run_with_env, || {
            self.fallback.deliver(notification)
        })
    }

    fn take_diagnostics(&self) -> Vec<String> {
        std::mem::take(&mut *self.diagnostics.borrow_mut())
    }
}

pub struct OffNotifier;

impl Notifier for OffNotifier {
    fn deliver(&self, _notification: &Notification) -> DeliveryOutcome {
        DeliveryOutcome::Suppressed
    }
}

#[cfg(test)]
mod tests;
```

The diagnostic queue records the command failure before fallback; the command layer emits those messages
in its final document or run log. This preserves the single-document JSON output contract. All consumers,
including config refusal, ingest and retention, drain `take_diagnostics`; the returned `DeliveryOutcome`
does not replace the work's result.

- [ ] **Step 4: Verify green and diagnostic retention**

Run: `cargo fmt --all`

Run `cargo test --workspace --features dev-tools`, `cargo fmt --all -- --check` and
`cargo clippy --workspace --all-targets --features dev-tools -- -D warnings`. Expected: all tests pass.
Mutate substitution to rescan an inserted `{count}`, disable the missing-helper suppression, and drop the
queued command error in turn; each corresponding test must fail. Restore the source between mutants and
rerun green. Production composition tests in Tasks 27 and 32 assert that these diagnostics reach output
while the underlying work keeps its own exit status.

- [ ] **Step 5: Commit**

Run: `cargo fmt --all`

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(notify): retain delivery diagnostics and substitute tokens once"
```

______________________________________________________________________

### Task 27: `vpt ingest` at the command line: composition, the lock, repair before work

**Files:**

- Create: `crates/vpt-adapters/src/clock.rs`
- Modify: `crates/vpt-adapters/src/lib.rs`
- Modify: `crates/vpt/src/compose.rs`, `crates/vpt/Cargo.toml`
- Create: `crates/vpt/src/compose/clock.rs`, `crates/vpt/src/compose/errors.rs`,
  `crates/vpt/src/compose/ledger.rs`, `crates/vpt/src/compose/runtime.rs`,
  `crates/vpt/src/compose/tests.rs`
- Create: `crates/vpt/src/commands/ingest.rs`, `crates/vpt/src/documents/mod.rs`,
  `crates/vpt/src/documents/record.rs`
- Modify: `crates/vpt/src/lib.rs`, `crates/vpt/src/commands/mod.rs`
- Test: `crates/vpt/tests/ingest.rs`; `crates/vpt/tests/support/mod.rs` gains `write_config` and
  `add_recording`

**Interfaces:**

- Consumes adapter capabilities:
  `resolve(settings: &Settings, config_dir: &Path) -> Result<Roots, RootError>`;
  `Roots::{create_state_dir(&self) -> Result<(), RootError>, create_leaves(&self) -> Result<(), RootError>}`;
  `RootDir::open(path: &Path) -> Result<RootDir, ContainedError>`;
  `WriteLock::acquire(state: &RootDir, wait: Duration) -> Result<WriteLock, LockError>`;
  `SqliteLedger::{open(state: &RootDir) -> Result<SqliteLedger, OpenError>, open_read_only(state: &Path) -> Result<Option<SqliteLedger>, OpenError>}`;
  `VoiceMemosStore::{open(path: &Path) -> Result<VoiceMemosStore, ContainedError>, with_titles_root(self, state: &RootDir, refresh: bool) -> VoiceMemosStore}`;
  `ClonefileArchive::open_read_only(path: &Path) -> Result<ClonefileArchive, ContainedError>`;
  `FilesystemStores::open_read_only(paths: &[PathBuf]) -> Result<FilesystemStores, ContainedError>`;
  `HelperClient::{new(path: PathBuf) -> HelperClient, take_diagnostics(&self) -> Vec<String>}`.

- Consumes `ConfigError::PathEscape(String)` and `LedgerError::PathEscape(PathBuf)`; produces
  `compose::errors::{config_error(error: &ConfigError, path: &Path) -> ErrorDocument, ledger_error(error: LedgerError) -> ErrorDocument}`
  with exit 3 `path_escape` preserved.

- Consumes application capabilities:
  `repair_publications<J: PublicationJournal, S: Stores>(journal: &J, stores: &S, render: impl Fn(&Path) -> Option<Vec<u8>>) -> Result<RepairReport, RepairError>`;
  `Ingest::run(&self, mode: &Mode) -> Result<IngestReport, Box<IngestError>>`;
  `Mode { pub dry_run: bool, pub once: Option<PathBuf> }`;
  `Notifier::{deliver(&self, notification: &Notification) -> DeliveryOutcome, take_diagnostics(&self) -> Vec<String>}`.

- Produces crate-private composition: `AccessMode::{ReadOnly, Mutating}`;
  `Operation::{Ingest, Retention}`;
  `Runtime::load(environment: &Environment, config: Option<&Path>, access: AccessMode, operation: Operation) -> Result<Runtime, Box<ErrorDocument>>`;
  `Runtime::mutating(&self) -> Result<RepairReport, Box<ErrorDocument>>`;
  `Runtime::trash(&self) -> &dyn Trash`; `Runtime::diagnostics(&self) -> Vec<String>`;
  `WRITE_LOCK_WAIT: Duration = Duration::from_secs(5)`. Runtime fields:
  `settings: Settings, roots: Roots, ledger: RuntimeLedger, recorder: Option<VoiceMemosStore>, archive: ClonefileArchive, stores: FilesystemStores, clock: RuntimeClock, helper: HelperClient, notifier: Box<dyn Notifier>`,
  all `pub(crate)`; `_lock: Option<WriteLock>` is private and retained for the runtime lifetime. Only
  `Operation::Ingest` opens the recorder. Writable composition passes one opened state `RootDir` to the
  lock, ledger, and title builder; neither ledger nor title setup reopens its path.
  `RuntimeLedger::{Sqlite(SqliteLedger), Empty(MemoryLedger)}` implements `RecordingLedger` and
  `PublicationJournal`;
  `RuntimeLedger::read_only(path: &Path) -> Result<RuntimeLedger, Box<ErrorDocument>>`.
  `RuntimeClock::from_environment(environment: &Environment) -> Result<RuntimeClock, Box<ErrorDocument>>`
  implements `Clock`; only `dev-tools` reads `VPT_TEST_NOW_SECS`.

- Produces adapter root export `SystemClock`, implementing
  `Clock::{now(&self) -> UtcInstant, offset_at(&self, at: UtcInstant) -> UtcOffset}`.

- Produces command-private `documents::record_json(record: &RecordingRecord) -> serde_json::Value`;
  `commands::ingest::run(runtime: &Runtime, dry_run: bool, once: Option<PathBuf>) -> Outcome`.

- Test support adds `FIXED_NOW_SECS: i64 = 1_787_690_916`,
  `Sandbox::{write_config(&self, extra: &str), add_recording(&self, name: &str, bytes: &[u8]) -> PathBuf, ledger(&self) -> rusqlite::Connection}`.
  Every `Sandbox::vpt()` sets `VPT_TEST_NOW_SECS` to that constant, and recording mtime is
  `FIXED_NOW_SECS - 60`. Task 32's `set_mtime` subtracts its age from the same constant.

- [ ] **Step 1: Write the failing tests**

Add `tempfile = "3.27.0"` under the existing `[dev-dependencies]` in `crates/vpt/Cargo.toml`. Add
`pub(crate) mod ingest;` to `commands/mod.rs` and create `commands/ingest.rs` empty. Integration files
under `tests/` are auto-discovered by Cargo. The compose unit registration and test source below are part
of this step, before either red command.

`crates/vpt/tests/ingest.rs`:

```rust
mod support;

use support::{Sandbox, run, stderr, stdout};
use vpt_domain::fixtures::m4a;

const CAPTURED: i64 = 1_787_690_856;

#[test]
fn ingest_archives_a_recording_and_prints_the_result_document() {
    let sandbox = Sandbox::new("ingest-json");
    sandbox.install_fake_helper();
    sandbox.write_config("");
    sandbox.add_recording("a.m4a", &m4a(CAPTURED, 12, b"audio"));

    let output = run(sandbox.vpt().args(["ingest", "--json"]));

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let document: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("json");
    assert_eq!(document["schema"], "vpt.result/1");
    assert_eq!(document["command"], "ingest");
    assert_eq!(document["ingested"].as_array().map(Vec::len), Some(1));
    assert_eq!(document["ingested"][0]["duration_secs"], 12);
    assert_eq!(document["deferred"], serde_json::json!([]));
    assert_eq!(document["skipped"], 0);
    let id = document["ingested"][0]["id"].as_str().expect("id");
    assert!(sandbox.path().join(format!("home/.vpt/audio/{id}.m4a")).exists());
    assert!(sandbox.path().join("state/vpt/write.lock").exists());
}

#[test]
fn a_second_ingest_skips_and_the_human_output_says_so() {
    let sandbox = Sandbox::new("ingest-again");
    sandbox.install_fake_helper();
    sandbox.write_config("");
    sandbox.add_recording("a.m4a", &m4a(CAPTURED, 1, b"audio"));
    run(sandbox.vpt().arg("ingest"));

    let output = run(sandbox.vpt().arg("ingest"));

    assert_eq!(output.status.code(), Some(0));
    assert!(stdout(&output).contains("skipped 1"), "{}", stdout(&output));
}

#[test]
fn a_missing_config_is_a_config_error_that_names_setup() {
    let sandbox = Sandbox::new("ingest-no-config");

    let output = run(sandbox.vpt().args(["ingest", "--json"]));

    assert_eq!(output.status.code(), Some(2));
    let document: serde_json::Value = serde_json::from_str(&stderr(&output)).expect("json");
    assert_eq!(document["error"]["kind"], "config");
    assert!(document["error"]["message"].as_str().expect("message").contains("vpt setup"));
}

#[test]
fn a_missing_audio_store_parent_is_refused_at_startup_with_the_key_named() {
    let sandbox = Sandbox::new("ingest-bad-store");
    sandbox.install_fake_helper();
    sandbox.write_config(&format!("[stores]\naudio = \"{}\"\n", sandbox.path().join("nowhere/audio").display()));

    let output = run(sandbox.vpt().args(["ingest", "--json"]));

    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("stores.audio"), "{}", stderr(&output));
}

#[test]
fn a_completed_publication_left_dirty_is_repaired_before_the_sweep() {
    let sandbox = Sandbox::new("ingest-repair-done");
    sandbox.install_fake_helper();
    sandbox.write_config("");
    run(sandbox.vpt().arg("ingest"));
    let target = sandbox.path().join("home/.vpt/transcripts/a.md");
    std::fs::write(&target, b"published").expect("target");
    let mut file = std::fs::File::open(&target).expect("target open");
    let digest = vpt_adapters::digest_open(&mut file).expect("digest").hex();
    sandbox
        .ledger()
        .execute(
            "INSERT INTO dirty_publications (target, expected_previous, intended, recorded_at) VALUES (?1, NULL, ?2, 1)",
            [target.to_string_lossy().as_ref(), digest.as_str()],
        )
        .expect("seed");

    let output = run(sandbox.vpt().arg("ingest"));

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let pending: i64 = sandbox.ledger().query_row("SELECT COUNT(*) FROM dirty_publications", [], |r| r.get(0)).expect("count");
    assert_eq!(pending, 0);
}

#[test]
fn a_modified_target_refuses_every_mutating_command_with_target_modified() {
    let sandbox = Sandbox::new("ingest-repair-modified");
    sandbox.install_fake_helper();
    sandbox.write_config("");
    run(sandbox.vpt().arg("ingest"));
    let target = sandbox.path().join("home/.vpt/transcripts/a.md");
    std::fs::write(&target, b"someone else's prose").expect("target");
    sandbox
        .ledger()
        .execute(
            "INSERT INTO dirty_publications (target, expected_previous, intended, recorded_at) VALUES (?1, ?2, ?3, 1)",
            [target.to_string_lossy().as_ref(), &"ab".repeat(32), &"cd".repeat(32)],
        )
        .expect("seed");
    sandbox.add_recording("a.m4a", &m4a(CAPTURED, 1, b"audio"));

    let output = run(sandbox.vpt().args(["ingest", "--json"]));

    assert_eq!(output.status.code(), Some(3));
    let document: serde_json::Value = serde_json::from_str(&stderr(&output)).expect("json");
    assert_eq!(document["error"]["rule"], "target_modified");
    assert!(std::fs::read_dir(sandbox.path().join("home/.vpt/audio")).expect("audio").next().is_none(), "nothing was ingested");
}

#[test]
fn dry_run_creates_no_state_directory_and_no_title_copy() {
    let sandbox = Sandbox::new("ingest-dry");
    sandbox.install_fake_helper();
    sandbox.write_config("");
    sandbox.add_recording("a.m4a", &m4a(CAPTURED, 1, b"audio"));

    let output = run(sandbox.vpt().args(["ingest", "--dry-run", "--json"]));

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let document: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("json");
    assert_eq!(document["would_ingest"].as_array().map(Vec::len), Some(1));
    assert!(!sandbox.path().join("state/vpt").exists());
    assert!(!sandbox.path().join("state/vpt/title-copy").exists());
    assert!(std::fs::read_dir(sandbox.path().join("home/.vpt/audio")).map(|mut d| d.next().is_none()).unwrap_or(true));
}

#[test]
fn dry_run_once_only_proposes_the_selected_file() {
    let sandbox = Sandbox::new("ingest-dry-once");
    sandbox.write_config("");
    let selected = sandbox.add_recording("a.m4a", &m4a(CAPTURED, 1, b"a"));
    sandbox.add_recording("b.m4a", &m4a(CAPTURED, 1, b"b"));
    let output = run(sandbox.vpt().args(["ingest", "--dry-run", "--once"])
        .arg(&selected).arg("--json"));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let document: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("json");
    let proposed = document["would_ingest"].as_array().expect("proposals");
    assert_eq!(proposed.len(), 1);
    assert_eq!(proposed[0]["path"], selected.to_string_lossy().as_ref());
    assert!(!sandbox.path().join("state/vpt").exists());
}

#[test]
fn dry_run_root_and_source_failures_never_send_a_notification() {
    for bad_root in [false, true] {
        let sandbox = Sandbox::new(if bad_root { "dry-invalid-root" } else { "dry-unreadable-source" });
        sandbox.install_fake_helper();
        let extra = format!("[stores]\naudio = {:?}\n", sandbox.path().join("missing/audio"));
        sandbox.write_config(if bad_root { &extra } else { "" });
        let config = std::fs::read_to_string(sandbox.config_path()).expect("config").replace(
            "mode = \"off\"", "mode = \"desktop\"",
        );
        std::fs::write(sandbox.config_path(), config).expect("desktop");
        use std::os::unix::fs::PermissionsExt;
        let source = sandbox.path().join("voice-memos/Recordings");
        if !bad_root {
            std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o000)).expect("unreadable");
        }
        let output = run(sandbox.vpt().args(["ingest", "--dry-run", "--json"])
            .env("VPT_FAKE_LOG", sandbox.fake_log()));
        if !bad_root {
            std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o700)).expect("restore fixture");
        }
        assert!(!output.status.success());
        assert!(!sandbox.fake_log().exists());
        assert!(!sandbox.path().join("state/vpt").exists());
    }
}

#[test]
fn unreadable_source_startup_reports_one_ingest_failure_and_dry_run_reports_none() {
    use std::os::unix::fs::PermissionsExt;
    for (ancestor, dry_run) in [(false, false), (true, false), (false, true), (true, true)] {
        let sandbox = Sandbox::new("source-startup-failure");
        sandbox.install_fake_helper();
        sandbox.write_config("");
        let helper = sandbox.path().join("bin/vpt-macos");
        let config = std::fs::read_to_string(sandbox.config_path()).expect("config").replace(
            "mode = \"off\"",
            &format!("mode = \"command\"\ncommand = [{helper:?}, \"command-sink\", \"{{event}}\"]"),
        );
        std::fs::write(sandbox.config_path(), config).expect("command notifier");
        let denied = sandbox.path().join(if ancestor { "voice-memos" } else { "voice-memos/Recordings" });
        std::fs::set_permissions(&denied, std::fs::Permissions::from_mode(0o000)).expect("deny source");
        let mut command = sandbox.vpt();
        command.args(["ingest", "--json"]);
        if dry_run { command.arg("--dry-run"); }
        let output = run(&mut command);
        std::fs::set_permissions(&denied, std::fs::Permissions::from_mode(0o700)).expect("restore fixture");
        assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
        assert!(stdout(&output).is_empty());
        let error: serde_json::Value = serde_json::from_str(&stderr(&output)).expect("one error");
        assert_eq!(error["error"]["kind"], "store");
        let log = std::fs::read_to_string(sandbox.fake_log()).unwrap_or_default();
        if dry_run {
            assert!(log.is_empty(), "{log}");
        } else {
            let lines: Vec<_> = log.lines().collect();
            assert_eq!(lines.len(), 1, "{log}");
            assert!(lines[0].starts_with("command-sink\tingest_failed\t"), "{log}");
        }
        assert!(!sandbox.path().join("state/vpt").exists());
    }
}

#[test]
fn a_configuration_leaf_link_is_a_path_escape_at_the_command_boundary() {
    let sandbox = Sandbox::new("config-leaf-link");
    sandbox.write_config("");
    let selected = sandbox.config_path();
    let held = selected.with_file_name("held.toml");
    std::fs::rename(&selected, &held).expect("hold config");
    let before = std::fs::read(&held).expect("bytes");
    std::os::unix::fs::symlink(&held, &selected).expect("config link");
    let output = run(sandbox.vpt().args(["ingest", "--json"]));
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    let error: serde_json::Value = serde_json::from_str(&stderr(&output)).expect("error");
    assert_eq!(error["error"]["rule"], "path_escape");
    assert_eq!(std::fs::read(held).expect("unchanged"), before);
    assert!(!sandbox.path().join("state/vpt").exists());
}

```

Support additions in `crates/vpt/tests/support/mod.rs`:

```rust
pub const FIXED_NOW_SECS: i64 = 1_787_690_916;

impl Sandbox {
    pub fn write_config(&self, extra: &str) {
        let config = format!(
            "config_version = 1\n[home]\npath = \"{home}\"\nstate_dir = \"{state}\"\n[helper]\npath = \"{helper}\"\n\
             [source]\nrecordings_dir = \"{recordings}\"\n[notify]\nmode = \"off\"\n{extra}",
            home = self.root.join("home/.vpt").display(),
            state = self.root.join("state/vpt").display(),
            helper = self.root.join("bin/vpt-macos").display(),
            recordings = self.root.join("voice-memos/Recordings").display(),
        );
        std::fs::create_dir_all(self.config_path().parent().expect("dir")).expect("config dir");
        std::fs::write(self.config_path(), config).expect("config");
        std::fs::create_dir_all(self.root.join("home/.vpt")).expect("home");
    }

    pub fn add_recording(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.root.join("voice-memos/Recordings").join(name);
        std::fs::write(&path, bytes).expect("recording");
        let file = std::fs::File::options().write(true).open(&path).expect("open");
        let instant = std::time::UNIX_EPOCH + std::time::Duration::from_secs(u64::try_from(FIXED_NOW_SECS - 60).expect("fixture epoch"));
        file.set_times(std::fs::FileTimes::new().set_modified(instant)).expect("mtime");
        path
    }

    pub fn ledger(&self) -> rusqlite::Connection {
        let connection = rusqlite::Connection::open(self.root.join("state/vpt/vpt.db")).expect("ledger");
        let mut enabled: std::ffi::c_int = 1;
        // SAFETY: the connection and integer outlive this synchronous call.
        let result = unsafe {
            rusqlite::ffi::sqlite3_file_control(
                connection.handle(), c"main".as_ptr(), rusqlite::ffi::SQLITE_FCNTL_PERSIST_WAL,
                (&mut enabled as *mut std::ffi::c_int).cast(),
            )
        };
        assert_eq!(result, rusqlite::ffi::SQLITE_OK, "preserve fixture sidecars");
        connection
    }
}
```

Add this environment entry to the existing `Sandbox::vpt()` chain in Step 1:

```rust
            .env("VPT_TEST_NOW_SECS", FIXED_NOW_SECS.to_string())
```

Register `mod clock;` with its `SystemClock` re-export in the adapter root before the red run. Create
`compose/clock.rs`, `compose/errors.rs`, `compose/ledger.rs` and `compose/runtime.rs` empty, add their
private `mod` declarations in `compose.rs`, and add `#[cfg(test)] mod tests;` at the end of `compose.rs`.
The source modules start with the tests and imports below; their missing production symbols make this a
compiled red.

`crates/vpt/src/compose/tests.rs`:

```rust
use super::*;
use std::os::unix::fs::PermissionsExt;
use vpt_adapters::{LockError, RootDir};
use vpt_application::ports::Clock;

fn configured() -> (tempfile::TempDir, Environment) {
    let temp = tempfile::tempdir().expect("temp");
    let root = temp.path().canonicalize().expect("canonical");
    for directory in ["config", "state", "voice-memos/Recordings", "home"] {
        std::fs::create_dir_all(root.join(directory)).expect("directory");
    }
    let config = root.join("config/config.toml");
    std::fs::write(&config, format!(
        "config_version = 1\n[home]\npath = {:?}\nstate_dir = {:?}\n[source]\nrecordings_dir = {:?}\n[notify]\nmode = \"off\"\n",
        root.join("home/vpt"), root.join("state/vpt"), root.join("voice-memos/Recordings"),
    )).expect("config");
    let environment = Environment { vars: vec![
        ("HOME".into(), root.join("home").display().to_string()),
        ("VPT_CONFIG".into(), config.display().to_string()),
        ("VPT_TEST_NOW_SECS".into(), "1787690916".into()),
    ] };
    (temp, environment)
}

#[test]
fn read_only_composition_creates_no_state_or_store_leaf() {
    let (_temp, environment) = configured();
    let runtime = Runtime::load(&environment, None, AccessMode::ReadOnly, Operation::Ingest).expect("read only");
    assert!(!runtime.roots.state_dir.exists());
    assert!(!runtime.roots.home.exists());
    assert!(runtime._lock.is_none());
}

#[test]
fn lock_contention_precedes_database_open_mode_changes_and_store_creation() {
    let (_temp, environment) = configured();
    let settings = settings_from(&environment, None).expect("settings");
    std::fs::create_dir(&settings.state_dir).expect("state");
    let state = RootDir::open(&settings.state_dir).expect("state root");
    let _held = WriteLock::acquire(&state, Duration::ZERO).expect("held");
    let database = settings.state_dir.join("vpt.db");
    std::fs::write(&database, b"not yet opened").expect("database");
    std::fs::set_permissions(&database, std::fs::Permissions::from_mode(0o644)).expect("mode");
    let outcome = Runtime::load_with_wait(&environment, None, AccessMode::Mutating, Operation::Ingest, Duration::ZERO);
    assert_eq!(outcome.err().expect("locked").exit_code(), 1);
    assert_eq!(std::fs::read(&database).expect("same bytes"), b"not yet opened");
    assert_eq!(std::fs::metadata(&database).expect("metadata").permissions().mode() & 0o777, 0o644);
    assert!(!settings.home.exists());
}

#[test]
fn mutating_composition_retains_the_lock_until_runtime_is_dropped() {
    let (_temp, environment) = configured();
    let runtime = Runtime::load(&environment, None, AccessMode::Mutating, Operation::Ingest).expect("mutable");
    let state = RootDir::open(&runtime.roots.state_dir).expect("state");
    assert!(matches!(WriteLock::acquire(&state, Duration::ZERO), Err(LockError::Busy)));
    drop(runtime);
    assert!(WriteLock::acquire(&state, Duration::ZERO).is_ok());
}

#[cfg(feature = "dev-tools")]
#[test]
fn dev_tools_clock_is_fixed_and_utc() {
    let (_temp, environment) = configured();
    let clock = RuntimeClock::from_environment(&environment).expect("clock");
    assert_eq!(clock.now().secs, 1_787_690_916);
    assert_eq!(clock.offset_at(clock.now()).secs, 0);
}

#[cfg(not(feature = "dev-tools"))]
#[test]
fn production_clock_does_not_read_the_test_environment_value() {
    let environment = Environment { vars: vec![("VPT_TEST_NOW_SECS".into(), "invalid".into())] };
    assert!(RuntimeClock::from_environment(&environment).is_ok());
}

#[test]
fn configuration_and_ledger_path_escapes_keep_the_refusal_rule() {
    use vpt_adapters::config::ConfigError;
    use vpt_application::ports::LedgerError;
    for error in [
        errors::config_error(&ConfigError::PathEscape("/config/config.toml".into()), Path::new("/config/config.toml")),
        errors::ledger_error(LedgerError::PathEscape(PathBuf::from("/state/vpt.db"))),
    ] {
        assert_eq!(error.exit_code(), 3);
        assert_eq!(error.rule.as_deref(), Some("path_escape"));
    }
}

#[test]
fn retention_composition_does_not_open_the_recording_source() {
    let (_temp, environment) = configured();
    let settings = settings_from(&environment, None).expect("settings");
    std::fs::rename(&settings.source.recordings_dir, settings.source.recordings_dir.with_file_name("held")).expect("hide source");
    let runtime = Runtime::load(&environment, None, AccessMode::ReadOnly, Operation::Retention).expect("retention");
    assert!(runtime.recorder.is_none());
    assert!(!runtime.roots.state_dir.exists());
}

#[test]
fn locked_runtime_refuses_replacement_state_for_ledger_and_title_refresh() {
    use vpt_application::ports::{LedgerError, RecorderError, RecorderStore, RecordingLedger};
    let (_temp, environment) = configured();
    let runtime = Runtime::load(&environment, None, AccessMode::Mutating, Operation::Ingest).expect("runtime");
    let state = &runtime.roots.state_dir;
    let held = state.with_file_name("held-state");
    std::fs::rename(state, &held).expect("hold original state");
    std::fs::create_dir(state).expect("replacement state");
    assert!(matches!(runtime.ledger.recordings(), Err(LedgerError::PathEscape(path)) if path == *state));
    let recorder = runtime.recorder.as_ref().expect("source");
    assert!(matches!(recorder.refresh_titles(), Err(RecorderError::Escape(path)) if path == *state));
    assert!(std::fs::read_dir(state).expect("replacement").next().is_none());
    let original = RootDir::open(&held).expect("original state");
    assert!(matches!(WriteLock::acquire(&original, Duration::ZERO), Err(LockError::Busy)));
}

```

- [ ] **Step 2: Run the tests to verify they fail**

Run separately: `cargo test -p vpt --features dev-tools --lib compose` and
`cargo test -p vpt --features dev-tools --test ingest`.

Expected: the compiled compose tests fail on missing implementation, and ingest fails with exit 2 and
"verb not implemented yet". Zero selected tests or a successful command does not satisfy this step.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-adapters/src/clock.rs`:

```rust
//! The system clock, and the local offset at an instant through `localtime_r`.

use vpt_application::ports::Clock;
use vpt_domain::time::{UtcInstant, UtcOffset};

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> UtcInstant {
        let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
        UtcInstant { secs }
    }

    fn offset_at(&self, at: UtcInstant) -> UtcOffset {
        let time: libc::time_t = at.secs as libc::time_t;
        let mut civil: libc::tm = unsafe { std::mem::zeroed() };
        // SAFETY: `time` and `civil` are valid for the call; localtime_r writes only into `civil`.
        let filled = unsafe { libc::localtime_r(&time, &mut civil) };
        if filled.is_null() {
            return UtcOffset { secs: 0 };
        }
        UtcOffset { secs: civil.tm_gmtoff as i32 }
    }
}
```

`crates/vpt/src/compose.rs` (full replacement):

```rust
mod clock;
pub(crate) mod errors;
mod ledger;
mod runtime;

pub(crate) use clock::RuntimeClock;
use errors::config_error;
pub(crate) use ledger::RuntimeLedger;

use std::path::{Path, PathBuf};
use std::time::Duration;
use vpt_adapters::{
    ClonefileArchive, CommandNotifier, DesktopNotifier, FilesystemStores, HelperClient,
    OffNotifier, Roots, VoiceMemosStore, WriteLock,
};
use vpt_adapters::config::{config_path, default_state_dir, from_table, load_file};
use vpt_application::ports::Notifier;
use vpt_application::{NotifyMode, Settings};
use vpt_protocol::error::ErrorDocument;

pub(crate) const WRITE_LOCK_WAIT: Duration = Duration::from_secs(5);

pub(crate) struct Environment {
    vars: Vec<(String, String)>,
}

impl Environment {
    pub(crate) fn from_process() -> Self {
        Self { vars: std::env::vars().collect() }
    }

    pub(crate) fn var(&self, name: &str) -> Option<String> {
        self.vars.iter().find(|(key, _)| key == name).map(|(_, value)| value.clone())
    }

    pub(crate) fn config_path(&self) -> PathBuf {
        config_path(|name| self.var(name))
    }

    pub(crate) fn selected_config(&self, config: Option<&Path>) -> PathBuf {
        config.map(Path::to_path_buf).unwrap_or_else(|| self.config_path())
    }

    pub(crate) fn home_dir(&self) -> PathBuf {
        PathBuf::from(self.var("HOME").unwrap_or_default())
    }

    pub(crate) fn state_dir_default(&self) -> String {
        default_state_dir(|name| self.var(name))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum AccessMode {
    ReadOnly,
    Mutating,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Operation {
    Ingest,
    Retention,
}

pub(crate) struct Runtime {
    pub(crate) settings: Settings,
    pub(crate) roots: Roots,
    pub(crate) ledger: RuntimeLedger,
    pub(crate) recorder: Option<VoiceMemosStore>,
    pub(crate) archive: ClonefileArchive,
    pub(crate) stores: FilesystemStores,
    pub(crate) clock: RuntimeClock,
    pub(crate) helper: HelperClient,
    pub(crate) notifier: Box<dyn Notifier>,
    _lock: Option<WriteLock>,
}

pub(crate) fn settings_from(environment: &Environment, config: Option<&Path>) -> Result<Settings, Box<ErrorDocument>> {
    let path = environment.selected_config(config);
    let table = load_file(&path).map_err(|error| config_error(&error, &path))?;
    from_table(&table, &environment.home_dir()).map_err(|error| Box::new(config_error(&error, &path)))
}

pub(crate) fn notifier_for(settings: &Settings) -> Box<dyn Notifier> {
    let desktop = DesktopNotifier::new(HelperClient::new(settings.helper_path.clone()));
    match &settings.notify.mode {
        NotifyMode::Desktop => Box::new(desktop),
        NotifyMode::Command(argv) => Box::new(CommandNotifier::new(argv.clone(), desktop)),
        NotifyMode::Off => Box::new(OffNotifier),
    }
}


#[cfg(test)]
mod tests;

```

`crates/vpt/src/compose/runtime.rs`:

```rust
use super::{AccessMode, Environment, Operation, Runtime, RuntimeClock, RuntimeLedger, WRITE_LOCK_WAIT, notifier_for, settings_from};
use super::errors::{contained_error, open_error, repair_error, root_error};
use std::path::Path;
use std::time::Duration;
use vpt_adapters::{ClonefileArchive, FilesystemStores, HelperClient, LockError, RootDir, RootError, SqliteLedger, VoiceMemosStore, WriteLock};
use vpt_adapters::config::resolve;
use vpt_application::ports::{Clock, Trash};
use vpt_application::{RepairReport, repair_publications};
use vpt_domain::notification::{EventKind, Notification};
use vpt_protocol::error::{ErrorDocument, ErrorKind};

impl Runtime {
    pub(crate) fn load(environment: &Environment, config: Option<&Path>, access: AccessMode, operation: Operation) -> Result<Self, Box<ErrorDocument>> {
        Self::load_with_wait(environment, config, access, operation, WRITE_LOCK_WAIT)
    }

    pub(super) fn load_with_wait(environment: &Environment, config: Option<&Path>, access: AccessMode, operation: Operation, wait: Duration) -> Result<Self, Box<ErrorDocument>> {
        let settings = settings_from(environment, config)?;
        let notifier = notifier_for(&settings);
        let clock = RuntimeClock::from_environment(environment)?;
        let config_path = environment.selected_config(config);
        let config_dir = config_path.parent().filter(|path| !path.as_os_str().is_empty()).unwrap_or(Path::new("."));
        let roots = resolve(&settings, config_dir).map_err(|error| {
            let source_io = matches!(&error, RootError::Io { key, .. } if key == "source.recordings_dir");
            let ingest_failure = operation == Operation::Ingest && source_io;
            let mut document = if ingest_failure {
                ErrorDocument::new(ErrorKind::Store, format!("{error:?}"))
            } else {
                root_error(&error)
            };
            if access == AccessMode::Mutating {
                let _ = notifier.deliver(&Notification::failed(
                    if ingest_failure { EventKind::IngestFailed } else { EventKind::ConfigRefused },
                    document.message.clone(), clock.now(),
                ));
                document = document.diagnostics(notifier.take_diagnostics());
            }
            document
        })?;
        let mut recorder = if operation == Operation::Retention {
            None
        } else {
            Some(VoiceMemosStore::open(&roots.recordings_dir).map_err(contained_error).map_err(|mut document| {
                if access == AccessMode::Mutating {
                    let _ = notifier.deliver(&Notification::failed(
                        EventKind::IngestFailed, document.message.clone(), clock.now(),
                    ));
                    document = document.diagnostics(notifier.take_diagnostics());
                }
                document
            })?)
        };
        let (lock, ledger, state) = if access == AccessMode::Mutating {
            roots.create_state_dir().map_err(|error| root_error(&error))?;
            let state = RootDir::open(&roots.state_dir).map_err(contained_error)?;
            let lock = WriteLock::acquire(&state, wait).map_err(|error| match error {
                LockError::Busy => ErrorDocument::new(ErrorKind::Ledger, "another vpt command holds the write lock"),
                LockError::Contained(error) => contained_error(error),
                LockError::Io(detail) => ErrorDocument::new(ErrorKind::Ledger, detail),
            })?;
            let ledger = SqliteLedger::open(&state).map_err(open_error)?;
            (Some(lock), RuntimeLedger::Sqlite(ledger), Some(state))
        } else {
            (None, RuntimeLedger::read_only(&roots.state_dir)?, None)
        };
        if lock.is_some() {
            roots.create_leaves().map_err(|error| root_error(&error))?;
        }
        if settings.source.read_titles {
            recorder = match (recorder, state.as_ref()) {
                (Some(store), Some(state)) => Some(store.with_titles_root(state, true)),
                (recorder, _) => recorder,
            };
        }
        let paths = vpt_domain::layout::StoreKey::all().map(|key| roots.stores.get(key).to_path_buf());
        let archive = ClonefileArchive::open_read_only(&roots.stores.audio).map_err(contained_error)?;
        let stores = FilesystemStores::open_read_only(&paths).map_err(contained_error)?;
        Ok(Self {
            helper: HelperClient::new(settings.helper_path.clone()),
            settings, roots, ledger, recorder, archive, stores, clock, notifier, _lock: lock,
        })
    }

    pub(crate) fn mutating(&self) -> Result<RepairReport, Box<ErrorDocument>> {
        if self._lock.is_none() {
            return Err(Box::new(ErrorDocument::new(ErrorKind::Ledger, "mutating startup requires the write lock")));
        }
        repair_publications(&self.ledger, &self.stores, |_| None).map_err(|error| Box::new(repair_error(error)))
    }

    pub(crate) fn trash(&self) -> &dyn Trash {
        &self.helper
    }

    pub(crate) fn diagnostics(&self) -> Vec<String> {
        let mut diagnostics = self.notifier.take_diagnostics();
        diagnostics.extend(self.helper.take_diagnostics());
        diagnostics
    }
}

```

`crates/vpt/src/compose/errors.rs`:

```rust
use std::path::Path;
use vpt_adapters::{ContainedError, OpenError, RootError};
use vpt_adapters::config::ConfigError;
use vpt_application::RepairError;
use vpt_application::ports::{LedgerError, StoreError};
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(crate) fn config_error(error: &ConfigError, path: &Path) -> ErrorDocument {
    let message = match error {
        ConfigError::PathEscape(path) => return ErrorDocument::new(
            ErrorKind::Refused, format!("{path} escapes the configuration root"),
        ).rule("path_escape"),
        ConfigError::Missing(_) => format!("no configuration at {}; run `vpt setup`", path.display()),
        ConfigError::Unreadable { detail, .. } => format!("configuration unreadable: {detail}"),
        ConfigError::Unparseable(_) => "configuration does not parse".into(),
        ConfigError::MissingVersion => "config_version is missing".into(),
        ConfigError::UnsupportedVersion(found) => format!("config_version {found} is not supported (this build reads 1)"),
        ConfigError::UnknownKey(key) => format!("unknown key {key}"),
        ConfigError::WrongType { key, expected } => format!("{key} must be a {expected}"),
        ConfigError::OutOfRange { key, rule } => format!("{key} must be {rule}"),
    };
    ErrorDocument::new(ErrorKind::Config, message)
}

pub(crate) fn root_error(error: &RootError) -> ErrorDocument {
    let message = match error {
        RootError::NotAbsolute { key } => format!("{key} must be absolute after expansion"),
        RootError::ParentMissing { key } => format!("{key}: the parent directory does not exist"),
        RootError::NotADirectory { key } => format!("{key}: not a directory"),
        RootError::PathEscape { key } => return ErrorDocument::new(ErrorKind::Refused, format!("{key}: path escapes its resolved root")).rule("path_escape"),
        RootError::Overlap { first, second } => format!("{first} overlaps {second}"),
        RootError::Io { key, detail } => format!("{key}: {detail}"),
    };
    ErrorDocument::new(ErrorKind::Config, message)
}

pub(crate) fn contained_error(error: ContainedError) -> ErrorDocument {
    match error {
        ContainedError::Io { path, kind } => ErrorDocument::new(ErrorKind::Store, format!("{kind} at {}", path.display())),
        error => ErrorDocument::new(ErrorKind::Refused, format!("{error:?}")).rule("path_escape"),
    }
}

pub(crate) fn open_error(error: OpenError) -> ErrorDocument {
    match error {
        OpenError::Contained(error) => contained_error(error),
        OpenError::Ledger(error) => ledger_error(error),
    }
}

pub(crate) fn ledger_error(error: LedgerError) -> ErrorDocument {
    match error {
        LedgerError::PathEscape(path) => ErrorDocument::new(
            ErrorKind::Refused, format!("{} escapes the retained state root", path.display()),
        ).rule("path_escape"),
        error => ErrorDocument::new(ErrorKind::Ledger, format!("{error:?}")),
    }
}

pub(crate) fn store_error(error: StoreError) -> ErrorDocument {
    match error {
        StoreError::Escape(path) => ErrorDocument::new(ErrorKind::Refused, format!("{} escapes its root", path.display())).rule("path_escape"),
        StoreError::Io(detail) => ErrorDocument::new(ErrorKind::Store, detail),
    }
}

pub(crate) fn repair_error(error: RepairError) -> ErrorDocument {
    match error {
        RepairError::TargetModified(path) | RepairError::Render(path) | RepairError::RenderedDigestMismatch(path) =>
            ErrorDocument::new(ErrorKind::Refused, format!("{} cannot be repaired safely", path.display())).rule("target_modified"),
        RepairError::Sync { cause, .. } | RepairError::Stores(cause) => store_error(cause),
        RepairError::Ledger(error) => ledger_error(error),
    }
}
```

`crates/vpt/src/compose/clock.rs`:

```rust
use super::Environment;
use vpt_adapters::SystemClock;
use vpt_application::ports::Clock;
use vpt_domain::time::{UtcInstant, UtcOffset};
use vpt_protocol::error::ErrorDocument;
#[cfg(feature = "dev-tools")]
use vpt_protocol::error::ErrorKind;

pub(crate) struct RuntimeClock {
    #[cfg(feature = "dev-tools")]
    fixed: Option<UtcInstant>,
}

impl RuntimeClock {
    pub(crate) fn from_environment(environment: &Environment) -> Result<Self, Box<ErrorDocument>> {
        #[cfg(feature = "dev-tools")]
        {
            let fixed = environment.var("VPT_TEST_NOW_SECS").map(|value| {
                value.parse::<i64>().map(|secs| UtcInstant { secs })
                    .map_err(|_| Box::new(ErrorDocument::new(ErrorKind::Config, "invalid test clock")))
            }).transpose()?;
            Ok(Self { fixed })
        }
        #[cfg(not(feature = "dev-tools"))]
        {
            let _ = environment;
            Ok(Self {})
        }
    }
}

impl Clock for RuntimeClock {
    fn now(&self) -> UtcInstant {
        #[cfg(feature = "dev-tools")]
        if let Some(fixed) = self.fixed {
            return fixed;
        }
        SystemClock.now()
    }

    fn offset_at(&self, at: UtcInstant) -> UtcOffset {
        #[cfg(feature = "dev-tools")]
        if self.fixed.is_some() {
            return UtcOffset { secs: 0 };
        }
        SystemClock.offset_at(at)
    }
}
```

The default build never reads the test variable. The fixed build uses UTC, so timestamps and retention
age assertions are independent of the host timezone.

`crates/vpt/src/compose/ledger.rs`:

```rust
use std::path::Path;
use vpt_adapters::{MemoryLedger, SqliteLedger};
use vpt_application::ports::{
    DirtyPublication, LedgerCommit, LedgerError, PublicationJournal, RecordingLedger,
    RecordingRecord, SeenRow,
};
use vpt_domain::{digest::Sha256Digest, identity::RecordingId, time::UtcInstant};
use vpt_protocol::error::ErrorDocument;

pub(crate) enum RuntimeLedger {
    Sqlite(SqliteLedger),
    Empty(MemoryLedger),
}

impl RuntimeLedger {
    pub(crate) fn read_only(path: &Path) -> Result<Self, Box<ErrorDocument>> {
        Ok(match SqliteLedger::open_read_only(path).map_err(super::errors::open_error)? {
            Some(ledger) => Self::Sqlite(ledger),
            None => Self::Empty(MemoryLedger::new()),
        })
    }

    fn recordings_port(&self) -> &dyn RecordingLedger {
        match self { Self::Sqlite(ledger) => ledger, Self::Empty(ledger) => ledger }
    }

    fn publications_port(&self) -> &dyn PublicationJournal {
        match self { Self::Sqlite(ledger) => ledger, Self::Empty(ledger) => ledger }
    }
}

impl RecordingLedger for RuntimeLedger {
    fn seen(&self, path: &Path) -> Result<Option<SeenRow>, LedgerError> { self.recordings_port().seen(path) }
    fn seen_all(&self) -> Result<Vec<SeenRow>, LedgerError> { self.recordings_port().seen_all() }
    fn record_seen(&self, row: &SeenRow) -> Result<(), LedgerError> { self.recordings_port().record_seen(row) }
    fn by_digest(&self, digest: &Sha256Digest) -> Result<Option<RecordingRecord>, LedgerError> { self.recordings_port().by_digest(digest) }
    fn by_id(&self, id: &RecordingId) -> Result<Option<RecordingRecord>, LedgerError> { self.recordings_port().by_id(id) }
    fn recordings(&self) -> Result<Vec<RecordingRecord>, LedgerError> { self.recordings_port().recordings() }
    fn commit(&self, batch: &LedgerCommit) -> Result<(), LedgerError> { self.recordings_port().commit(batch) }
    fn set_source_path(&self, id: &RecordingId, path: &Path) -> Result<(), LedgerError> { self.recordings_port().set_source_path(id, path) }
    fn set_audio_trashed(&self, id: &RecordingId, at: UtcInstant) -> Result<(), LedgerError> { self.recordings_port().set_audio_trashed(id, at) }
}

impl PublicationJournal for RuntimeLedger {
    fn pending_publications(&self) -> Result<Vec<DirtyPublication>, LedgerError> { self.publications_port().pending_publications() }
    fn clear_publication(&self, target: &Path) -> Result<(), LedgerError> { self.publications_port().clear_publication(target) }
}
```

`crates/vpt/src/documents/mod.rs`: `mod record; pub(crate) use record::record_json;`.
`crates/vpt/src/documents/record.rs`:

```rust
//! A recording's record as `vpt show` and `vpt list` print it.

use serde_json::{Value, json};
use vpt_application::ports::RecordingRecord;

pub fn record_json(record: &RecordingRecord) -> Value {
    json!({
        "id": record.id.as_str(),
        "source_path": record.source_path,
        "digest": record.digest.hex(),
        "captured_at": record.captured_at.rfc3339_with(record.captured_offset),
        "captured_at_utc": record.captured_at.rfc3339(),
        "duration_secs": record.duration_secs,
        "title": record.title,
        "title_source": record.title_source.as_str(),
        "ingested_at": record.ingested_at.rfc3339(),
        "audio_path": record.audio_path,
        "stages": {
            "transcribe": record.stages.transcribe.as_str(),
            "note": record.stages.note.as_str(),
            "synthesis": record.stages.synthesis.as_str(),
        },
        "audio_trashed_at": record.audio_trashed_at.map(|at| at.rfc3339()),
    })
}
```

`crates/vpt/src/commands/ingest.rs`:

```rust
//! `vpt ingest`.

use crate::cli::output::Outcome;
use crate::compose::Runtime;
use crate::documents::record_json;
use serde_json::json;
use std::path::PathBuf;
use vpt_application::{Ingest, IngestError, IngestFailure, IngestReport, Mode};
use vpt_protocol::error::{ErrorDocument, ErrorKind};
use vpt_protocol::result::document;

pub fn run(runtime: &Runtime, dry_run: bool, once: Option<PathBuf>) -> Outcome {
    let Some(recorder) = runtime.recorder.as_ref() else {
        return Outcome::Failure(ErrorDocument::new(ErrorKind::Config, "ingest composition has no recording source"));
    };
    if !dry_run {
        if let Err(error) = runtime.mutating() {
            return Outcome::Failure((*error).diagnostics(runtime.diagnostics()));
        }
    }
    let ingest = Ingest {
        recorder,
        archive: &runtime.archive,
        ledger: &runtime.ledger,
        clock: &runtime.clock,
        trash: runtime.trash(),
        notifier: runtime.notifier.as_ref(),
        settings: &runtime.settings.source,
    };
    let mode = Mode { dry_run, once };
    match ingest.run(&mode) {
        Ok(mut report) => {
            report.log.extend(runtime.diagnostics());
            Outcome::Success { human: human(&report), document: document("ingest", body(&report)) }
        }
        Err(mut error) => {
            error.log.extend(runtime.diagnostics());
            Outcome::Failure(failure(*error))
        }
    }
}

fn body(report: &IngestReport) -> serde_json::Value {
    json!({
        "ingested": report.ingested.iter().map(record_json).collect::<Vec<_>>(),
        "already_ingested": report.already_ingested.iter().map(|id| id.as_str()).collect::<Vec<_>>(),
        "recovered": report.recovered.iter().map(|id| id.as_str()).collect::<Vec<_>>(),
        "deferred": report.deferred.iter().map(|d| json!({"path": d.path, "reason": d.reason.as_str()})).collect::<Vec<_>>(),
        "skipped": report.skipped,
        "would_ingest": report.would_ingest.iter().map(|w| json!({"path": w.path, "title": w.title, "title_source": w.title_source.as_str()})).collect::<Vec<_>>(),
        "log": report.log,
    })
}

fn human(report: &IngestReport) -> String {
    let mut lines = Vec::new();
    for record in &report.ingested {
        lines.push(format!("ingested {}", record.id));
    }
    for would in &report.would_ingest {
        lines.push(format!("would ingest {}", would.path.display()));
    }
    for deferred in &report.deferred {
        lines.push(format!("deferred {} ({})", deferred.path.display(), deferred.reason.as_str()));
    }
    for line in &report.log {
        lines.push(format!("note: {line}"));
    }
    lines.push(format!(
        "ingested {}, already ingested {}, deferred {}, skipped {}",
        report.ingested.len(),
        report.already_ingested.len(),
        report.deferred.len(),
        report.skipped
    ));
    lines.join("\n") + "\n"
}

fn failure(error: IngestError) -> ErrorDocument {
    let completed: Vec<String> = error.completed.iter().map(|id| id.as_str().to_owned()).collect();
    let message = error.failure.message();
    let document = match error.failure {
        IngestFailure::ArchiveCollision { .. } => ErrorDocument::new(ErrorKind::Refused, message).rule("archive_collision"),
        IngestFailure::PathEscape(_) => ErrorDocument::new(ErrorKind::Refused, message).rule("path_escape"),
        IngestFailure::HelperVersion { .. } => ErrorDocument::new(ErrorKind::Refused, message).rule("helper_version"),
        IngestFailure::Ledger(error) => crate::compose::errors::ledger_error(error),
        _ => ErrorDocument::new(ErrorKind::Store, message),
    };
    document.completed(completed).diagnostics(error.log)
}
```

In `crates/vpt/src/lib.rs`, `dispatch` gains:

```rust
        Verb::Ingest { dry_run, once } => {
            let access = if *dry_run { AccessMode::ReadOnly } else { AccessMode::Mutating };
            match Runtime::load(&environment, invocation.config.as_deref(), access, Operation::Ingest) {
                Ok(runtime) => commands::ingest::run(&runtime, *dry_run, once.clone()),
                Err(error) => Outcome::Failure(*error),
            }
        }
```

with `let environment = Environment::from_process();` at the top of `dispatch`, `mod documents;`, and the
imports `use compose::{AccessMode, Environment, Operation, Runtime};`. `commands/mod.rs` lists `ingest`,
`setup`, `version`. Add `mod clock;` and `pub use clock::SystemClock;` to the adapters `lib.rs` in Step
1\.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo fmt --all`

Run: `cargo test --workspace --features dev-tools` and
`cargo test -p vpt --lib compose --no-default-features`.

Expected: all PASS. Also run `cargo clippy --workspace --all-targets --features dev-tools -- -D warnings`
and expect no warnings; `main.rs` stays at three lines and `lib.rs` under 150.

- [ ] **Step 5: Commit**

Run: `cargo fmt --all`

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(cli): vpt ingest with the write lock and repair before work"
```

______________________________________________________________________

### Task 28: `vpt show`, `vpt list` and `vpt storage`

Three read-only verbs over the ledger and the stores. `list --stage <stage>` narrows to the recordings
whose named stage (`transcribe`, `note`, `synthesis`) has not succeeded, which is the work waiting there;
any other stage word is a usage error.

**Files:**

- Modify: `crates/vpt-application/src/ports/stores.rs`
- Create: `crates/vpt-application/src/inventory.rs`
- Modify: `crates/vpt-application/src/ports/mod.rs`, `crates/vpt-application/src/lib.rs`
- Modify: `crates/vpt-adapters/src/stores.rs`
- Create: `crates/vpt/src/commands/show.rs`, `crates/vpt/src/commands/list.rs`,
  `crates/vpt/src/commands/storage.rs`
- Modify: `crates/vpt/src/commands/mod.rs`, `crates/vpt/src/lib.rs`, `crates/vpt/src/compose.rs`
- Create: `crates/vpt/src/compose/observations.rs`
- Test: `crates/vpt/tests/show_list_storage.rs`

**Interfaces:**

- Consumes
  `RecordingLedger::{by_id(&self, id: &RecordingId) -> Result<Option<RecordingRecord>, LedgerError>, recordings(&self) -> Result<Vec<RecordingRecord>, LedgerError>}`;
  `record_json(record: &RecordingRecord) -> serde_json::Value`;
  `settings_from(environment: &Environment, config: Option<&Path>) -> Result<Settings, Box<ErrorDocument>>`;
  `RuntimeLedger::read_only(path: &Path) -> Result<RuntimeLedger, Box<ErrorDocument>>`;
  `planned_directory(path: &Path, key: &str) -> Result<PathBuf, RootError>`;
  `resolve_stores(settings: &Settings) -> Result<StorePaths, RootError>`;
  `StoreKey::{all() -> [StoreKey; 7], key_name(self) -> &'static str}`;
  `FilesystemStores::open_read_only(paths: &[PathBuf]) -> Result<FilesystemStores, ContainedError>`.

- Adds `StoreEntry { pub path: PathBuf, pub size: u64, pub mtime: FileTime }` to the
  `vpt_application::ports` exports and extends the existing `Stores` trait with
  `fn entries(&self, root: &Path) -> Result<Vec<StoreEntry>, StoreError>`. It lists regular files at
  depth one, refuses symbolic links, and treats an absent configured root as empty. Unknown roots are
  `StoreError::Escape`.

- Exports
  `vpt_application::{StoreInventory { pub files: u64, pub bytes: u64, pub oldest: Option<FileTime>, pub newest: Option<FileTime> }, inventory(entries: &[StoreEntry]) -> Option<StoreInventory>}`.

- Produces composition
  `ledger_from(environment: &Environment, config: Option<&Path>) -> Result<RuntimeLedger, Box<ErrorDocument>>`
  and
  `storage_from(environment: &Environment, config: Option<&Path>) -> Result<(StorePaths, FilesystemStores), Box<ErrorDocument>>`.

- Command-private functions: `show::run(ledger: &dyn RecordingLedger, id: &str) -> Outcome`,
  `list::run(ledger: &dyn RecordingLedger, stage: Option<&str>) -> Outcome`,
  `storage::run(paths: &StorePaths, source: &dyn Stores) -> Outcome`.

- Acceptance support: `state_snapshot(sandbox: &Sandbox) -> Vec<(PathBuf, u32, i64, i64, Vec<u8>)>`
  recursively captures sorted state and store paths, permission bits, modification time seconds and
  nanoseconds, and file bytes.

- [ ] **Step 1: Write the failing tests**

Register `mod inventory;` and `pub use inventory::{StoreInventory, inventory};` in application `lib.rs`
now. The file starts as the test module below. The adapter tests belong in the existing private stores
test module. Add `pub(crate) mod show; pub(crate) mod list; pub(crate) mod storage;` in
`commands/mod.rs`, with initially empty command files; Cargo discovers the integration test
automatically. Create `compose/observations.rs` empty and register `mod observations;` plus
`pub(crate) use observations::{ledger_from, storage_from};` in `compose.rs` before the red run.

`crates/vpt-application/src/inventory.rs`, test section:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn entry(size: u64, secs: i64) -> StoreEntry {
        StoreEntry { path: PathBuf::from(format!("/s/{secs}")), size, mtime: FileTime { secs, nanos: 0 } }
    }

    #[test]
    fn an_empty_store_has_no_oldest_or_newest() {
        assert_eq!(inventory(&[]).expect("bounded inventory"), StoreInventory { files: 0, bytes: 0, oldest: None, newest: None });
    }

    #[test]
    fn files_and_bytes_are_summed_and_the_extremes_found() {
        let report = inventory(&[entry(10, 300), entry(5, 100), entry(1, 200)]).expect("bounded inventory");
        assert_eq!(report.files, 3);
        assert_eq!(report.bytes, 16);
        assert_eq!(report.oldest, Some(FileTime { secs: 100, nanos: 0 }));
        assert_eq!(report.newest, Some(FileTime { secs: 300, nanos: 0 }));
    }
    #[test]
    fn an_unrepresentable_total_is_refused() {
        assert_eq!(inventory(&[entry(u64::MAX, 100), entry(1, 200)]), None);
    }

}
```

`crates/vpt-adapters/src/stores.rs`, appended inside its existing test module:

```rust
    #[test]
    fn entries_lists_regular_files_at_depth_one_and_absent_configured_roots() {
        let dir = tempfile::tempdir().expect("dir");
        let root = dir.path().canonicalize().expect("canonical");
        std::fs::write(root.join("a.m4a"), b"aaa").expect("a");
        std::fs::create_dir(root.join("sub")).expect("sub");
        std::fs::write(root.join("sub/b.m4a"), b"b").expect("b");
        let absent = root.join("absent");
        let stores = FilesystemStores::open_read_only(&[root.clone(), absent.clone()]).expect("stores");
        let entries = stores.entries(&root).expect("entries");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, root.join("a.m4a"));
        assert_eq!(entries[0].size, 3);
        assert!(stores.entries(&absent).expect("absent").is_empty());
        assert!(!absent.exists());
    }

    #[test]
    fn entries_refuses_a_symlink_instead_of_following_it() {
        let dir = tempfile::tempdir().expect("dir");
        let root = dir.path().canonicalize().expect("canonical");
        std::fs::write(root.join("a.m4a"), b"aaa").expect("a");
        std::os::unix::fs::symlink(root.join("a.m4a"), root.join("link.m4a")).expect("link");
        let stores = FilesystemStores::open(std::slice::from_ref(&root)).expect("stores");
        assert_eq!(stores.entries(&root), Err(StoreError::Escape(root.join("link.m4a"))));
    }
```

`crates/vpt/tests/show_list_storage.rs`:

```rust
mod support;

use support::{Sandbox, run, stderr, stdout};
use vpt_domain::fixtures::m4a;

const CAPTURED: i64 = 1_787_690_856;

fn ingested(name: &str) -> (Sandbox, String) {
    let sandbox = Sandbox::new(name);
    sandbox.install_fake_helper();
    sandbox.write_config("");
    sandbox.add_recording("a.m4a", &m4a(CAPTURED, 9, b"audio"));
    let output = run(sandbox.vpt().args(["ingest", "--json"]));
    let document: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("json");
    let id = document["ingested"][0]["id"].as_str().expect("id").to_owned();
    (sandbox, id)
}

#[test]
fn show_prints_the_record_and_an_unknown_id_is_exit_2() {
    let (sandbox, id) = ingested("show");

    let output = run(sandbox.vpt().args(["show", &id, "--json"]));

    let document: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("json");
    assert_eq!(document["command"], "show");
    assert_eq!(document["id"], id);
    assert_eq!(document["stages"]["transcribe"], "pending");
    let missing = run(sandbox.vpt().args(["show", "2026-08-24T144736-4f3ab19c02de", "--json"]));
    assert_eq!(missing.status.code(), Some(2));
    let error: serde_json::Value = serde_json::from_str(&stderr(&missing)).expect("json");
    assert_eq!(error["error"]["kind"], "usage");
    assert_eq!(error["error"]["ids"][0], "2026-08-24T144736-4f3ab19c02de");
    let malformed = run(sandbox.vpt().args(["show", "nonsense"]));
    assert_eq!(malformed.status.code(), Some(2));
}

#[test]
fn list_prints_every_recording_and_stage_narrows_to_outstanding_work() {
    let (sandbox, id) = ingested("list");

    let all = run(sandbox.vpt().args(["list", "--json"]));
    let document: serde_json::Value = serde_json::from_str(&stdout(&all)).expect("json");
    assert_eq!(document["recordings"][0]["id"], id);

    let waiting = run(sandbox.vpt().args(["list", "--stage", "transcribe", "--json"]));
    let document: serde_json::Value = serde_json::from_str(&stdout(&waiting)).expect("json");
    assert_eq!(document["recordings"].as_array().map(Vec::len), Some(1));
    let unknown = run(sandbox.vpt().args(["list", "--stage", "polish"]));
    assert_eq!(unknown.status.code(), Some(2));
    let human = run(sandbox.vpt().arg("list"));
    assert!(stdout(&human).contains(&id), "{}", stdout(&human));
}

#[test]
fn storage_reports_each_store_with_counts_and_bytes() {
    let (sandbox, _id) = ingested("storage");

    let output = run(sandbox.vpt().args(["storage", "--json"]));

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let document: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("json");
    assert_eq!(document["stores"]["audio"]["files"], 1);
    assert_eq!(document["stores"]["audio"]["bytes"], m4a(CAPTURED, 9, b"audio").len());
    assert!(document["stores"]["audio"]["oldest"].is_string());
    assert_eq!(document["stores"]["released"]["files"], 0);
    assert!(document["stores"]["released"]["oldest"].is_null());
    assert_eq!(document["stores"].as_object().map(|s| s.len()), Some(7));
}

#[test]
fn fresh_observations_do_not_create_state_or_stores() {
    for verb in ["list", "storage", "show"] {
        let sandbox = Sandbox::new(&format!("fresh-{verb}"));
        sandbox.write_config("");
        let mut command = sandbox.vpt();
        command.arg(verb).arg("--json");
        if verb == "show" { command.arg("2026-08-24T144736-4f3ab19c02de"); }
        let output = run(&mut command);
        assert_eq!(output.status.code(), Some(if verb == "show" { 2 } else { 0 }), "{}", stderr(&output));
        assert!(!sandbox.path().join("state/vpt").exists());
        assert!(!sandbox.path().join("home/.vpt/audio").exists());
    }
}

#[test]
fn observations_preserve_dirty_ledger_bytes_modes_and_store_contents() {
    use std::os::unix::fs::PermissionsExt;
    let (sandbox, id) = ingested("read-only-dirty");
    let target = sandbox.path().join("home/.vpt/transcripts/pending.md");
    std::fs::write(&target, b"operator prose").expect("target");
    sandbox.ledger().execute(
        "INSERT INTO dirty_publications(target, expected_previous, intended, recorded_at) VALUES (?1, NULL, ?2, 1)",
        [target.to_string_lossy().as_ref(), &"ab".repeat(32)],
    ).expect("dirty");
    let database = sandbox.path().join("state/vpt/vpt.db");
    std::fs::set_permissions(&database, std::fs::Permissions::from_mode(0o644)).expect("mode");
    let before = support::state_snapshot(&sandbox);
    for args in [vec!["show", &id, "--json"], vec!["list", "--json"], vec!["storage", "--json"]] {
        let output = run(sandbox.vpt().args(args));
        assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
        assert_eq!(support::state_snapshot(&sandbox), before);
    }
}

#[test]
fn show_and_list_need_neither_source_nor_valid_store_roots() {
    let (sandbox, id) = ingested("ledger-only-observations");
    std::fs::rename(sandbox.path().join("voice-memos"), sandbox.path().join("held-source")).expect("hide source");
    sandbox.write_config(&format!("[stores]\naudio = {:?}\n", sandbox.path().join("missing/audio")));
    for args in [vec!["show", &id, "--json"], vec!["list", "--json"]] {
        let output = run(sandbox.vpt().args(args));
        assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
        assert!(stdout(&output).contains(&id));
    }
}

#[test]
fn storage_needs_neither_source_nor_a_readable_ledger() {
    let (sandbox, _) = ingested("stores-only-observation");
    std::fs::rename(sandbox.path().join("voice-memos"), sandbox.path().join("held-source")).expect("hide source");
    std::fs::write(sandbox.path().join("state/vpt/vpt.db"), b"corrupt ledger").expect("corrupt fixture");
    let before = support::state_snapshot(&sandbox);
    let output = run(sandbox.vpt().args(["storage", "--json"]));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let document: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("result");
    assert_eq!(document["stores"]["audio"]["files"], 1);
    assert_eq!(support::state_snapshot(&sandbox), before);
}

#[test]
fn ledger_path_escapes_are_refused_by_show_and_list() {
    let (sandbox, id) = ingested("ledger-path-escape");
    let database = sandbox.path().join("state/vpt/vpt.db");
    let held = sandbox.path().join("held.db");
    std::fs::rename(&database, &held).expect("hold database");
    let before = std::fs::read(&held).expect("database bytes");
    std::os::unix::fs::symlink(&held, &database).expect("database link");
    for args in [vec!["show", &id, "--json"], vec!["list", "--json"]] {
        let output = run(sandbox.vpt().args(args));
        assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
        let error: serde_json::Value = serde_json::from_str(&stderr(&output)).expect("error");
        assert_eq!(error["error"]["rule"], "path_escape");
    }
    assert_eq!(std::fs::read(held).expect("unchanged"), before);
}

```

Append to `crates/vpt/tests/support/mod.rs` in Step 1:

```rust
pub fn state_snapshot(sandbox: &Sandbox) -> Vec<(PathBuf, u32, i64, i64, Vec<u8>)> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    fn visit(path: &Path, rows: &mut Vec<(PathBuf, u32, i64, i64, Vec<u8>)>) {
        let metadata = match path.symlink_metadata() {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
            Err(error) => panic!("snapshot: {error}"),
        };
        assert!(!metadata.file_type().is_symlink(), "snapshot fixture contains a link");
        let bytes = if metadata.is_file() { std::fs::read(path).expect("read") } else { Vec::new() };
        rows.push((path.to_path_buf(), metadata.permissions().mode() & 0o777, metadata.mtime(), metadata.mtime_nsec(), bytes));
        if metadata.is_dir() {
            for entry in std::fs::read_dir(path).expect("directory") {
                visit(&entry.expect("entry").path(), rows);
            }
        }
    }
    let mut rows = Vec::new();
    visit(&sandbox.path().join("state/vpt"), &mut rows);
    visit(&sandbox.path().join("home/.vpt"), &mut rows);
    rows.sort();
    rows
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run separately: `cargo test -p vpt-application inventory`, `cargo test -p vpt-adapters stores`, and
`cargo test -p vpt --features dev-tools --test show_list_storage`.

Expected: compile errors for `inventory` and `entries`; the three command tests FAIL with exit 2 "verb
not implemented yet". The new modules must compile and be selected; zero selected tests or a successful
command does not satisfy this step.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-application/src/ports/stores.rs` retains all existing `StoreError` and `Stores` members. Add
`use vpt_domain::time::FileTime;`, this declaration above `Stores`, and its method:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreEntry {
    pub path: PathBuf,
    pub size: u64,
    pub mtime: FileTime,
}
```

```rust
    fn entries(&self, root: &Path) -> Result<Vec<StoreEntry>, StoreError>;
```

The existing fake `impl Stores` blocks in publication tests gain the following method in Step 1:

```rust
    fn entries(&self, _root: &Path) -> Result<Vec<StoreEntry>, StoreError> {
        Ok(Vec::new())
    }
```

Import `StoreEntry` with `StoreError` in those private test modules.

`crates/vpt-application/src/inventory.rs`:

```rust
//! `vpt storage`: counts, bytes and the mtime extremes of one store.

use crate::ports::StoreEntry;
use vpt_domain::time::FileTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoreInventory {
    pub files: u64,
    pub bytes: u64,
    pub oldest: Option<FileTime>,
    pub newest: Option<FileTime>,
}

pub fn inventory(entries: &[StoreEntry]) -> Option<StoreInventory> {
    let key = |time: &FileTime| (time.secs, time.nanos);
    Some(StoreInventory {
        files: u64::try_from(entries.len()).ok()?,
        bytes: entries.iter().try_fold(0_u64, |total, entry| total.checked_add(entry.size))?,
        oldest: entries.iter().map(|entry| entry.mtime).min_by_key(key),
        newest: entries.iter().map(|entry| entry.mtime).max_by_key(key),
    })
}
```

`ports/mod.rs` adds `StoreEntry` to its existing `pub use stores::{StoreError, Stores};`. Register
`mod inventory;` and `pub use inventory::{StoreInventory, inventory};` in application `lib.rs` during
Step 1, before the red run. Extend the existing adapter `impl Stores`:

```rust
    fn entries(&self, path: &Path) -> Result<Vec<StoreEntry>, StoreError> {
        if !self.configured.iter().any(|configured| configured == path) {
            return Err(StoreError::Escape(path.to_path_buf()));
        }
        let Some(root) = self.roots.iter().find(|root| root.path() == path) else {
            return Ok(Vec::new());
        };
        let mut entries = Vec::new();
        for name in root.names().map_err(|error| store_error(path, error))? {
            let stat = root.stat(Path::new(&name)).map_err(|error| store_error(path, error))?;
            if stat.kind == Kind::Link {
                return Err(StoreError::Escape(path.join(name)));
            }
            if stat.kind != Kind::File {
                continue;
            }
            entries.push(StoreEntry {
                path: root.leaf(Path::new(&name)).map_err(|error| store_error(path, error))?,
                size: stat.size,
                mtime: FileTime { secs: stat.mtime_secs, nanos: stat.mtime_nanos },
            });
        }
        Ok(entries)
    }
```

Add `Kind` to the adapter's contained imports, `StoreEntry` to its port imports and
`use vpt_domain::time::FileTime;`. The retained root supplies every stat and leaf operation.

`crates/vpt/src/commands/show.rs`:

```rust
//! `vpt show <id>`.

use crate::cli::output::Outcome;
use crate::documents::record_json;
use vpt_application::ports::RecordingLedger;
use vpt_domain::identity::RecordingId;
use vpt_protocol::error::{ErrorDocument, ErrorKind};
use vpt_protocol::result::document;

pub fn run(ledger: &dyn RecordingLedger, id: &str) -> Outcome {
    let Ok(identity) = RecordingId::parse(id) else {
        return Outcome::Failure(ErrorDocument::new(ErrorKind::Usage, format!("{id} is not a recording id")));
    };
    match ledger.by_id(&identity) {
        Ok(Some(record)) => {
            let json = record_json(&record);
            Outcome::Success { human: format!("{}\n", serde_json::to_string_pretty(&json).unwrap_or_default()), document: document("show", json) }
        }
        Ok(None) => Outcome::Failure(ErrorDocument::new(ErrorKind::Usage, format!("no recording {id}")).ids(vec![id.to_owned()])),
        Err(error) => Outcome::Failure(crate::compose::errors::ledger_error(error)),
    }
}
```

`crates/vpt/src/commands/list.rs`:

```rust
//! `vpt list [--stage <stage>]`.

use crate::cli::output::Outcome;
use crate::documents::record_json;
use serde_json::json;
use vpt_application::ports::{RecordingLedger, RecordingRecord, StageState};
use vpt_protocol::error::{ErrorDocument, ErrorKind};
use vpt_protocol::result::document;

pub fn run(ledger: &dyn RecordingLedger, stage: Option<&str>) -> Outcome {
    let outstanding: fn(&RecordingRecord) -> StageState = match stage {
        None => |_| StageState::Pending,
        Some("transcribe") => |record| record.stages.transcribe,
        Some("note") => |record| record.stages.note,
        Some("synthesis") => |record| record.stages.synthesis,
        Some(other) => return Outcome::Failure(ErrorDocument::new(ErrorKind::Usage, format!("{other} is not a stage"))),
    };
    let records = match ledger.recordings() {
        Ok(records) => records,
        Err(error) => return Outcome::Failure(crate::compose::errors::ledger_error(error)),
    };
    let selected: Vec<&RecordingRecord> = records.iter().filter(|record| outstanding(record) != StageState::Succeeded).collect();
    let human = selected.iter().map(|record| format!("{}  {}\n", record.id, record.title.as_deref().unwrap_or("(untitled)"))).collect::<String>();
    Outcome::Success {
        human: if human.is_empty() { "no recordings\n".into() } else { human },
        document: document("list", json!({"recordings": selected.iter().map(|record| record_json(record)).collect::<Vec<_>>()})),
    }
}
```

With no `--stage`, every record's selector answers `Pending`, so every record is listed.

`crates/vpt/src/commands/storage.rs`:

```rust
//! `vpt storage`.

use crate::cli::output::Outcome;
use serde_json::{Map, Value, json};
use vpt_application::inventory;
use vpt_application::ports::Stores;
use vpt_domain::layout::StoreKey;
use vpt_domain::time::{FileTime, UtcInstant};
use vpt_protocol::error::{ErrorDocument, ErrorKind};
use vpt_protocol::result::document;

fn stamp(time: Option<FileTime>) -> Value {
    time.map_or(Value::Null, |t| Value::String(UtcInstant { secs: t.secs }.rfc3339()))
}

pub fn run(paths: &vpt_application::StorePaths, source: &dyn Stores) -> Outcome {
    let mut stores = Map::new();
    let mut human = String::new();
    for key in StoreKey::all() {
        let root = paths.get(key);
        let entries = match source.entries(root) {
            Ok(entries) => entries,
            Err(error) => return Outcome::Failure(crate::compose::errors::store_error(error)),
        };
        let Some(report) = inventory(&entries) else {
            return Outcome::Failure(ErrorDocument::new(
                ErrorKind::Store, format!("{} exceeds the supported inventory total", root.display()),
            ));
        };
        human.push_str(&format!("{:<15} {:>6} files {:>12} bytes  {}\n", key.key_name(), report.files, report.bytes, root.display()));
        stores.insert(
            key.key_name().to_owned(),
            json!({"files": report.files, "bytes": report.bytes, "oldest": stamp(report.oldest), "newest": stamp(report.newest), "path": root}),
        );
    }
    Outcome::Success { human, document: document("storage", json!({"stores": stores})) }
}
```

Add `mod observations;` and `pub(crate) use observations::{ledger_from, storage_from};` to `compose.rs`.
The empty observations file and declarations are registered in Step 1, before the red run.

`crates/vpt/src/compose/observations.rs`:

```rust
use super::{Environment, RuntimeLedger, settings_from};
use super::errors::{contained_error, root_error};
use std::path::Path;
use vpt_adapters::FilesystemStores;
use vpt_adapters::config::{planned_directory, resolve_stores};
use vpt_application::StorePaths;
use vpt_domain::layout::StoreKey;
use vpt_protocol::error::ErrorDocument;

pub(crate) fn ledger_from(environment: &Environment, config: Option<&Path>) -> Result<RuntimeLedger, Box<ErrorDocument>> {
    let settings = settings_from(environment, config)?;
    let state = planned_directory(&settings.state_dir, "home.state_dir").map_err(|error| root_error(&error))?;
    RuntimeLedger::read_only(&state)
}

pub(crate) fn storage_from(environment: &Environment, config: Option<&Path>) -> Result<(StorePaths, FilesystemStores), Box<ErrorDocument>> {
    let settings = settings_from(environment, config)?;
    let paths = resolve_stores(&settings).map_err(|error| root_error(&error))?;
    let configured = StoreKey::all().map(|key| paths.get(key).to_path_buf());
    let stores = FilesystemStores::open_read_only(&configured).map_err(contained_error)?;
    Ok((paths, stores))
}
```

The dispatch arms in `crates/vpt/src/lib.rs`:

```rust
        Verb::Show { id } => match compose::ledger_from(&environment, invocation.config.as_deref()) {
            Ok(ledger) => commands::show::run(&ledger, id),
            Err(error) => Outcome::Failure(*error),
        },
        Verb::List { stage } => match compose::ledger_from(&environment, invocation.config.as_deref()) {
            Ok(ledger) => commands::list::run(&ledger, stage.as_deref()),
            Err(error) => Outcome::Failure(*error),
        },
        Verb::Storage => match compose::storage_from(&environment, invocation.config.as_deref()) {
            Ok((paths, stores)) => commands::storage::run(&paths, &stores),
            Err(error) => Outcome::Failure(*error),
        },
```

Retain the Task 27 ingest dispatch unchanged. These observations create only their named capability; show
and list do not resolve or open stores or Voice Memos, and storage never opens the ledger.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo fmt --all`

Run: `cargo test --workspace --features dev-tools`

Expected: all PASS.

- [ ] **Step 5: Commit**

Run: `cargo fmt --all`

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(cli): vpt show, vpt list and vpt storage"
```

______________________________________________________________________

### Task 29: The managed symlink: `vpt symlink deploy` and `vpt symlink verify`

**Files:**

- Create: `crates/vpt-adapters/src/symlink.rs`
- Modify: `crates/vpt-adapters/src/lib.rs`
- Create: `crates/vpt/src/commands/symlink.rs`
- Modify: `crates/vpt/src/commands/mod.rs`, `crates/vpt/src/lib.rs`
- Test: `crates/vpt/tests/symlink.rs`

**Interfaces:**

- Consumes
  `settings_from(environment: &Environment, config: Option<&Path>) -> Result<Settings, Box<ErrorDocument>>`,
  `Settings.home: PathBuf`, `Settings.symlink_target: Option<PathBuf>`,
  `RootDir::open(path: &Path) -> Result<RootDir, ContainedError>`, and
  `RootDir::subdirectory(&self, name: &str) -> Result<RootDir, ContainedError>`.

- Adapter root exports: `LinkState::{Absent, LinkTo(PathBuf), Other(String), Unreadable(String)}`;
  `SymlinkError::{Occupied(String), TargetParentMissing, TargetNotDirectory, TargetMissing, Missing, WrongTarget(PathBuf), Io(String)}`;
  `inspect_symlink(link: &Path) -> LinkState`;
  `deploy_symlink(link: &Path, target: &Path) -> Result<(), SymlinkError>`;
  `verify_symlink(link: &Path, target: &Path) -> Result<(), SymlinkError>`.

- Command-private functions `deploy(environment: &Environment, config: Option<&Path>) -> Outcome` and
  `verify(environment: &Environment, config: Option<&Path>) -> Outcome`.

- [ ] **Step 1: Write the failing tests**

In adapter `lib.rs`, register `mod symlink;` and
`pub use symlink::{LinkState, SymlinkError, deploy as deploy_symlink, inspect as inspect_symlink, verify as verify_symlink};`.
The file starts with the test module. Register `pub(crate) mod symlink;` in `commands/mod.rs` and create
that command file empty.

`crates/vpt-adapters/src/symlink.rs`, test section:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
        let dir = tempfile::tempdir().expect("dir");
        let root = dir.path().canonicalize().expect("canonical");
        std::fs::create_dir(root.join("notes")).expect("parent");
        let link = root.join(".vpt");
        let target = root.join("notes/vpt");
        (dir, link, target)
    }

    #[test]
    fn deploy_creates_a_missing_leaf_and_the_link_and_verify_then_passes() {
        let (_dir, link, target) = setup();
        deploy(&link, &target).expect("deployed");
        assert!(target.is_dir());
        assert_eq!(std::fs::read_link(&link).expect("link"), target);
        assert_eq!(verify(&link, &target), Ok(()));
    }

    #[test]
    fn deploy_is_idempotent_over_its_own_link() {
        let (_dir, link, target) = setup();
        deploy(&link, &target).expect("first");
        assert_eq!(deploy(&link, &target), Ok(()));
    }

    #[test]
    fn deploy_refuses_a_directory_in_the_way_naming_what_is_there() {
        let (_dir, link, target) = setup();
        std::fs::create_dir(&link).expect("dir in the way");
        assert_eq!(deploy(&link, &target), Err(SymlinkError::Occupied("a directory".into())));
    }

    #[test]
    fn deploy_refuses_a_link_to_somewhere_else_and_a_missing_target_parent() {
        let (dir, link, target) = setup();
        std::os::unix::fs::symlink(dir.path().join("elsewhere"), &link).expect("other link");
        assert!(matches!(deploy(&link, &target), Err(SymlinkError::Occupied(_))));
        let orphan = dir.path().join("missing/parent/vpt");
        assert_eq!(deploy(&dir.path().join("other"), &orphan), Err(SymlinkError::TargetParentMissing));
    }

    #[test]
    fn verify_names_a_missing_link_and_a_wrong_target_and_writes_nothing() {
        let (dir, link, target) = setup();
        assert_eq!(verify(&link, &target), Err(SymlinkError::Missing));
        std::os::unix::fs::symlink(dir.path().join("notes"), &link).expect("wrong link");
        assert_eq!(verify(&link, &target), Err(SymlinkError::WrongTarget(dir.path().join("notes"))));
        assert!(!target.exists());
    }

    #[test]
    fn relative_equivalent_links_verify_and_deploy() {
        let (_dir, link, target) = setup();
        std::fs::create_dir(&target).expect("target");
        std::os::unix::fs::symlink("notes/vpt", &link).expect("relative");
        assert_eq!(verify(&link, &target), Ok(()));
        assert_eq!(deploy(&link, &target), Ok(()));
        assert_eq!(std::fs::read_link(&link).expect("preserved"), PathBuf::from("notes/vpt"));
    }

    #[test]
    fn matching_dangling_link_fails_verification_and_deploy_creates_the_leaf() {
        let (_dir, link, target) = setup();
        std::os::unix::fs::symlink("notes/vpt", &link).expect("relative");
        assert_eq!(verify(&link, &target), Err(SymlinkError::TargetMissing));
        assert!(!target.exists());
        assert_eq!(deploy(&link, &target), Ok(()));
        assert!(target.is_dir());
        assert_eq!(verify(&link, &target), Ok(()));
    }

    #[test]
    fn regular_file_target_is_refused_without_replacing_its_bytes() {
        let (_dir, link, target) = setup();
        std::fs::write(&target, b"owned by operator").expect("file");
        std::os::unix::fs::symlink(&target, &link).expect("link");
        assert_eq!(verify(&link, &target), Err(SymlinkError::TargetNotDirectory));
        assert_eq!(deploy(&link, &target), Err(SymlinkError::TargetNotDirectory));
        assert_eq!(std::fs::read(&target).expect("untouched"), b"owned by operator");
    }

}
```

`crates/vpt/tests/symlink.rs`:

```rust
mod support;

use support::{Sandbox, run, stderr, stdout};

fn configured(name: &str) -> (Sandbox, std::path::PathBuf) {
    let sandbox = Sandbox::new(name);
    std::fs::create_dir_all(sandbox.path().join("home/notes")).expect("vault");
    let target = sandbox.path().join("home/notes/vpt");
    let config = format!(
        "config_version = 1\n[home]\nsymlink_target = \"{}\"\nstate_dir = \"{}\"\n[source]\nrecordings_dir = \"{}\"\n",
        target.display(),
        sandbox.path().join("state/vpt").display(),
        sandbox.path().join("voice-memos/Recordings").display()
    );
    std::fs::create_dir_all(sandbox.config_path().parent().expect("dir")).expect("config dir");
    std::fs::write(sandbox.config_path(), config).expect("config");
    (sandbox, target)
}

#[test]
fn deploy_creates_the_link_at_the_default_home_and_verify_passes() {
    let (sandbox, target) = configured("symlink-deploy");

    let output = run(sandbox.vpt().args(["symlink", "deploy", "--json"]));

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let document: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("json");
    assert_eq!(document["link"], sandbox.path().join("home/.vpt").to_string_lossy());
    assert_eq!(document["target"], target.to_string_lossy());
    assert_eq!(document["ok"], true);
    let verify = run(sandbox.vpt().args(["symlink", "verify"]));
    assert_eq!(verify.status.code(), Some(0));
}

#[test]
fn verify_exits_3_naming_the_discrepancy_and_deploy_refuses_an_occupied_path() {
    let (sandbox, _target) = configured("symlink-verify");
    std::fs::create_dir(sandbox.path().join("home/.vpt")).expect("directory in the way");

    let verify = run(sandbox.vpt().args(["symlink", "verify", "--json"]));
    assert_eq!(verify.status.code(), Some(3));
    let error: serde_json::Value = serde_json::from_str(&stderr(&verify)).expect("json");
    assert_eq!(error["error"]["rule"], "symlink_verify");

    let deploy = run(sandbox.vpt().args(["symlink", "deploy"]));
    assert_eq!(deploy.status.code(), Some(3));
    assert!(stderr(&deploy).contains("a directory"), "{}", stderr(&deploy));
}

#[test]
fn without_a_symlink_target_both_verbs_are_usage_errors() {
    let sandbox = Sandbox::new("symlink-unset");
    sandbox.write_config("");

    let output = run(sandbox.vpt().args(["symlink", "verify", "--json"]));

    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("symlink_target"), "{}", stderr(&output));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run separately: `cargo test -p vpt-adapters symlink` and
`cargo test -p vpt --features dev-tools --test symlink`. Expected: compile errors naming `deploy` and
`verify`; the command tests FAIL with exit 2. Both new test modules must be compiled and selected; zero
selected tests or a successful command does not satisfy this step.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-adapters/src/symlink.rs`:

```rust
use std::path::{Path, PathBuf};
use crate::{ContainedError, RootDir};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkState {
    Absent,
    LinkTo(PathBuf),
    Other(String),
    Unreadable(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymlinkError {
    Occupied(String),
    TargetParentMissing,
    TargetNotDirectory,
    TargetMissing,
    Missing,
    WrongTarget(PathBuf),
    Io(String),
}

pub fn inspect(link: &Path) -> LinkState {
    match std::fs::symlink_metadata(link) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => LinkState::Absent,
        Err(error) => LinkState::Unreadable(error.kind().to_string()),
        Ok(metadata) if metadata.file_type().is_symlink() => {
            std::fs::read_link(link).map_or_else(
                |error| LinkState::Unreadable(error.kind().to_string()), LinkState::LinkTo,
            )
        }
        Ok(metadata) if metadata.is_dir() => LinkState::Other("a directory".into()),
        Ok(_) => LinkState::Other("a file".into()),
    }
}

fn destination(link: &Path, value: &Path) -> PathBuf {
    if value.is_absolute() { value.to_path_buf() } else { link.parent().unwrap_or(Path::new(".")).join(value) }
}

fn resolved_directory(path: &Path) -> Result<PathBuf, SymlinkError> {
    let resolved = path.canonicalize().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound { SymlinkError::TargetMissing }
        else { SymlinkError::Io(error.kind().to_string()) }
    })?;
    let metadata = resolved.metadata().map_err(|error| SymlinkError::Io(error.kind().to_string()))?;
    if !metadata.is_dir() {
        return Err(SymlinkError::TargetNotDirectory);
    }
    Ok(resolved)
}

fn ensure_target(target: &Path) -> Result<PathBuf, SymlinkError> {
    match resolved_directory(target) {
        Ok(path) => Ok(path),
        Err(SymlinkError::TargetMissing) => {
            let parent = target.parent().ok_or(SymlinkError::TargetParentMissing)?;
            let parent = resolved_directory(parent).map_err(|_| SymlinkError::TargetParentMissing)?;
            let leaf = target.file_name().and_then(|name| name.to_str()).ok_or(SymlinkError::TargetNotDirectory)?;
            let parent = RootDir::open(&parent).map_err(contained)?;
            parent.subdirectory(leaf).map(|root| root.path().to_path_buf()).map_err(contained)
        }
        Err(error) => Err(error),
    }
}

fn contained(error: ContainedError) -> SymlinkError {
    match error {
        ContainedError::NotADirectory(_) | ContainedError::NotRegular(_) => SymlinkError::TargetNotDirectory,
        error => SymlinkError::Io(format!("{error:?}")),
    }
}

fn prospective(path: &Path) -> Option<PathBuf> {
    if let Ok(path) = path.canonicalize() { return Some(path); }
    Some(path.parent()?.canonicalize().ok()?.join(path.file_name()?))
}

pub fn deploy(link: &Path, target: &Path) -> Result<(), SymlinkError> {
    let existing = match inspect(link) {
        LinkState::LinkTo(value) => {
            let destination = destination(link, &value);
            if prospective(&destination) != prospective(target) || prospective(target).is_none() {
                return Err(SymlinkError::Occupied(format!("a link to {}", value.display())));
            }
            true
        }
        LinkState::Other(what) => return Err(SymlinkError::Occupied(what)),
        LinkState::Unreadable(detail) => return Err(SymlinkError::Io(detail)),
        LinkState::Absent => false,
    };
    let target = ensure_target(target)?;
    if existing {
        return verify(link, &target);
    }
    std::os::unix::fs::symlink(target, link).map_err(|error| SymlinkError::Io(error.kind().to_string()))
}

pub fn verify(link: &Path, target: &Path) -> Result<(), SymlinkError> {
    match inspect(link) {
        LinkState::LinkTo(value) => {
            if prospective(&destination(link, &value)) != prospective(target) {
                return Err(SymlinkError::WrongTarget(value));
            }
            let expected = resolved_directory(target)?;
            let found = resolved_directory(&destination(link, &value))?;
            if found == expected { Ok(()) } else { Err(SymlinkError::WrongTarget(value)) }
        }
        LinkState::Absent => Err(SymlinkError::Missing),
        LinkState::Other(what) => Err(SymlinkError::Occupied(what)),
        LinkState::Unreadable(detail) => Err(SymlinkError::Io(detail)),
    }
}
```

Add `mod symlink;` and
`pub use symlink::{LinkState, SymlinkError, deploy as deploy_symlink, inspect as inspect_symlink, verify as verify_symlink};`
to the adapters `lib.rs` in Step 1. `crates/vpt/src/commands/symlink.rs`:

```rust
//! `vpt symlink deploy` and `vpt symlink verify`.

use crate::cli::output::Outcome;
use crate::compose::{Environment, settings_from};
use serde_json::json;
use std::path::{Path, PathBuf};
use vpt_adapters::{SymlinkError, deploy_symlink as deploy_link, verify_symlink as verify_link};
use vpt_protocol::error::{ErrorDocument, ErrorKind};
use vpt_protocol::result::document;

fn link_and_target(environment: &Environment, config: Option<&Path>) -> Result<(PathBuf, PathBuf), Box<ErrorDocument>> {
    let settings = settings_from(environment, config)?;
    let target = settings.symlink_target.ok_or_else(|| ErrorDocument::new(ErrorKind::Usage, "home.symlink_target is not set"))?;
    if !settings.home.is_absolute() || !target.is_absolute() {
        return Err(Box::new(ErrorDocument::new(ErrorKind::Config, "home.path and home.symlink_target must be absolute")));
    }
    Ok((settings.home, target))
}

fn describe(error: &SymlinkError, link: &Path, target: &Path) -> String {
    match error {
        SymlinkError::Occupied(what) => format!("{} exists and is {what}", link.display()),
        SymlinkError::TargetParentMissing => format!("the parent of {} does not exist", target.display()),
        SymlinkError::TargetNotDirectory => format!("{} is not a directory", target.display()),
        SymlinkError::TargetMissing => format!("{} does not exist", target.display()),
        SymlinkError::Missing => format!("{} does not exist", link.display()),
        SymlinkError::WrongTarget(existing) => format!("{} points at {} rather than {}", link.display(), existing.display(), target.display()),
        SymlinkError::Io(detail) => detail.clone(),
    }
}

fn outcome(verb: &str, rule: &str, result: Result<(), SymlinkError>, link: &Path, target: &Path) -> Outcome {
    match result {
        Ok(()) => Outcome::Success {
            human: format!("{} -> {}\n", link.display(), target.display()),
            document: document(verb, json!({"link": link, "target": target, "ok": true})),
        },
        Err(SymlinkError::Io(detail)) => Outcome::Failure(ErrorDocument::new(ErrorKind::Store, detail)),
        Err(error) => Outcome::Failure(ErrorDocument::new(ErrorKind::Refused, describe(&error, link, target)).rule(rule)),
    }
}

pub fn deploy(environment: &Environment, config: Option<&Path>) -> Outcome {
    match link_and_target(environment, config) {
        Ok((link, target)) => outcome("symlink deploy", "symlink_deploy", deploy_link(&link, &target), &link, &target),
        Err(error) => Outcome::Failure(*error),
    }
}

pub fn verify(environment: &Environment, config: Option<&Path>) -> Outcome {
    match link_and_target(environment, config) {
        Ok((link, target)) => outcome("symlink verify", "symlink_verify", verify_link(&link, &target), &link, &target),
        Err(error) => Outcome::Failure(*error),
    }
}
```

`settings_from` (Task 27) is the only startup step these verbs take: no roots, no ledger, no helper.
Dispatch arms:

```rust
        Verb::SymlinkDeploy => commands::symlink::deploy(&environment, invocation.config.as_deref()),
        Verb::SymlinkVerify => commands::symlink::verify(&environment, invocation.config.as_deref()),
```

and `commands/mod.rs` gains `symlink`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo fmt --all`

Run: `cargo test --workspace --features dev-tools`

Expected: all PASS.

- [ ] **Step 5: Commit**

Run: `cargo fmt --all`

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(symlink): vpt symlink deploy and verify over the default home"
```

______________________________________________________________________

### Task 30: `vpt doctor`

Doctor never refuses at startup: every check runs, the whole list is printed either way, and any failed
check makes the exit code 3 with the array inside a `vpt.error/1` document. The stage 1 checks are
config, stores, one git-tree check per store, helper, recordings directory, symlink, one count per Apple
subdirectory, cleanup pending and source gone.

**Files:**

- Create: `crates/vpt-adapters/src/git_tree.rs`
- Modify: `crates/vpt-adapters/src/lib.rs`
- Modify: `crates/vpt/src/cli/output.rs`
- Create: `crates/vpt/src/doctor/mod.rs`, `crates/vpt/src/doctor/checks.rs`,
  `crates/vpt/src/doctor/probes.rs`
- Modify: `crates/vpt/src/lib.rs`
- Test: `crates/vpt/tests/doctor.rs`

**Interfaces:**

- Consumes
  `settings_from(environment: &Environment, config: Option<&Path>) -> Result<Settings, Box<ErrorDocument>>`;
  `resolve(settings: &Settings, config_dir: &Path) -> Result<Roots, RootError>`;
  `planned_directory(path: &Path, key: &str) -> Result<PathBuf, RootError>`;
  `HelperClient::version(&self) -> Result<String, HelperError>`;
  `VoiceMemosStore::open(path: &Path) -> Result<VoiceMemosStore, ContainedError>`;
  `RecorderStore::candidates(&self) -> Result<Vec<Candidate>, RecorderError>`;
  `RootDir::{open(path: &Path) -> Result<RootDir, ContainedError>, open_subdirectory(&self, name: &str) -> Result<RootDir, ContainedError>, names(&self) -> Result<Vec<String>, ContainedError>}`;
  `ClonefileArchive::open_read_only(path: &Path) -> Result<ClonefileArchive, ContainedError>`;
  `Archive::staged_leftovers(&self) -> Result<Vec<PathBuf>, ArchiveError>`;
  `RuntimeLedger::read_only(path: &Path) -> Result<RuntimeLedger, Box<ErrorDocument>>`;
  `RecordingLedger::seen_all(&self) -> Result<Vec<SeenRow>, LedgerError>`;
  `Check { pub name: String, pub ok: bool, pub detail: String }`;
  `ErrorDocument::{checks(self, checks: Vec<Check>) -> Self, diagnostics(self, diagnostics: Vec<String>) -> Self}`.

- Adapter root export `enclosing_git_tree(path: &Path) -> Option<PathBuf>` returns the nearest ancestor,
  including the path itself, with a `.git` entry of any type.

- `Outcome::FailedReport { error: ErrorDocument, human: String }`;
  `emit(outcome: Outcome, json: bool) -> i32` preserves stdout write failures as exit 1.

- Command-private `doctor::run(environment: &Environment, config: Option<&Path>) -> Outcome`;
  `checks::all(environment: &Environment, config: Option<&Path>) -> Vec<Check>` is `pub(super)` inside
  private `doctor::checks`.

- `INSTALL_HINT: &str` names the two install commands from spec section 13. Every invocation emits
  exactly 18 named Stage 1 checks, including failures and explicit dependent checks that could not run.

- [ ] **Step 1: Write the failing tests**

In adapter `lib.rs`, add `mod git_tree;` and `pub use git_tree::enclosing_git_tree;` now. The file starts
with the test module below. The output tests are appended inside the existing `#[cfg(test)] mod tests` in
`cli/output.rs`. Cargo discovers `tests/doctor.rs` automatically.

`crates/vpt-adapters/src/git_tree.rs`, test section:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nearest_ancestor_with_a_git_entry_is_found_from_a_nested_path() {
        let dir = tempfile::tempdir().expect("dir");
        std::fs::create_dir_all(dir.path().join("vault/.git")).expect("git");
        std::fs::create_dir_all(dir.path().join("vault/notes/vpt")).expect("nested");
        assert_eq!(enclosing_git_tree(&dir.path().join("vault/notes/vpt")), Some(dir.path().join("vault")));
    }

    #[test]
    fn a_git_file_counts_and_no_entry_is_none() {
        let dir = tempfile::tempdir().expect("dir");
        std::fs::create_dir_all(dir.path().join("worktree/sub")).expect("worktree");
        std::fs::write(dir.path().join("worktree/.git"), b"gitdir: elsewhere").expect("git file");
        assert_eq!(enclosing_git_tree(&dir.path().join("worktree/sub")), Some(dir.path().join("worktree")));
        assert_eq!(enclosing_git_tree(&dir.path().join("plain")), None);
    }
}
```

`crates/vpt/src/cli/output.rs`, appended test:

```rust
    #[test]
    fn a_failed_report_keeps_the_error_exit_code() {
        let outcome = Outcome::FailedReport { error: ErrorDocument::new(ErrorKind::Refused, "x").rule("doctor_checks"), human: "ok config\n".into() };
        let mut out = Vec::new();
        let mut err = Vec::new();
        assert_eq!(emit_to(outcome, true, &mut out, &mut err), 3);
        assert!(out.is_empty());
        assert_eq!(serde_json::from_slice::<serde_json::Value>(&err).expect("error")["error"]["rule"], "doctor_checks");
    }

    #[test]
    fn failed_stdout_is_a_store_error_for_both_output_modes() {
        struct Broken;
        impl Write for Broken {
            fn write(&mut self, _bytes: &[u8]) -> std::io::Result<usize> {
                Err(std::io::ErrorKind::BrokenPipe.into())
            }
            fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
        }
        for json in [false, true] {
            let mut err = Vec::new();
            let outcome = Outcome::Success { document: serde_json::json!({"ok": true}), human: "ok\n".into() };
            assert_eq!(emit_to(outcome, json, &mut Broken, &mut err), 1);
            if json {
                assert_eq!(serde_json::from_slice::<serde_json::Value>(&err).expect("error")["error"]["kind"], "store");
            } else {
                assert!(String::from_utf8(err).expect("text").contains("standard output write failed"));
            }
        }
    }

```

`crates/vpt/tests/doctor.rs`:

```rust
mod support;

use support::{Sandbox, run, stderr, stdout};

fn checks(document: &serde_json::Value, key: &str) -> Vec<(String, bool, String)> {
    let list: Vec<(String, bool, String)> = document[key].as_array().expect("checks array").iter().map(|check| (
        check["name"].as_str().expect("name").to_owned(),
        check["ok"].as_bool().expect("ok"),
        check["detail"].as_str().expect("detail").to_owned(),
    )).collect();
    let names: std::collections::BTreeSet<&str> = list.iter().map(|(name, _, _)| name.as_str()).collect();
    let expected = [
        "config", "stores", "helper", "recordings_dir", "symlink", "cleanup_pending", "source_gone",
        "git_tree:audio", "git_tree:transcripts", "git_tree:analysis", "git_tree:briefs",
        "git_tree:engine_outputs", "git_tree:drafts", "git_tree:released",
        "subdirectory:Capture", "subdirectory:CaptureRecovery",
        "subdirectory:CloudRecordings_ckAssets", "subdirectory:EncryptedCloudRecordings",
    ].into_iter().collect();
    assert_eq!(names, expected);
    assert_eq!(list.len(), 18);
    list
}

fn check<'a>(list: &'a [(String, bool, String)], name: &str) -> &'a (String, bool, String) {
    list.iter().find(|(n, _, _)| n == name).unwrap_or_else(|| panic!("no check {name} in {list:?}"))
}

#[test]
fn a_healthy_setup_passes_every_check_and_prints_them_on_stdout() {
    let sandbox = Sandbox::new("doctor-ok");
    sandbox.install_fake_helper();
    sandbox.write_config("");

    let output = run(sandbox.vpt().args(["doctor", "--json"]));

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let document: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("json");
    assert_eq!(document["command"], "doctor");
    let list = checks(&document, "checks");
    assert!(list.iter().all(|(_, ok, _)| *ok), "{list:?}");
    assert!(check(&list, "helper").2.contains("1.0.0"));
    assert!(check(&list, "recordings_dir").2.contains("readable"));
    assert_eq!(check(&list, "symlink").2, "not configured");
    assert!(check(&list, "subdirectory:Capture").2.contains("absent"));
    assert_eq!(check(&list, "cleanup_pending").2, "none");
    assert_eq!(check(&list, "git_tree:audio").2, "not in a git tree");
    assert!(!sandbox.path().join("state/vpt").exists());
    assert!(!sandbox.path().join("home/.vpt/audio").exists());
}

#[test]
fn a_missing_helper_fails_with_the_two_install_commands_and_the_human_run_lists_every_check() {
    let sandbox = Sandbox::new("doctor-helper");
    sandbox.write_config("");

    let output = run(sandbox.vpt().args(["doctor", "--json"]));

    assert_eq!(output.status.code(), Some(3));
    assert!(stdout(&output).is_empty());
    let error: serde_json::Value = serde_json::from_str(&stderr(&output)).expect("json");
    assert_eq!(error["error"]["rule"], "doctor_checks");
    let list = checks(&error["error"], "checks");
    let helper = check(&list, "helper");
    assert!(!helper.1);
    assert!(helper.2.contains("cargo install --git https://github.com/webdavis/vpt vpt"), "{}", helper.2);
    assert!(helper.2.contains("swift build -c release"), "{}", helper.2);
    let human = run(sandbox.vpt().arg("doctor"));
    assert_eq!(human.status.code(), Some(3));
    assert!(stderr(&human).contains("config") && stderr(&human).contains("FAIL helper"), "{}", stderr(&human));
}

#[test]
fn doctor_preserves_additive_helper_diagnostics_on_success_and_refusal() {
    use std::os::unix::fs::PermissionsExt;
    for (version, exit) in [("1.0.0", 0), ("2.0.0", 3)] {
        let sandbox = Sandbox::new("doctor-helper-diagnostics");
        sandbox.write_config("");
        let helper = sandbox.path().join("bin/vpt-macos");
        let reply = serde_json::json!({"schema":"vpt.helper/1", "version":version, "capabilities":[]});
        std::fs::write(&helper, format!("#!/bin/sh\nprintf '%s\\n' '{reply}'\n")).expect("helper fixture");
        std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o755)).expect("executable");
        let output = run(sandbox.vpt().args(["doctor", "--json"]));
        assert_eq!(output.status.code(), Some(exit));
        let text = if exit == 0 { stdout(&output) } else { stderr(&output) };
        let document: serde_json::Value = serde_json::from_str(&text).expect("one document");
        let body = if exit == 0 { &document } else { &document["error"] };
        let list = checks(body, "checks");
        assert_eq!(check(&list, "helper").1, exit == 0);
        assert!(check(&list, "helper").2.contains("unknown additive helper field: capabilities"));
        assert!(!sandbox.path().join("state/vpt").exists());
    }
}

#[test]
fn without_a_config_the_config_check_fails_and_the_rest_are_reported_as_not_run() {
    let sandbox = Sandbox::new("doctor-no-config");

    let output = run(sandbox.vpt().args(["doctor", "--json"]));

    assert_eq!(output.status.code(), Some(3));
    let error: serde_json::Value = serde_json::from_str(&stderr(&output)).expect("json");
    let list = checks(&error["error"], "checks");
    assert!(!check(&list, "config").1);
    assert!(check(&list, "config").2.contains("vpt setup"));
    assert_eq!(check(&list, "helper").2, "not run: the configuration did not load");
}

#[test]
fn a_store_inside_a_git_tree_is_reported_and_the_sensitive_two_are_named() {
    let sandbox = Sandbox::new("doctor-git");
    sandbox.install_fake_helper();
    sandbox.write_config("");
    std::fs::create_dir_all(sandbox.path().join("home/.git")).expect("git tree");

    let output = run(sandbox.vpt().args(["doctor", "--json"]));

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let document: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("json");
    let list = checks(&document, "checks");
    assert!(check(&list, "git_tree:audio").2.contains("confirm the ignore rules exclude audio"));
    assert!(check(&list, "git_tree:drafts").2.contains("sensitive"));
    assert!(check(&list, "git_tree:engine_outputs").2.contains("sensitive"));
    assert!(!check(&list, "git_tree:transcripts").2.contains("sensitive"));
}

#[test]
fn a_staged_file_left_behind_is_cleanup_pending_and_a_managed_link_is_verified() {
    let sandbox = Sandbox::new("doctor-cleanup");
    sandbox.install_fake_helper();
    sandbox.write_config("");
    std::fs::create_dir_all(sandbox.path().join("home/.vpt/audio")).expect("audio");
    std::fs::write(sandbox.path().join("home/.vpt/audio/.vpt-staging-1"), b"x").expect("leftover");

    let output = run(sandbox.vpt().args(["doctor", "--json"]));

    assert_eq!(output.status.code(), Some(3));
    let error: serde_json::Value = serde_json::from_str(&stderr(&output)).expect("json");
    let list = checks(&error["error"], "checks");
    let pending = check(&list, "cleanup_pending");
    assert!(!pending.1);
    assert!(pending.2.contains(".vpt-staging-1"), "{}", pending.2);
}

#[test]
fn a_root_failure_preserves_independent_checks_and_marks_dependencies() {
    let sandbox = Sandbox::new("doctor-bad-root");
    sandbox.install_fake_helper();
    sandbox.write_config(&format!("[stores]\naudio = {:?}\n", sandbox.path().join("absent/audio")));
    let output = run(sandbox.vpt().args(["doctor", "--json"]));
    assert_eq!(output.status.code(), Some(3));
    let document: serde_json::Value = serde_json::from_str(&stderr(&output)).expect("json");
    let list = checks(&document["error"], "checks");
    assert!(!check(&list, "stores").1);
    assert!(check(&list, "helper").1);
    assert!(check(&list, "recordings_dir").1);
    assert!(check(&list, "git_tree:audio").1);
    assert!(check(&list, "cleanup_pending").2.starts_with("not run:"));
    assert!(!sandbox.path().join("state/vpt").exists());
}

#[test]
fn one_bad_apple_subdirectory_does_not_suppress_the_other_checks() {
    let sandbox = Sandbox::new("doctor-subdirectory");
    sandbox.install_fake_helper();
    sandbox.write_config("");
    let source = sandbox.path().join("voice-memos/Recordings");
    std::os::unix::fs::symlink(sandbox.path(), source.join("Capture")).expect("link");
    let output = run(sandbox.vpt().args(["doctor", "--json"]));
    assert_eq!(output.status.code(), Some(3));
    let document: serde_json::Value = serde_json::from_str(&stderr(&output)).expect("json");
    let list = checks(&document["error"], "checks");
    assert!(!check(&list, "subdirectory:Capture").1);
    assert!(check(&list, "subdirectory:CaptureRecovery").1);
    assert!(check(&list, "helper").1);
}

#[test]
fn doctor_and_ingest_dry_run_preserve_existing_state() {
    use vpt_domain::fixtures::m4a;
    let sandbox = Sandbox::new("doctor-observes");
    sandbox.install_fake_helper();
    sandbox.write_config("");
    sandbox.add_recording("a.m4a", &m4a(1_787_690_856, 1, b"audio"));
    assert!(run(sandbox.vpt().arg("ingest")).status.success());
    let before = support::state_snapshot(&sandbox);
    for args in [vec!["doctor", "--json"], vec!["ingest", "--dry-run", "--json"]] {
        let output = run(sandbox.vpt().args(args));
        assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
        assert_eq!(support::state_snapshot(&sandbox), before);
    }
}

#[test]
fn doctor_reports_git_ancestry_through_home_links_even_when_another_root_fails() {
    for bad_root in [false, true] {
        let sandbox = Sandbox::new("doctor-resolved-git");
        sandbox.install_fake_helper();
        sandbox.write_config("");
        let repository = sandbox.path().join("repository");
        std::fs::create_dir_all(repository.join(".git")).expect("repository");
        std::fs::rename(sandbox.path().join("home/.vpt"), repository.join("vault")).expect("move home");
        std::os::unix::fs::symlink(repository.join("vault"), sandbox.path().join("home/.vpt")).expect("home link");
        if bad_root {
            sandbox.write_config(&format!("[stores]\nreleased = {:?}\n", sandbox.path().join("missing/released")));
        }
        let output = run(sandbox.vpt().args(["doctor", "--json"]));
        assert_eq!(output.status.code(), Some(if bad_root { 3 } else { 0 }), "{}", stderr(&output));
        let text = if bad_root { stderr(&output) } else { stdout(&output) };
        let document: serde_json::Value = serde_json::from_str(&text).expect("document");
        let list = checks(if bad_root { &document["error"] } else { &document }, "checks");
        for key in ["audio", "drafts", "engine_outputs"] {
            assert!(check(&list, &format!("git_tree:{key}")).2.contains(&repository.display().to_string()));
        }
        assert_eq!(check(&list, "stores").1, !bad_root);
        assert!(!sandbox.path().join("state/vpt").exists());
    }
}

```

- [ ] **Step 2: Run the tests to verify they fail**

Run each command separately: `cargo test -p vpt-adapters git_tree`,
`cargo test -p vpt --features dev-tools --lib cli::output`, and
`cargo test -p vpt --features dev-tools --test doctor`. Expected: each new module is compiled and
selected and fails on the missing implementation. Zero selected tests or a successful command does not
satisfy this step.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-adapters/src/git_tree.rs`:

```rust
//! Whether a path sits inside a git working tree: a walk up for a `.git` entry.

use std::path::{Path, PathBuf};

pub fn enclosing_git_tree(path: &Path) -> Option<PathBuf> {
    path.ancestors().find(|ancestor| ancestor.join(".git").symlink_metadata().is_ok()).map(Path::to_path_buf)
}
```

In Step 1 add `mod git_tree;` and `pub use git_tree::enclosing_git_tree;` to the adapters `lib.rs`. In
`crates/vpt/src/cli/output.rs` the enum and `emit` become:

```rust
pub enum Outcome {
    Success { document: serde_json::Value, human: String },
    Failure(ErrorDocument),
    FailedReport { error: ErrorDocument, human: String },
}

pub fn emit(outcome: Outcome, json: bool) -> i32 {
    emit_to(outcome, json, &mut std::io::stdout(), &mut std::io::stderr())
}

fn emit_to(outcome: Outcome, json: bool, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    match outcome {
        Outcome::Success { document, human } => {
            let text = if json { format!("{document}\n") } else { human };
            if stdout.write_all(text.as_bytes()).is_err() {
                return failure(&ErrorDocument::new(ErrorKind::Store, "standard output write failed"), json, "", stderr);
            }
            0
        }
        Outcome::Failure(error) => failure(&error, json, "", stderr),
        Outcome::FailedReport { error, human } => failure(&error, json, &human, stderr),
    }
}

fn failure(error: &ErrorDocument, json: bool, report: &str, stderr: &mut dyn Write) -> i32 {
    let diagnostics = error.diagnostics.iter().map(|line| format!("note: {line}\n")).collect::<String>();
    let text = if json { format!("{}\n", error.to_json()) } else { format!("{report}{diagnostics}vpt: {}\n", error.message) };
    let _ = stderr.write_all(text.as_bytes());
    error.exit_code()
}
```

`crates/vpt/src/doctor/mod.rs`:

```rust
//! `vpt doctor`: every check, always, then one verdict.

mod checks;
mod probes;

use crate::cli::output::Outcome;
use crate::compose::Environment;
use serde_json::json;
use std::path::Path;
use vpt_protocol::error::{Check, ErrorDocument, ErrorKind};
use vpt_protocol::result::document;

pub const INSTALL_HINT: &str = "install with `cargo install --git https://github.com/webdavis/vpt vpt`, then \
`swift build -c release` in helper/vpt-macos and copy .build/release/vpt-macos beside vpt";

pub fn run(environment: &Environment, config: Option<&Path>) -> Outcome {
    let checks = checks::all(environment, config);
    let human = checks.iter().map(line).collect::<String>();
    let failed = checks.iter().filter(|check| !check.ok).count();
    if failed == 0 {
        return Outcome::Success { human, document: document("doctor", json!({"checks": checks})) };
    }
    let error = ErrorDocument::new(ErrorKind::Refused, format!("{failed} checks failed")).rule("doctor_checks").checks(checks);
    Outcome::FailedReport { error, human }
}

fn line(check: &Check) -> String {
    format!("{} {:<26} {}\n", if check.ok { "ok  " } else { "FAIL" }, check.name, check.detail)
}
```

`crates/vpt/src/doctor/checks.rs`:

```rust
use super::probes::{cleanup_pending, git_tree, helper, source_gone, subdirectory, symlink};
use crate::compose::{Environment, settings_from};
use std::path::Path;
use vpt_adapters::VoiceMemosStore;
use vpt_adapters::config::{planned_directory, resolve};
use vpt_application::ports::RecorderStore;
use vpt_domain::layout::StoreKey;
use vpt_protocol::error::Check;

const APPLE: [&str; 4] = ["Capture", "CaptureRecovery", "CloudRecordings_ckAssets", "EncryptedCloudRecordings"];
const NOT_RUN: &str = "not run: the configuration did not load";

pub(super) fn check(name: impl Into<String>, ok: bool, detail: impl Into<String>) -> Check {
    Check { name: name.into(), ok, detail: detail.into() }
}

fn names() -> Vec<String> {
    let mut names: Vec<String> = ["config", "stores", "helper", "recordings_dir", "symlink", "cleanup_pending", "source_gone"]
        .map(str::to_owned).into();
    names.extend(StoreKey::all().map(|key| format!("git_tree:{}", key.key_name())));
    names.extend(APPLE.map(|name| format!("subdirectory:{name}")));
    names
}

pub(super) fn all(environment: &Environment, config: Option<&Path>) -> Vec<Check> {
    let settings = match settings_from(environment, config) {
        Ok(settings) => settings,
        Err(error) => return names().into_iter().map(|name| {
            let detail = if name == "config" { error.message.clone() } else { NOT_RUN.into() };
            check(name, false, detail)
        }).collect(),
    };
    let mut checks = vec![check("config", true, format!("config_version {}", settings.config_version))];
    let selected = environment.selected_config(config);
    let config_dir = selected.parent().filter(|path| !path.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let roots = resolve(&settings, config_dir);
    checks.push(match &roots {
        Ok(roots) => check("stores", true, format!("home {}", roots.home.display())),
        Err(error) => check("stores", false, format!("{error:?}")),
    });
    checks.extend(StoreKey::all().map(|key| {
        let path = match &roots {
            Ok(roots) => Ok(roots.stores.get(key).to_path_buf()),
            Err(_) => planned_directory(settings.stores.get(key), &format!("stores.{}", key.key_name())),
        };
        match path {
            Ok(path) => git_tree(key, &path),
            Err(error) => check(format!("git_tree:{}", key.key_name()), false, format!("{error:?}")),
        }
    }));
    checks.push(helper(&settings));
    let source = if settings.source.recordings_dir.is_absolute() {
        settings.source.recordings_dir.canonicalize()
            .map_err(|error| error.kind().to_string())
            .and_then(|path| VoiceMemosStore::open(&path).map_err(|error| format!("{error:?}")))
    } else {
        Err("source.recordings_dir must be absolute".into())
    };
    checks.push(match &source {
        Ok(store) => match store.candidates() {
            Ok(candidates) => check("recordings_dir", true, format!("readable, {} candidates", candidates.len())),
            Err(error) => check("recordings_dir", false, format!("{error:?}")),
        },
        Err(detail) => check("recordings_dir", false, detail),
    });
    checks.push(symlink(&settings));
    for name in APPLE {
        checks.push(match &source {
            Ok(store) => subdirectory(store.recordings_dir(), name),
            Err(_) => check(format!("subdirectory:{name}"), false, "not run: recordings_dir could not be opened"),
        });
    }
    match &roots {
        Ok(roots) => {
            checks.push(cleanup_pending(&roots.stores.audio));
            checks.push(source_gone(&roots.state_dir));
        }
        Err(_) => {
            checks.push(check("cleanup_pending", false, "not run: store roots did not resolve"));
            checks.push(source_gone(&settings.state_dir));
        }
    }
    checks
}

```

`crates/vpt/src/doctor/probes.rs`:

```rust
use super::{INSTALL_HINT, checks::check};
use crate::compose::RuntimeLedger;
use std::path::Path;
use vpt_adapters::{
    ClonefileArchive, HelperClient, HelperError, LinkState, RootDir, BUILT_AGAINST_MAJOR,
    enclosing_git_tree, inspect_symlink, verify_symlink,
};
use vpt_application::ports::{Archive, RecordingLedger};
use vpt_application::Settings;
use vpt_domain::layout::StoreKey;
use vpt_protocol::error::Check;

pub(super) fn git_tree(key: StoreKey, path: &Path) -> Check {
    let name = format!("git_tree:{}", key.key_name());
    if !path.is_absolute() { return check(name, false, "store path must be absolute"); }
    match enclosing_git_tree(path) {
        None => check(name, true, "not in a git tree"),
        Some(tree) => {
            let note = match key {
                StoreKey::Audio => "; confirm the ignore rules exclude audio",
                StoreKey::Drafts | StoreKey::EngineOutputs => "; sensitive: this store holds private JSON",
                _ => "",
            };
            check(name, true, format!("inside git tree {}{note}", tree.display()))
        }
    }
}

pub(super) fn helper(settings: &Settings) -> Check {
    let helper = HelperClient::new(settings.helper_path.clone());
    let mut result = match helper.version() {
        Ok(version) => check("helper", true, format!("vpt-macos {version} at {}", settings.helper_path.display())),
        Err(HelperError::Absent) => check("helper", false, format!("{} not found; {INSTALL_HINT}", settings.helper_path.display())),
        Err(HelperError::MajorMismatch { found }) => check("helper", false, format!("major version {found}, this build needs {BUILT_AGAINST_MAJOR}")),
        Err(error) => check("helper", false, format!("{error:?}")),
    };
    for diagnostic in helper.take_diagnostics() {
        result.detail.push_str(&format!("; {diagnostic}"));
    }
    result
}

pub(super) fn subdirectory(recordings: &Path, name: &str) -> Check {
    let result = RootDir::open(recordings).and_then(|root| root.open_subdirectory(name));
    let label = format!("subdirectory:{name}");
    match result {
        Ok(root) => match root.names() {
            Ok(names) => check(label, true, format!("{} entries", names.len())),
            Err(error) => check(label, false, format!("{error:?}")),
        },
        Err(vpt_adapters::ContainedError::Io { kind: std::io::ErrorKind::NotFound, .. }) => check(label, true, "absent"),
        Err(error) => check(label, false, format!("{error:?}")),
    }
}

pub(super) fn symlink(settings: &Settings) -> Check {
    match &settings.symlink_target {
        Some(target) => match verify_symlink(&settings.home, target) {
            Ok(()) => check("symlink", true, format!("{} -> {}", settings.home.display(), target.display())),
            Err(error) => check("symlink", false, format!("{error:?}")),
        },
        None => match inspect_symlink(&settings.home) {
            LinkState::LinkTo(target) => check("symlink", true, format!("followed, not managed: {} -> {}", settings.home.display(), target.display())),
            LinkState::Unreadable(detail) => check("symlink", false, detail),
            _ => check("symlink", true, "not configured"),
        },
    }
}

pub(super) fn cleanup_pending(audio: &Path) -> Check {
    let archive = match ClonefileArchive::open_read_only(audio) {
        Ok(archive) => archive,
        Err(error) => return check("cleanup_pending", false, format!("{error:?}")),
    };
    match archive.staged_leftovers() {
        Ok(leftovers) if leftovers.is_empty() => check("cleanup_pending", true, "none"),
        Ok(leftovers) => {
            let names: Vec<String> = leftovers.iter().map(|path| path.display().to_string()).collect();
            check("cleanup_pending", false, format!("{} staged files await the Trash: {}", names.len(), names.join(", ")))
        }
        Err(error) => check("cleanup_pending", false, format!("{error:?}")),
    }
}

pub(super) fn source_gone(state: &Path) -> Check {
    if !state.is_absolute() { return check("source_gone", false, "home.state_dir must be absolute"); }
    let rows = RuntimeLedger::read_only(state).and_then(|ledger| {
        ledger.seen_all().map_err(|error| Box::new(crate::compose::errors::ledger_error(error)))
    });
    match rows {
        Ok(rows) => check("source_gone", true, format!("{} recordings whose source is gone", rows.iter().filter(|row| row.source_gone_at.is_some()).count())),
        Err(error) => check("source_gone", false, error.message),
    }
}
```

Every configured invocation emits the same 18 check names. Root resolution failure does not suppress
helper, source, symlink, git-tree or ledger checks. Only cleanup depends on the whole store-root
validation. An absent archive and ledger are empty observations. No doctor path creates or repairs state.

Dispatch: `Verb::Doctor => doctor::run(&environment, invocation.config.as_deref()),` with `mod doctor;`
in `lib.rs`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo fmt --all`

Run: `cargo test --workspace --features dev-tools`

Expected: all PASS. Format before counting: `checks.rs` owns the census, `probes.rs` owns independent
adapter observations, and both remain below 200 implementation lines.

- [ ] **Step 5: Commit**

Run: `cargo fmt --all`

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(doctor): every stage 1 check with the doctor_checks verdict"
```

______________________________________________________________________

### Task 31: The retention decision and the intents journal

Retention needs two things before it can run: the pure expiry rule and a journal of intents that survives
a crash between the intent and the helper's answer. The journal is a port with the SQLite and in-memory
ledgers behind it, under one contract, like the two before it.

**Files:**

- Modify: `crates/vpt-domain/src/retention.rs`
- Create: `crates/vpt-application/src/ports/retention.rs`
- Modify: `crates/vpt-application/src/ports/mod.rs`
- Create: `crates/vpt-adapters/src/ledger/sqlite/retention.rs`
- Create: `crates/vpt-adapters/src/ledger/contract/retention.rs`
- Modify: `crates/vpt-adapters/src/ledger/sqlite/mod.rs`, `crates/vpt-adapters/src/ledger/memory.rs`,
  `crates/vpt-adapters/src/ledger/contract.rs`
- Modify: `crates/vpt/src/compose/ledger.rs`
- Test: the contract invocations in both ledgers' test modules gain the new suite

**Interfaces:**

- Consumes: `Hold`, `FileTime`, `UtcInstant`, `LedgerError`, the `retention_intents` table of Task 11.
- Produces:
  - `vpt_domain::retention::expired(mtime: FileTime, hold: Hold, now: UtcInstant) -> bool`.
  - `vpt_application::ports::{ArtifactKind::Audio,`
    `NewIntent { pub kind: ArtifactKind, pub recording: RecordingId, pub path: PathBuf,`
    `pub expected: Sha256Digest, pub recorded_at: UtcInstant },`
    `RetentionIntent { pub id: i64, pub kind: ArtifactKind, pub recording: RecordingId,`
    `pub path: PathBuf, pub expected: Sha256Digest, pub recorded_at: UtcInstant }, RetentionJournal}`
    with `fn record_intent(&self, intent: &NewIntent) -> Result<i64, LedgerError>`,
    `fn pending_intents(&self) -> Result<Vec<RetentionIntent>, LedgerError>`,
    `fn complete_intent(&self, id: i64, at: UtcInstant) -> Result<(), LedgerError>`.
  - `retention_journal_contract!` and the two implementations.

The `expected` column is text so a later stage can record a note's identity and stage there; stage 1
records the digest hex, the only ownership proof an audio clone has.

- [ ] **Step 1: Write the failing tests**

The existing `retention` domain module already compiles. Register `mod retention;` in the application
ports module and `pub use retention::{ArtifactKind, NewIntent, RetentionIntent, RetentionJournal};` in
Step 1, with the new file containing only the tests until Step 3. Register `mod retention;` in the SQLite
module before the red run. Keep `contract` and both ledgers' test modules private.

`crates/vpt-domain/src/retention.rs`, test section:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const NOW: UtcInstant = UtcInstant { secs: 1_000_000 };

    #[test]
    fn a_hold_of_never_expires_nothing() {
        assert!(!expired(FileTime { secs: 0, nanos: 0 }, Hold::never(), NOW));
    }

    #[test]
    fn a_file_expires_at_the_hold_boundary_and_not_one_second_before() {
        let hold = Hold::of_seconds(86_400);
        assert!(!expired(FileTime { secs: NOW.secs - 86_399, nanos: 0 }, hold, NOW));
        assert!(expired(FileTime { secs: NOW.secs - 86_400, nanos: 0 }, hold, NOW));
        assert!(expired(FileTime { secs: 0, nanos: 0 }, hold, NOW));
    }

    #[test]
    fn a_file_from_the_future_never_expires() {
        assert!(!expired(FileTime { secs: NOW.secs + 10, nanos: 0 }, Hold::of_seconds(1), NOW));
    }

    #[test]
    fn a_large_hold_does_not_wrap_and_nanoseconds_delay_the_boundary() {
        assert!(!expired(FileTime { secs: 0, nanos: 0 }, Hold::of_seconds(u64::MAX), NOW));
        let hold = Hold::of_seconds(1);
        assert!(!expired(FileTime { secs: NOW.secs - 1, nanos: 1 }, hold, NOW));
        assert!(expired(FileTime { secs: NOW.secs - 1, nanos: 0 }, hold, NOW));
        assert!(!expired(FileTime { secs: i64::MIN, nanos: 0 }, Hold::of_seconds(u64::MAX), UtcInstant { secs: i64::MAX - 1 }));
    }
}
```

`crates/vpt-adapters/src/ledger/contract.rs` registers its private child in Step 1:

```rust
mod retention;
pub(crate) use retention::{retention_journal_contract, retention_scenarios};
```

`crates/vpt-adapters/src/ledger/contract/retention.rs`:

```rust
pub(crate) mod retention_scenarios {
    use super::super::digest;
    use std::path::PathBuf;
    use vpt_application::ports::{ArtifactKind, NewIntent, RetentionJournal};
    use vpt_domain::identity::RecordingId;
    use vpt_domain::time::UtcInstant;

    fn intent(seed: u8) -> NewIntent {
        NewIntent {
            kind: ArtifactKind::Audio,
            recording: RecordingId::parse("2026-08-24T144736-4f3ab19c02de").expect("id"),
            path: PathBuf::from(format!("/store/{seed}.m4a")),
            expected: digest(seed),
            recorded_at: UtcInstant { secs: 100 },
        }
    }

    pub fn a_recorded_intent_is_pending_with_its_fields<J: RetentionJournal>(journal: &J) {
        let id = journal.record_intent(&intent(1)).expect("recorded");
        let pending = journal.pending_intents().expect("pending");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, id);
        assert_eq!(pending[0].path, PathBuf::from("/store/1.m4a"));
        assert_eq!(pending[0].expected, digest(1));
        assert_eq!(pending[0].kind, ArtifactKind::Audio);
    }

    pub fn completing_an_intent_removes_it_and_keeps_the_others_in_order<J: RetentionJournal>(journal: &J) {
        let first = journal.record_intent(&intent(1)).expect("first");
        let second = journal.record_intent(&intent(2)).expect("second");
        journal.complete_intent(first, UtcInstant { secs: 200 }).expect("completed");
        let pending = journal.pending_intents().expect("pending");
        assert_eq!(pending.iter().map(|i| i.id).collect::<Vec<_>>(), vec![second]);
    }
}

macro_rules! retention_journal_contract {
    ($make:expr) => {
        mod retention_journal_contract {
            use super::*;
            use crate::ledger::contract::retention_scenarios::*;

            #[test]
            fn a_recorded_intent_is_pending_with_its_fields_() {
                let (_guard, journal) = $make();
                a_recorded_intent_is_pending_with_its_fields(&journal);
            }
            #[test]
            fn completing_an_intent_removes_it_and_keeps_the_others_in_order_() {
                let (_guard, journal) = $make();
                completing_an_intent_removes_it_and_keeps_the_others_in_order(&journal);
            }
        }
    };
}
pub(crate) use retention_journal_contract;
```

and, in the `tests` modules of `sqlite/mod.rs` and `memory.rs`, beside the two earlier invocations:

```rust
    crate::ledger::contract::retention_journal_contract!(open_ledger);
```

```rust
    crate::ledger::contract::retention_journal_contract!(fresh);
```

- [ ] **Step 2: Run the tests to verify they fail**

Run independently, including the second command after the first expected failure:

Run: `cargo test -p vpt-domain retention`

Run: `cargo test -p vpt-adapters ledger`

Expected: compile errors naming `expired`, `NewIntent`, `RetentionJournal`. The new test modules must be
compiled and selected. Zero selected tests or a successful command does not satisfy this step.

- [ ] **Step 3: Write the minimal implementation**

Insert the imports and `expired` function before the existing test module in
`crates/vpt-domain/src/retention.rs`. Keep the test module last:

```rust
use crate::time::{FileTime, UtcInstant};

/// Expired when the hold is set and the file's age has reached it.
pub fn expired(mtime: FileTime, hold: Hold, now: UtcInstant) -> bool {
    let age = (i128::from(now.secs) - i128::from(mtime.secs)) * 1_000_000_000 - i128::from(mtime.nanos);
    let limit = i128::from(hold.seconds()) * 1_000_000_000;
    hold.seconds() != 0 && age >= limit
}
```

`crates/vpt-application/src/ports/retention.rs`:

```rust
//! The journal of retention intents: recorded before the helper is asked,
//! completed after it answers, reconciled after anything else.

use super::ledger::LedgerError;
use std::path::PathBuf;
use vpt_domain::digest::Sha256Digest;
use vpt_domain::identity::RecordingId;
use vpt_domain::time::UtcInstant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactKind {
    Audio,
}

impl ArtifactKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ArtifactKind::Audio => "audio",
        }
    }

    pub fn parse(text: &str) -> Option<ArtifactKind> {
        (text == "audio").then_some(ArtifactKind::Audio)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewIntent {
    pub kind: ArtifactKind,
    pub recording: RecordingId,
    pub path: PathBuf,
    pub expected: Sha256Digest,
    pub recorded_at: UtcInstant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionIntent {
    pub id: i64,
    pub kind: ArtifactKind,
    pub recording: RecordingId,
    pub path: PathBuf,
    pub expected: Sha256Digest,
    pub recorded_at: UtcInstant,
}

pub trait RetentionJournal {
    fn record_intent(&self, intent: &NewIntent) -> Result<i64, LedgerError>;
    fn pending_intents(&self) -> Result<Vec<RetentionIntent>, LedgerError>;
    fn complete_intent(&self, id: i64, at: UtcInstant) -> Result<(), LedgerError>;
}
```

Keep the private module and curated exports registered in Step 1.
`crates/vpt-adapters/src/ledger/sqlite/retention.rs`:

```rust
//! `retention_intents` behind the journal port.

use super::{SqliteLedger, map};
use rusqlite::params;
use std::path::PathBuf;
use vpt_application::ports::{ArtifactKind, LedgerError, NewIntent, RetentionIntent, RetentionJournal};
use vpt_domain::digest::Sha256Digest;
use vpt_domain::identity::RecordingId;
use vpt_domain::time::UtcInstant;

impl RetentionJournal for SqliteLedger {
    fn record_intent(&self, intent: &NewIntent) -> Result<i64, LedgerError> {
        self.transaction(|tx| {
            tx.execute(
                "INSERT INTO retention_intents (artifact_kind, recording, path, expected, recorded_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![intent.kind.as_str(), intent.recording.as_str(), intent.path.to_string_lossy(), intent.expected.hex(), intent.recorded_at.secs],
            )
            .map_err(map)?;
            Ok(tx.last_insert_rowid())
        })
    }

    fn pending_intents(&self) -> Result<Vec<RetentionIntent>, LedgerError> {
        self.read(|connection| {
            let mut statement = connection
                .prepare("SELECT id, artifact_kind, recording, path, expected, recorded_at FROM retention_intents WHERE completed_at IS NULL ORDER BY id")
                .map_err(map)?;
            let rows = statement
                .query_map([], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, String>(4)?, row.get::<_, i64>(5)?))
                })
                .map_err(map)?;
            let mut intents = Vec::new();
            for row in rows {
                let (id, kind, recording, path, expected, recorded_at) = row.map_err(map)?;
                intents.push(RetentionIntent {
                    id,
                    kind: ArtifactKind::parse(&kind).ok_or_else(|| LedgerError::Corrupt(format!("artifact kind {kind}")))?,
                    recording: RecordingId::parse(&recording).map_err(|_| LedgerError::Corrupt(format!("recording {recording}")))?,
                    path: PathBuf::from(path),
                    expected: Sha256Digest::from_hex(&expected).ok_or_else(|| LedgerError::Corrupt(format!("digest {expected}")))?,
                    recorded_at: UtcInstant { secs: recorded_at },
                });
            }
            Ok(intents)
        })
    }

    fn complete_intent(&self, id: i64, at: UtcInstant) -> Result<(), LedgerError> {
        self.transaction(|tx| {
            tx.execute("UPDATE retention_intents SET completed_at = ?2 WHERE id = ?1", params![id, at.secs]).map_err(map)?;
            Ok(())
        })
    }
}
```

`sqlite/mod.rs` adds `mod retention;`. Append to `crates/vpt-adapters/src/ledger/memory.rs`, with
`NewIntent`, `RetentionIntent` and `RetentionJournal` added to its imports:

```rust
impl RetentionJournal for MemoryLedger {
    fn record_intent(&self, intent: &NewIntent) -> Result<i64, LedgerError> {
        self.with(|state| {
            let id = state.intents.len() as i64 + 1;
            state.intents.push((
                RetentionIntent {
                    id,
                    kind: intent.kind,
                    recording: intent.recording.clone(),
                    path: intent.path.clone(),
                    expected: intent.expected,
                    recorded_at: intent.recorded_at,
                },
                None,
            ));
            Ok(id)
        })
    }

    fn pending_intents(&self) -> Result<Vec<RetentionIntent>, LedgerError> {
        self.with(|state| Ok(state.intents.iter().filter(|(_, done)| done.is_none()).map(|(intent, _)| intent.clone()).collect()))
    }

    fn complete_intent(&self, id: i64, at: UtcInstant) -> Result<(), LedgerError> {
        self.with(|state| {
            if let Some((_, done)) = state.intents.iter_mut().find(|(intent, _)| intent.id == id) {
                *done = Some(at);
            }
            Ok(())
        })
    }
}
```

`State` gains `intents: Vec<(RetentionIntent, Option<UtcInstant>)>`. Measure the final file with
`just file-size`; the three implementation blocks do not establish its formatted line count.

Append to `crates/vpt/src/compose/ledger.rs`:

```rust
use vpt_application::ports::{NewIntent, RetentionIntent, RetentionJournal};

impl RetentionJournal for RuntimeLedger {
    fn record_intent(&self, intent: &NewIntent) -> Result<i64, LedgerError> {
        match self {
            Self::Sqlite(ledger) => ledger.record_intent(intent),
            Self::Empty(ledger) => ledger.record_intent(intent),
        }
    }

    fn pending_intents(&self) -> Result<Vec<RetentionIntent>, LedgerError> {
        match self {
            Self::Sqlite(ledger) => ledger.pending_intents(),
            Self::Empty(ledger) => ledger.pending_intents(),
        }
    }

    fn complete_intent(&self, id: i64, at: UtcInstant) -> Result<(), LedgerError> {
        match self {
            Self::Sqlite(ledger) => ledger.complete_intent(id, at),
            Self::Empty(ledger) => ledger.complete_intent(id, at),
        }
    }
}
```

The existing imports in that file supply `LedgerError` and `UtcInstant`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo fmt --all`

Run: `cargo test --workspace --features dev-tools`

Expected: all PASS, four new contract tests among them.

- [ ] **Step 5: Commit**

Run: `cargo fmt --all`

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(retention): the expiry rule and the intents journal under one contract"
```

______________________________________________________________________

### Task 32: `vpt retention run`

**Files:**

- Create: `crates/vpt-application/src/retention/{mod,report,reconcile}.rs`.
- Modify: `crates/vpt-application/src/lib.rs`.
- Create: `crates/vpt-adapters/tests/retention_reconcile.rs`.
- Create: `crates/vpt/src/compose/recovery.rs`, `crates/vpt/src/commands/retention.rs`.
- Modify: `crates/vpt/src/compose.rs`, `crates/vpt/src/compose/runtime.rs`,
  `crates/vpt/src/commands/{mod,ingest}.rs`, `crates/vpt/src/lib.rs`.
- Test: `crates/vpt/tests/{retention,retention_startup}.rs`; `crates/vpt/tests/support/mod.rs` gains
  `set_mtime`.

**Interfaces:**

- Consumes `RecordingLedger::{recordings,by_id,set_audio_trashed}`, `RetentionJournal`,
  `Stores::{entries,validate_path,digest_of}`, `Trash::trash`, `Clock::now`, `RetentionSettings::hold`,
  `StorePaths::get`, `HelperClient::version`.
- Produces curated application exports:
  - `Retention<'a,L,J,S,T,C> { pub ledger:&'a L, pub journal:&'a J, pub stores:&'a S,`
    `pub trash:&'a T, pub clock:&'a C, pub settings:&'a RetentionSettings, pub paths:&'a StorePaths }`,
    with `L:RecordingLedger, J:RetentionJournal, S:Stores, T:Trash, C:Clock` and
    `run(&self,dry_run:bool,initial:ReconcileProgress)->Result<RetentionReport,RetentionFailure>`.
  - `Moved { pub recording:RecordingId, pub store:StoreKey, pub path:PathBuf }`.
  - `KeptReason::{Untracked,DigestMismatch}` with `as_str(self)->&'static str`;
    `Kept { pub path:PathBuf, pub reason:KeptReason }`.
  - `ReconcileProgress { pub moved:Vec<Moved>, pub completed:Vec<RecordingId> }`;
    `ReconcileFailure { pub error:RetentionError, pub progress:ReconcileProgress }`.
  - `RetentionReport { pub moved:Vec<Moved>, pub kept:Vec<Kept>,` `pub completed:Vec<RecordingId> }` and
    `RetentionFailure { pub error:RetentionError,` `pub report:RetentionReport }`.
  - `RetentionError::{Disabled,TargetModified(PathBuf),Ledger(LedgerError),Stores(StoreError),Trash(TrashError)}`.
  - `reconcile_intents<J:RetentionJournal,L:RecordingLedger,S:Stores,T:Trash,C:Clock>(`
    `journal:&J,ledger:&L,stores:&S,trash:&T,clock:&C,audio:&Path)`
    `->Result<ReconcileProgress,ReconcileFailure>`.
- Command composition:
  - `Operation::{Ingest,Retention}` and `AccessMode::{ReadOnly,Mutating}`.
  - `Runtime::load(environment:&Environment,config:Option<&Path>,access:AccessMode,operation:Operation)->Result<Runtime,Box<ErrorDocument>>`.
  - `Runtime.recorder:Option<VoiceMemosStore>`; only `Operation::Ingest` opens it.
  - `compose::errors::ledger_error(error:LedgerError)->ErrorDocument` maps
    `LedgerError::PathEscape(PathBuf)` to exit 3 `path_escape`.
  - `StartupFailure { pub error:ErrorDocument, pub progress:ReconcileProgress }`.
  - `Runtime::mutating(&self)->Result<ReconcileProgress,Box<StartupFailure>>`.
  - `Runtime::retention_notice(&self,moved:&[Moved])`.
  - `Runtime::finish_recovery(&self,outcome:Outcome,progress:&ReconcileProgress)->Outcome`.
  - `commands::retention::run(runtime:&Runtime,dry_run:bool)->Outcome`.
  - `Sandbox::set_mtime(&self,path:&Path,seconds_ago:u64)`.

Retention reports carry recording identities separately from paths. Recovery returns every successful
side effect even when a later intent fails. The command emits one retention event after combining
recovery with new moves, including on partial failure. Dry runs use the read-only runtime and never
reconcile.

- [ ] **Step 1: Write the failing tests**

Register the private `retention` application module and its curated exports before the red run. Register
`mod report; mod reconcile;` in `retention/mod.rs` and create those files before importing their types.
Integration tests are automatically selected by their explicit `--test` names.

`crates/vpt-adapters/tests/retention_reconcile.rs`:

```rust
use std::path::{Path, PathBuf};
use vpt_adapters::{FilesystemStores, MemoryLedger};
use vpt_application::ports::{
    ArtifactKind, Clock, LedgerCommit, LedgerError, NewIntent, RecordingLedger, RecordingRecord,
    RetentionIntent, RetentionJournal, StageStates, StoreError, Stores, TitleOrigin, Trash, TrashError,
};
use vpt_application::{ReconcileFailure, RetentionError, reconcile_intents};
use vpt_domain::identity::RecordingId;
use vpt_domain::time::{UtcInstant, UtcOffset};

struct FixedClock;
impl Clock for FixedClock {
    fn now(&self) -> UtcInstant { UtcInstant { secs: 1_787_690_916 } }
    fn offset_at(&self, _: UtcInstant) -> UtcOffset { UtcOffset { secs: 0 } }
}

struct LocalTrash {
    root: PathBuf,
    calls: std::cell::Cell<usize>,
}
impl Trash for LocalTrash {
    fn trash(&self, path: &Path) -> Result<PathBuf, TrashError> {
        self.calls.set(self.calls.get() + 1);
        let target = self.root.join(path.file_name().expect("fixture leaf"));
        std::fs::rename(path, &target).map_err(|e| TrashError::Failed(e.kind().to_string()))?;
        Ok(target)
    }
}

struct World {
    dir: tempfile::TempDir,
    audio: PathBuf,
    ledger: MemoryLedger,
    stores: FilesystemStores,
    trash: LocalTrash,
}
fn fixture_world() -> World {
    let dir = tempfile::tempdir().expect("dir");
    let audio = dir.path().join("audio");
    let trash = dir.path().join("trash");
    std::fs::create_dir(&audio).expect("audio");
    std::fs::create_dir(&trash).expect("trash");
    let audio = audio.canonicalize().expect("canonical audio root");
    World {
        stores: FilesystemStores::open(std::slice::from_ref(&audio)).expect("stores"),
        audio,
        ledger: MemoryLedger::new(),
        trash: LocalTrash { root: trash, calls: std::cell::Cell::new(0) },
        dir,
    }
}
fn intent(world: &World, path: PathBuf, bytes: &[u8], present: bool) -> RecordingId {
    if present { std::fs::write(&path, bytes).expect("fixture"); }
    let digest = world.stores.digest_bytes(bytes);
    let at = UtcInstant { secs: 1_787_604_456 };
    let offset = UtcOffset { secs: -21_600 };
    let id = RecordingId::derive(at, offset, &digest).expect("id");
    let record = RecordingRecord {
        id: id.clone(), source_path: None, digest, captured_at: at, captured_offset: offset,
        duration_secs: 3, title: None, title_source: TitleOrigin::Unavailable,
        ingested_at: at, audio_path: path.clone(), stages: StageStates::fresh(), audio_trashed_at: None,
    };
    world.ledger.commit(&LedgerCommit {
        recordings: vec![record], ..LedgerCommit::default()
    }).expect("record");
    world.ledger.record_intent(&NewIntent {
        kind: ArtifactKind::Audio, recording: id.clone(), path, expected: digest, recorded_at: at,
    }).expect("intent");
    id
}
fn reconcile(world: &World) -> Result<vpt_application::ReconcileProgress, ReconcileFailure> {
    reconcile_intents(&world.ledger, &world.ledger, &world.stores, &world.trash, &FixedClock, &world.audio)
}

#[test]
fn an_absent_path_completes_the_expiration_without_the_trash() {
    let world = fixture_world();
    let id = intent(&world, world.audio.join("gone.m4a"), b"gone", false);
    let report = reconcile(&world).expect("reconciled");
    assert!(report.moved.is_empty());
    assert_eq!(report.completed, vec![id.clone()]);
    assert_eq!(world.trash.calls.get(), 0);
    assert!(world.ledger.pending_intents().expect("pending").is_empty());
    assert!(world.ledger.by_id(&id).expect("row").expect("record").audio_trashed_at.is_some());
}

#[test]
fn an_unchanged_original_is_moved_again_and_completed() {
    let world = fixture_world();
    let path = world.audio.join("still.m4a");
    let id = intent(&world, path.clone(), b"still", true);
    let report = reconcile(&world).expect("reconciled");
    assert_eq!(report.completed, vec![id]);
    assert_eq!(report.moved[0].path, path);
    assert_eq!(world.trash.calls.get(), 1);
    assert!(world.trash.root.join("still.m4a").exists());
    assert!(world.ledger.pending_intents().expect("pending").is_empty());
}

#[test]
fn replaced_content_is_a_refusal_naming_the_path_and_stays_pending() {
    let world = fixture_world();
    let path = world.audio.join("changed.m4a");
    intent(&world, path.clone(), b"original", true);
    std::fs::write(&path, b"replacement").expect("replacement");
    let failure = reconcile(&world).expect_err("refused");
    assert_eq!(failure.error, RetentionError::TargetModified(path));
    assert!(failure.progress.completed.is_empty());
    assert_eq!(world.trash.calls.get(), 0);
    assert_eq!(world.ledger.pending_intents().expect("pending").len(), 1);
}

#[test]
fn a_later_refusal_retains_the_first_move_and_its_recording_identity() {
    let world = fixture_world();
    let first = world.audio.join("a.m4a");
    let id = intent(&world, first.clone(), b"first", true);
    let second = world.audio.join("b.m4a");
    intent(&world, second.clone(), b"second", true);
    std::fs::write(&second, b"changed").expect("replacement");
    let failure = reconcile(&world).expect_err("second refuses");
    assert_eq!(failure.error, RetentionError::TargetModified(second.clone()));
    assert_eq!(failure.progress.completed, vec![id]);
    assert_eq!(failure.progress.moved[0].path, first);
    assert!(!first.exists());
    assert!(second.exists());
    assert_eq!(world.ledger.pending_intents().expect("pending").len(), 1);
}

struct FailComplete<'a>(&'a MemoryLedger);
impl RetentionJournal for FailComplete<'_> {
    fn record_intent(&self, intent: &NewIntent) -> Result<i64, LedgerError> { self.0.record_intent(intent) }
    fn pending_intents(&self) -> Result<Vec<RetentionIntent>, LedgerError> { self.0.pending_intents() }
    fn complete_intent(&self, _: i64, _: UtcInstant) -> Result<(), LedgerError> {
        Err(LedgerError::Corrupt("injected completion failure".into()))
    }
}

#[test]
fn journal_failure_after_trash_retains_the_move_and_completed_id() {
    let world = fixture_world();
    let path = world.audio.join("a.m4a");
    let id = intent(&world, path.clone(), b"first", true);
    let failure = reconcile_intents(
        &FailComplete(&world.ledger), &world.ledger, &world.stores, &world.trash,
        &FixedClock, &world.audio,
    ).expect_err("completion fails");
    assert!(matches!(failure.error, RetentionError::Ledger(_)));
    assert_eq!(failure.progress.completed, vec![id]);
    assert_eq!(failure.progress.moved[0].path, path);
    assert!(!path.exists());
    assert_eq!(world.ledger.pending_intents().expect("pending").len(), 1);
}

#[test]
fn journal_paths_outside_audio_and_leaf_links_never_reach_trash() {
    let world = fixture_world();
    let outside = world.dir.path().join("outside.m4a");
    intent(&world, outside.clone(), b"outside", true);
    let failure = reconcile(&world).expect_err("escape");
    assert_eq!(failure.error, RetentionError::Stores(StoreError::Escape(outside.clone())));
    assert_eq!(std::fs::read(&outside).expect("untouched"), b"outside");
    assert_eq!(world.trash.calls.get(), 0);

    let world = fixture_world();
    let leaf = world.audio.join("link.m4a");
    intent(&world, leaf.clone(), b"link", false);
    std::os::unix::fs::symlink(&outside, &leaf).expect("symlink");
    assert!(matches!(reconcile(&world).expect_err("link").error, RetentionError::Stores(StoreError::Escape(_))));
    assert_eq!(world.trash.calls.get(), 0);
}

#[test]
fn replacing_the_audio_root_with_a_symlink_never_reaches_trash() {
    let world = fixture_world();
    let path = world.audio.join("a.m4a");
    intent(&world, path, b"original", true);
    let original = world.dir.path().join("original");
    std::fs::rename(&world.audio, &original).expect("move fixture root");
    std::os::unix::fs::symlink(&original, &world.audio).expect("root link");
    assert!(matches!(reconcile(&world).expect_err("root link").error, RetentionError::Stores(StoreError::Escape(_))));
    assert_eq!(world.trash.calls.get(), 0);
    assert_eq!(std::fs::read(original.join("a.m4a")).expect("untouched"), b"original");
}
```

`crates/vpt/tests/retention.rs`:

```rust
mod support;
use support::{Sandbox, run, stderr, stdout};
use vpt_domain::fixtures::m4a;

const CAPTURED: i64 = 1_787_604_456;
const ENABLED: &str = "[retention]\nenabled = true\ninclude_audio = true\n[retention.hold]\naudio = \"1d\"\n";

fn ingested(name: &str, extra: &str) -> (Sandbox, String, std::path::PathBuf) {
    let sandbox = Sandbox::new(name);
    sandbox.install_fake_helper();
    sandbox.write_config(extra);
    sandbox.add_recording("a.m4a", &m4a(CAPTURED, 3, b"audio"));
    let output = run(sandbox.vpt().args(["ingest", "--json"]));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let document: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("json");
    let id = document["ingested"][0]["id"].as_str().expect("id").to_owned();
    let clone = sandbox.path().join(format!("home/.vpt/audio/{id}.m4a"));
    sandbox.set_mtime(&clone, 0);
    (sandbox, id, clone)
}

#[test]
fn an_expired_clone_moves_to_the_trash_through_the_helper_and_the_ledger_records_it() {
    let (sandbox, id, clone) = ingested("retention-move", ENABLED);
    sandbox.set_mtime(&clone, 2 * 86_400);
    let output = run(sandbox.vpt().args(["retention", "run", "--json"]));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let document: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("json");
    assert_eq!(document["moved"], serde_json::json!([{"store":"audio","path":clone}]));
    assert!(!clone.exists());
    assert!(sandbox.path().join(format!("trash/{id}.m4a")).exists());
    let shown = run(sandbox.vpt().args(["show", &id, "--json"]));
    let record: serde_json::Value = serde_json::from_str(&stdout(&shown)).expect("json");
    assert!(record["audio_trashed_at"].is_string());
    let intents: i64 = sandbox.ledger().query_row(
        "SELECT COUNT(*) FROM retention_intents WHERE completed_at IS NOT NULL", [], |r| r.get(0),
    ).expect("count");
    assert_eq!(intents, 1);
}

#[test]
fn an_untracked_file_in_a_store_survives_and_is_reported_kept() {
    let (sandbox, _, _) = ingested("retention-untracked", ENABLED);
    let stray = sandbox.path().join("home/.vpt/audio/stray.m4a");
    std::fs::write(&stray, b"not vpt's").expect("stray");
    sandbox.set_mtime(&stray, 30 * 86_400);
    let output = run(sandbox.vpt().args(["retention", "run", "--json"]));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let document: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("json");
    assert!(stray.exists());
    assert_eq!(document["kept"], serde_json::json!([{"path":stray,"reason":"untracked"}]));
    assert_eq!(document["moved"], serde_json::json!([]));
}

#[test]
fn audio_is_excluded_unless_include_audio_is_set() {
    let (sandbox, _, clone) = ingested("retention-exclude", "[retention]\nenabled = true\n[retention.hold]\naudio = \"1d\"\n");
    sandbox.set_mtime(&clone, 2 * 86_400);
    let output = run(sandbox.vpt().args(["retention", "run", "--json"]));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(clone.exists());
    let document: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("json");
    assert_eq!(document["moved"], serde_json::json!([]));
}

#[test]
fn dry_run_lists_what_would_move_and_moves_nothing() {
    let (sandbox, _, clone) = ingested("retention-dry", ENABLED);
    sandbox.set_mtime(&clone, 2 * 86_400);
    let before = std::fs::read(sandbox.path().join("state/vpt/vpt.db")).expect("db");
    let output = run(sandbox.vpt().args(["retention", "run", "--dry-run", "--json"]));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let document: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("json");
    assert_eq!(document["moved"][0]["path"], clone.to_string_lossy().as_ref());
    assert!(clone.exists());
    assert_eq!(std::fs::read(sandbox.path().join("state/vpt/vpt.db")).expect("db"), before);
    let intents: i64 = sandbox.ledger().query_row("SELECT COUNT(*) FROM retention_intents", [], |r| r.get(0)).expect("count");
    assert_eq!(intents, 0);
    assert!(!sandbox.fake_log().exists());
}

#[test]
fn fresh_dry_run_creates_no_state_or_store_leaf() {
    let sandbox = Sandbox::new("retention-fresh-dry");
    sandbox.install_fake_helper();
    sandbox.write_config(ENABLED);
    let output = run(sandbox.vpt().args(["retention", "run", "--dry-run", "--json"]));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(!sandbox.path().join("state/vpt").exists());
    assert!(!sandbox.path().join("home/.vpt/audio").exists());
    assert!(!sandbox.fake_log().exists());
}

#[test]
fn an_expired_clone_with_replaced_bytes_is_kept_without_an_intent() {
    let (sandbox, _, clone) = ingested("retention-replaced", ENABLED);
    std::fs::write(&clone, b"replacement").expect("replacement");
    sandbox.set_mtime(&clone, 2 * 86_400);
    let output = run(sandbox.vpt().args(["retention", "run", "--json"]));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let document: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("json");
    assert_eq!(document["moved"], serde_json::json!([]));
    assert_eq!(document["kept"], serde_json::json!([{"path":clone,"reason":"digest_mismatch"}]));
    assert_eq!(std::fs::read(&clone).expect("preserved"), b"replacement");
    let count: i64 = sandbox.ledger().query_row(
        "SELECT COUNT(*) FROM retention_intents", [], |row| row.get(0),
    ).expect("count");
    assert_eq!(count, 0);
    assert!(!sandbox.fake_log().exists());
}

#[test]
fn disabled_absent_and_unvalidated_helpers_refuse_before_moves() {
    let (sandbox, _, _) = ingested("retention-disabled", "");
    assert_eq!(run(sandbox.vpt().args(["retention", "run"])).status.code(), Some(2));
    let (sandbox, _, clone) = ingested("retention-helper", ENABLED);
    sandbox.set_mtime(&clone, 2 * 86_400);
    for (version, code, rule) in [("2.0.0", 3, Some("helper_version")), ("bad", 1, None)] {
        let output = run(sandbox.vpt().args(["retention", "run", "--json"]).env("VPT_FAKE_VERSION", version));
        assert_eq!(output.status.code(), Some(code), "{}", stderr(&output));
        let document: serde_json::Value = serde_json::from_str(&stderr(&output)).expect("json");
        assert_eq!(document["error"]["rule"], serde_json::json!(rule));
        assert!(clone.exists());
    }
    std::fs::rename(sandbox.path().join("bin/vpt-macos"), sandbox.path().join("bin/helper-disabled")).expect("move fixture link");
    let output = run(sandbox.vpt().args(["retention", "run", "--json"]));
    assert_eq!(output.status.code(), Some(3));
    let document: serde_json::Value = serde_json::from_str(&stderr(&output)).expect("json");
    assert_eq!(document["error"]["rule"], "no_trash");
    assert!(clone.exists());
}
#[test]
fn retention_does_not_open_the_voice_memos_source() {
    let (sandbox, _, clone) = ingested("retention-without-source", ENABLED);
    sandbox.set_mtime(&clone, 2 * 86_400);
    std::fs::rename(sandbox.path().join("voice-memos"), sandbox.path().join("held-source")).expect("hide source");
    let before = support::state_snapshot(&sandbox);
    let dry = run(sandbox.vpt().args(["retention", "run", "--dry-run", "--json"]));
    assert_eq!(dry.status.code(), Some(0), "{}", stderr(&dry));
    let document: serde_json::Value = serde_json::from_str(&stdout(&dry)).expect("proposal");
    assert_eq!(document["moved"][0]["path"], clone.to_string_lossy().as_ref());
    assert_eq!(support::state_snapshot(&sandbox), before);
    assert!(!sandbox.fake_log().exists());
    let moved = run(sandbox.vpt().args(["retention", "run", "--json"]));
    assert_eq!(moved.status.code(), Some(0), "{}", stderr(&moved));
    assert!(!clone.exists());
}

```

`crates/vpt/tests/retention_startup.rs`:

```rust
mod support;
use support::{Sandbox, run, stderr, stdout};
use vpt_domain::fixtures::m4a;

const ENABLED: &str = "[retention]\nenabled = true\ninclude_audio = true\n[retention.hold]\naudio = \"1d\"\n";
fn fixture(name: &str) -> (Sandbox, Vec<String>) {
    let sandbox = Sandbox::new(name);
    sandbox.install_fake_helper();
    sandbox.write_config(ENABLED);
    sandbox.add_recording("a.m4a", &m4a(1_787_604_456, 3, b"first"));
    sandbox.add_recording("b.m4a", &m4a(1_787_604_456, 3, b"second"));
    let output = run(sandbox.vpt().args(["ingest", "--json"]));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let document: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("json");
    let ids = document["ingested"].as_array().expect("records").iter()
        .map(|row| row["id"].as_str().expect("id").to_owned()).collect::<Vec<_>>();
    let text = std::fs::read_to_string(sandbox.config_path()).expect("config")
        .replace("mode = \"off\"", "mode = \"desktop\"");
    std::fs::write(sandbox.config_path(), text).expect("notify config");
    (sandbox, ids)
}
fn seed(sandbox: &Sandbox, id: &str) {
    sandbox.ledger().execute(
        "INSERT INTO retention_intents (artifact_kind,recording,path,expected,recorded_at)
         SELECT 'audio',id,audio_path,digest,1 FROM recordings WHERE id=?1", [id],
    ).expect("pending intent");
}
fn clone_path(sandbox: &Sandbox, id: &str) -> std::path::PathBuf {
    sandbox.path().join(format!("home/.vpt/audio/{id}.m4a"))
}
fn notifications(sandbox: &Sandbox) -> usize {
    std::fs::read_to_string(sandbox.fake_log()).unwrap_or_default().lines()
        .filter(|line| line.starts_with("notify\t")).count()
}

#[test]
fn retention_combines_recovery_and_new_moves_in_one_report_and_event() {
    let (sandbox, ids) = fixture("retention-recovery-event");
    seed(&sandbox, &ids[0]);
    sandbox.set_mtime(&clone_path(&sandbox, &ids[1]), 2 * 86_400);
    let output = run(sandbox.vpt().args(["retention", "run", "--json"]));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let document: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("json");
    assert_eq!(document["moved"].as_array().expect("moves").len(), 2);
    assert_eq!(notifications(&sandbox), 1);
    let log = std::fs::read_to_string(sandbox.fake_log()).expect("log");
    assert!(log.contains("2 artifacts moved to the Trash"), "{log}");
}

#[test]
fn partial_startup_retains_completed_ids_and_one_event_on_retention_and_ingest() {
    for verb in ["retention", "ingest"] {
        let (sandbox, ids) = fixture(&format!("partial-{verb}"));
        seed(&sandbox, &ids[0]);
        seed(&sandbox, &ids[1]);
        std::fs::write(clone_path(&sandbox, &ids[1]), b"replacement").expect("replacement");
        let mut command = sandbox.vpt();
        command.arg(verb);
        if verb == "retention" { command.arg("run"); }
        let output = run(command.arg("--json"));
        assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
        assert!(stdout(&output).is_empty());
        let document: serde_json::Value = serde_json::from_str(&stderr(&output)).expect("json");
        assert_eq!(document["error"]["rule"], "retention_target_modified");
        assert_eq!(document["error"]["completed"], serde_json::json!([ids[0]]));
        assert!(!clone_path(&sandbox, &ids[0]).exists());
        assert!(clone_path(&sandbox, &ids[1]).exists());
        assert_eq!(notifications(&sandbox), 1);
    }
}

#[test]
fn a_ledger_failure_after_the_move_still_reports_its_recording_id() {
    let (sandbox, ids) = fixture("retention-row-failure");
    sandbox.set_mtime(&clone_path(&sandbox, &ids[0]), 2 * 86_400);
    sandbox.set_mtime(&clone_path(&sandbox, &ids[1]), 0);
    sandbox.ledger().execute_batch(
        "CREATE TRIGGER fail_trashed BEFORE UPDATE OF audio_trashed_at ON recordings
         BEGIN SELECT RAISE(FAIL,'injected row failure'); END;",
    ).expect("failure trigger");
    let output = run(sandbox.vpt().args(["retention", "run", "--json"]));
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    let document: serde_json::Value = serde_json::from_str(&stderr(&output)).expect("json");
    assert_eq!(document["error"]["completed"], serde_json::json!([ids[0]]));
    assert!(!clone_path(&sandbox, &ids[0]).exists());
    assert_eq!(notifications(&sandbox), 1);
    let pending: i64 = sandbox.ledger().query_row(
        "SELECT COUNT(*) FROM retention_intents WHERE completed_at IS NULL", [], |row| row.get(0),
    ).expect("pending");
    assert_eq!(pending, 1);
}

#[test]
fn incompatible_helper_blocks_reconciliation_without_a_trash_request() {
    let (sandbox, ids) = fixture("retention-recovery-version");
    seed(&sandbox, &ids[0]);
    let output = run(sandbox.vpt().args(["ingest", "--json"]).env("VPT_FAKE_VERSION", "2.0.0"));
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    let document: serde_json::Value = serde_json::from_str(&stderr(&output)).expect("json");
    assert_eq!(document["error"]["rule"], "helper_version");
    assert!(clone_path(&sandbox, &ids[0]).exists());
    assert!(!sandbox.fake_log().exists());
}

#[test]
fn dry_run_preserves_pending_intents_and_emits_no_event() {
    let (sandbox, ids) = fixture("retention-pending-dry");
    seed(&sandbox, &ids[0]);
    sandbox.set_mtime(&clone_path(&sandbox, &ids[0]), 2 * 86_400);
    let output = run(sandbox.vpt().args(["retention", "run", "--dry-run", "--json"]));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(clone_path(&sandbox, &ids[0]).exists());
    let pending: i64 = sandbox.ledger().query_row(
        "SELECT COUNT(*) FROM retention_intents WHERE completed_at IS NULL", [], |row| row.get(0),
    ).expect("pending");
    assert_eq!(pending, 1);
    assert!(!sandbox.fake_log().exists());
}
```

Support addition, using the same fixed clock Task 27 gives every child:

```rust
impl Sandbox {
    pub fn set_mtime(&self, path: &Path, seconds_ago: u64) {
        let seconds = u64::try_from(FIXED_NOW_SECS).expect("positive fixture clock")
            .checked_sub(seconds_ago).expect("fixture mtime after epoch");
        let when = std::time::UNIX_EPOCH + std::time::Duration::from_secs(seconds);
        let file = std::fs::File::options().write(true).open(path).expect("fixture file");
        file.set_times(std::fs::FileTimes::new().set_modified(when)).expect("fixture mtime");
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run separately:

```bash
cargo test -p vpt-adapters --test retention_reconcile
cargo test -p vpt --features dev-tools --test retention --test retention_startup
```

Expected: missing retention exports cause the new test modules to fail compilation; once connected, the
new command cases fail behaviorally. Zero selected tests or a successful command does not satisfy the red
step. Record the leaf names selected.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-application/src/retention/report.rs`:

```rust
use crate::ports::{LedgerError, StoreError, TrashError};
use std::path::PathBuf;
use vpt_domain::identity::RecordingId;
use vpt_domain::layout::StoreKey;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Moved {
    pub recording: RecordingId,
    pub store: StoreKey,
    pub path: PathBuf,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeptReason { Untracked, DigestMismatch }
impl KeptReason {
    pub fn as_str(self) -> &'static str {
        match self { Self::Untracked => "untracked", Self::DigestMismatch => "digest_mismatch" }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kept { pub path: PathBuf, pub reason: KeptReason }

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ReconcileProgress {
    pub moved: Vec<Moved>,
    pub completed: Vec<RecordingId>,
}
impl ReconcileProgress {
    pub(super) fn complete(&mut self, id: &RecordingId) {
        if !self.completed.contains(id) { self.completed.push(id.clone()); }
    }
    pub(super) fn failure(&self, error: RetentionError) -> ReconcileFailure {
        ReconcileFailure { error, progress: self.clone() }
    }
}
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RetentionReport {
    pub moved: Vec<Moved>,
    pub kept: Vec<Kept>,
    pub completed: Vec<RecordingId>,
}
impl From<ReconcileProgress> for RetentionReport {
    fn from(progress: ReconcileProgress) -> Self {
        Self { moved: progress.moved, kept: vec![], completed: progress.completed }
    }
}
impl RetentionReport {
    pub(super) fn failure(&self, error: RetentionError) -> RetentionFailure {
        RetentionFailure { error, report: self.clone() }
    }
    pub(super) fn moved(&mut self, item: Moved, dry_run: bool) {
        if !dry_run && !self.completed.contains(&item.recording) {
            self.completed.push(item.recording.clone());
        }
        self.moved.push(item);
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetentionError {
    Disabled,
    TargetModified(PathBuf),
    Ledger(LedgerError),
    Stores(StoreError),
    Trash(TrashError),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconcileFailure {
    pub error: RetentionError,
    pub progress: ReconcileProgress,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionFailure {
    pub error: RetentionError,
    pub report: RetentionReport,
}
```

`crates/vpt-application/src/retention/reconcile.rs`:

```rust
use super::{Moved, ReconcileFailure, ReconcileProgress, RetentionError};
use crate::ports::{Clock, LedgerError, RecordingLedger, RetentionJournal, StoreError, Stores, Trash};
use std::path::Path;
use vpt_domain::layout::StoreKey;

pub fn reconcile_intents<J, L, S, T, C>(
    journal: &J, ledger: &L, stores: &S, trash: &T, clock: &C, audio: &Path,
) -> Result<ReconcileProgress, ReconcileFailure>
where
    J: RetentionJournal, L: RecordingLedger, S: Stores, T: Trash, C: Clock,
{
    let mut progress = ReconcileProgress::default();
    let intents = journal.pending_intents().map_err(|e| progress.failure(RetentionError::Ledger(e)))?;
    for intent in intents {
        if intent.path.parent() != Some(audio) {
            return Err(progress.failure(RetentionError::Stores(StoreError::Escape(intent.path))));
        }
        let record = ledger.by_id(&intent.recording)
            .map_err(|e| progress.failure(RetentionError::Ledger(e)))?
            .ok_or_else(|| progress.failure(RetentionError::Ledger(
                LedgerError::Corrupt("pending retention intent has no recording".into()),
            )))?;
        if record.audio_path != intent.path || record.digest != intent.expected {
            return Err(progress.failure(RetentionError::TargetModified(intent.path)));
        }
        stores.validate_path(&intent.path).map_err(|e| progress.failure(RetentionError::Stores(e)))?;
        match stores.digest_of(&intent.path).map_err(|e| progress.failure(RetentionError::Stores(e)))? {
            None => {}
            Some(digest) if digest == intent.expected => {
                stores.validate_path(&intent.path).map_err(|e| progress.failure(RetentionError::Stores(e)))?;
                trash.trash(&intent.path).map_err(|e| progress.failure(RetentionError::Trash(e)))?;
                progress.moved.push(Moved {
                    recording: intent.recording.clone(), store: StoreKey::Audio, path: intent.path.clone(),
                });
                progress.complete(&intent.recording);
            }
            Some(_) => return Err(progress.failure(RetentionError::TargetModified(intent.path))),
        }
        let now = clock.now();
        ledger.set_audio_trashed(&intent.recording, now)
            .map_err(|e| progress.failure(RetentionError::Ledger(e)))?;
        progress.complete(&intent.recording);
        journal.complete_intent(intent.id, now).map_err(|e| progress.failure(RetentionError::Ledger(e)))?;
    }
    Ok(progress)
}
```

`crates/vpt-application/src/retention/mod.rs`:

```rust
mod reconcile;
mod report;
pub use reconcile::reconcile_intents;
pub use report::{
    Kept, KeptReason, Moved, ReconcileFailure, ReconcileProgress, RetentionError,
    RetentionFailure, RetentionReport,
};

use crate::ports::{
    ArtifactKind, Clock, NewIntent, RecordingLedger, RecordingRecord, RetentionJournal,
    StoreEntry, StoreError, Stores, Trash,
};
use crate::{RetentionSettings, StorePaths};
use vpt_domain::layout::StoreKey;
use vpt_domain::retention::expired;

pub struct Retention<'a, L, J, S, T, C> {
    pub ledger: &'a L,
    pub journal: &'a J,
    pub stores: &'a S,
    pub trash: &'a T,
    pub clock: &'a C,
    pub settings: &'a RetentionSettings,
    pub paths: &'a StorePaths,
}
impl<L, J, S, T, C> Retention<'_, L, J, S, T, C>
where
    L: RecordingLedger, J: RetentionJournal, S: Stores, T: Trash, C: Clock,
{
    pub fn run(&self, dry_run: bool, initial: ReconcileProgress) -> Result<RetentionReport, RetentionFailure> {
        let mut report = RetentionReport::from(initial);
        if !self.settings.enabled { return Err(report.failure(RetentionError::Disabled)); }
        let owned = self.ledger.recordings().map_err(|e| report.failure(RetentionError::Ledger(e)))?;
        for key in StoreKey::all() {
            if key == StoreKey::Audio && !self.settings.include_audio { continue; }
            let hold = self.settings.hold(key);
            if hold.seconds() == 0 { continue; }
            let entries = self.stores.entries(self.paths.get(key))
                .map_err(|e| report.failure(RetentionError::Stores(e)))?;
            for entry in entries {
                let record = owned.iter().find(|record| {
                    key == StoreKey::Audio && record.audio_path == entry.path && record.audio_trashed_at.is_none()
                });
                let Some(record) = record else {
                    report.kept.push(Kept { path: entry.path, reason: KeptReason::Untracked });
                    continue;
                };
                if expired(entry.mtime, hold, self.clock.now()) {
                    self.expire(key, &entry, record, dry_run, &mut report)?;
                }
            }
        }
        Ok(report)
    }

    fn expire(
        &self, key: StoreKey, entry: &StoreEntry, record: &RecordingRecord,
        dry_run: bool, report: &mut RetentionReport,
    ) -> Result<(), RetentionFailure> {
        if entry.path.parent() != Some(self.paths.get(key)) {
            return Err(report.failure(RetentionError::Stores(StoreError::Escape(entry.path.clone()))));
        }
        self.stores.validate_path(&entry.path).map_err(|e| report.failure(RetentionError::Stores(e)))?;
        if self.stores.digest_of(&entry.path).map_err(|e| report.failure(RetentionError::Stores(e)))? != Some(record.digest) {
            report.kept.push(Kept { path: entry.path.clone(), reason: KeptReason::DigestMismatch });
            return Ok(());
        }
        let moved = Moved { recording: record.id.clone(), store: key, path: entry.path.clone() };
        if dry_run {
            report.moved(moved, true);
            return Ok(());
        }
        let now = self.clock.now();
        let intent = NewIntent {
            kind: ArtifactKind::Audio, recording: record.id.clone(), path: entry.path.clone(),
            expected: record.digest, recorded_at: now,
        };
        let id = self.journal.record_intent(&intent).map_err(|e| report.failure(RetentionError::Ledger(e)))?;
        self.stores.validate_path(&entry.path).map_err(|e| report.failure(RetentionError::Stores(e)))?;
        self.trash.trash(&entry.path).map_err(|e| report.failure(RetentionError::Trash(e)))?;
        report.moved(moved, false);
        self.ledger.set_audio_trashed(&record.id, now).map_err(|e| report.failure(RetentionError::Ledger(e)))?;
        self.journal.complete_intent(id, now).map_err(|e| report.failure(RetentionError::Ledger(e)))?;
        Ok(())
    }
}
```

Application `lib.rs` retains a private `mod retention;` and re-exports only:

```rust
pub use retention::{
    Kept, KeptReason, Moved, ReconcileFailure, ReconcileProgress, Retention,
    RetentionError, RetentionFailure, RetentionReport, reconcile_intents,
};
```

`Stores::validate_path` is Task 14's checked-root operation: it reopens the canonical root with
`O_NOFOLLOW_ANY`, compares device and inode to its held descriptor, and checks the leaf without following
a symbolic link. A journal path is checked against the audio root and the recording's pinned path before
any content open, and checked again immediately before the helper call.

Replace `Runtime::mutating` in `compose/runtime.rs` and remove its now-unused `RepairReport` import:

```rust
    pub(crate) fn mutating(&self) -> Result<ReconcileProgress, Box<StartupFailure>> {
        if self._lock.is_none() {
            return Err(Box::new(StartupFailure {
                error: ErrorDocument::new(ErrorKind::Ledger, "mutating startup requires the write lock"),
                progress: ReconcileProgress::default(),
            }));
        }
        repair_publications(&self.ledger, &self.stores, |_| None).map_err(|error| StartupFailure {
            error: repair_error(error), progress: ReconcileProgress::default(),
        })?;
        reconcile_intents(
            &self.ledger, &self.ledger, &self.stores, &self.helper, &self.clock, &self.roots.stores.audio,
        ).map_err(|failure| Box::new(StartupFailure {
            error: retention_error(failure.error), progress: failure.progress,
        }))
    }
```

Add the private composition module and curated imports to `compose.rs`:

```rust
mod recovery;
pub(crate) use recovery::{StartupFailure, retention_error};
```

In `compose/runtime.rs`, add `use super::{StartupFailure, retention_error};` and replace its application
import with:

```rust
use vpt_application::{ReconcileProgress, reconcile_intents, repair_publications};
```

`crates/vpt/src/compose/recovery.rs`:

```rust
use super::Runtime;
use crate::cli::output::Outcome;
use serde_json::json;
use vpt_application::ports::{Clock, StoreError, TrashError};
use vpt_application::{Moved, ReconcileProgress, RetentionError};
use vpt_domain::notification::{EventKind, Notification};
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(crate) struct StartupFailure {
    pub error: ErrorDocument,
    pub progress: ReconcileProgress,
}

pub(crate) fn retention_error(error: RetentionError) -> ErrorDocument {
    match error {
        RetentionError::Disabled => ErrorDocument::new(ErrorKind::Usage, "retention.enabled is false"),
        RetentionError::TargetModified(path) => ErrorDocument::new(
            ErrorKind::Refused, format!("{} changed since its retention intent was recorded", path.display()),
        ).rule("retention_target_modified"),
        RetentionError::Ledger(error) => crate::compose::errors::ledger_error(error),
        RetentionError::Stores(StoreError::Escape(path)) => ErrorDocument::new(
            ErrorKind::Refused, format!("{} escapes its store root", path.display()),
        ).rule("path_escape"),
        RetentionError::Stores(StoreError::Io(detail)) => ErrorDocument::new(ErrorKind::Store, detail),
        RetentionError::Trash(TrashError::HelperAbsent) => ErrorDocument::new(
            ErrorKind::Refused, "the helper is absent, so nothing can reach the Trash",
        ).rule("no_trash"),
        RetentionError::Trash(TrashError::HelperVersion { found }) => ErrorDocument::new(
            ErrorKind::Refused, format!("the helper is major version {found}; this build needs 1"),
        ).rule("helper_version"),
        RetentionError::Trash(error) => ErrorDocument::new(ErrorKind::Helper, format!("{error:?}")),
    }
}
impl Runtime {
    pub(crate) fn retention_notice(&self, moved: &[Moved]) {
        if moved.is_empty() { return; }
        let mut counts: Vec<(String, u64)> = vec![];
        for item in moved {
            if let Some((_, count)) = counts.iter_mut().find(|(key, _)| key == item.store.key_name()) {
                *count += 1;
            } else {
                counts.push((item.store.key_name().into(), 1));
            }
        }
        self.notifier.deliver(&Notification::done(
            EventKind::Retention, format!("{} artifacts moved to the Trash", moved.len()), counts, self.clock.now(),
        ));
    }

    pub(crate) fn finish_recovery(&self, mut outcome: Outcome, progress: &ReconcileProgress) -> Outcome {
        self.retention_notice(&progress.moved);
        let lines: Vec<String> = progress.moved.iter()
            .map(|item| format!("moved {} ({})", item.path.display(), item.store.key_name())).collect();
        match &mut outcome {
            Outcome::Success { human, document } => {
                for line in &lines { human.push_str(line); human.push('\n'); }
                if !lines.is_empty() {
                    human.push_str(&format!("retention moved {}\n", lines.len()));
                    document["retention_moved"] = json!(progress.moved.iter().map(|item| {
                        json!({"store":item.store.key_name(),"path":item.path})
                    }).collect::<Vec<_>>());
                }
                let diagnostics = self.diagnostics();
                for line in &diagnostics { human.push_str(&format!("note: {line}\n")); }
                if !diagnostics.is_empty() { document["diagnostics"] = json!(diagnostics); }
            }
            Outcome::Failure(error) | Outcome::FailedReport { error, .. } => {
                for id in &progress.completed {
                    if !error.completed.iter().any(|known| known == id.as_str()) {
                        error.completed.push(id.as_str().to_owned());
                    }
                }
                error.diagnostics.extend(lines);
                error.diagnostics.extend(self.diagnostics());
            }
        }
        outcome
    }
}
```

Replace only `run` in `crates/vpt/src/commands/ingest.rs`; retain its Task 27 output functions and add
`use vpt_application::ReconcileProgress;`:

```rust
pub fn run(runtime: &Runtime, dry_run: bool, once: Option<PathBuf>) -> Outcome {
    let Some(recorder) = runtime.recorder.as_ref() else {
        return Outcome::Failure(ErrorDocument::new(ErrorKind::Config, "ingest composition has no recording source"));
    };
    let progress = if dry_run {
        ReconcileProgress::default()
    } else {
        match runtime.mutating() {
            Ok(progress) => progress,
            Err(failure) => return runtime.finish_recovery(Outcome::Failure(failure.error), &failure.progress),
        }
    };
    let ingest = Ingest {
        recorder,
        archive: &runtime.archive,
        ledger: &runtime.ledger,
        clock: &runtime.clock,
        trash: runtime.trash(),
        notifier: runtime.notifier.as_ref(),
        settings: &runtime.settings.source,
    };
    let mode = Mode { dry_run, once };
    let outcome = match ingest.run(&mode) {
        Ok(mut report) => {
            report.log.extend(runtime.diagnostics());
            Outcome::Success { human: human(&report), document: document("ingest", body(&report)) }
        }
        Err(mut error) => {
            error.log.extend(runtime.diagnostics());
            Outcome::Failure(failure(*error))
        }
    };
    runtime.finish_recovery(outcome, &progress)
}
```

`crates/vpt/src/commands/retention.rs`:

```rust
use crate::cli::output::Outcome;
use crate::compose::{Runtime, retention_error};
use serde_json::json;
use vpt_adapters::{BUILT_AGAINST_MAJOR, HelperError};
use vpt_application::{ReconcileProgress, Retention, RetentionReport};
use vpt_protocol::error::{ErrorDocument, ErrorKind};
use vpt_protocol::result::document;

pub(crate) fn run(runtime: &Runtime, dry_run: bool) -> Outcome {
    if !runtime.settings.retention.enabled {
        return Outcome::Failure(ErrorDocument::new(ErrorKind::Usage, "retention.enabled is false"));
    }
    let preflight = match runtime.helper.version() {
        Ok(_) => Ok(()),
        Err(HelperError::Absent) => Err(ErrorDocument::new(
            ErrorKind::Refused, "the helper is absent, so nothing can reach the Trash",
        ).rule("no_trash")),
        Err(HelperError::MajorMismatch { found }) => Err(ErrorDocument::new(
            ErrorKind::Refused, format!("the helper is major version {found}; this build needs {BUILT_AGAINST_MAJOR}"),
        ).rule("helper_version")),
        Err(_) => Err(ErrorDocument::new(ErrorKind::Helper, "the helper version could not be validated")),
    };
    if let Err(error) = preflight {
        return Outcome::Failure(error.diagnostics(runtime.diagnostics()));
    }
    let initial = if dry_run {
        ReconcileProgress::default()
    } else {
        match runtime.mutating() {
            Ok(progress) => progress,
            Err(failure) => return runtime.finish_recovery(Outcome::Failure(failure.error), &failure.progress),
        }
    };
    let retention = Retention {
        ledger: &runtime.ledger, journal: &runtime.ledger, stores: &runtime.stores,
        trash: &runtime.helper, clock: &runtime.clock,
        settings: &runtime.settings.retention, paths: &runtime.roots.stores,
    };
    match retention.run(dry_run, initial) {
        Ok(report) => finish(runtime, report, None, dry_run),
        Err(failure) => finish(runtime, failure.report, Some(retention_error(failure.error)), dry_run),
    }
}

fn finish(runtime: &Runtime, report: RetentionReport, error: Option<ErrorDocument>, dry_run: bool) -> Outcome {
    if !dry_run { runtime.retention_notice(&report.moved); }
    let mut diagnostics = runtime.diagnostics();
    match error {
        Some(error) => {
            diagnostics.extend(report.moved.iter().map(|item| format!("moved {} ({})", item.path.display(), item.store.key_name())));
            diagnostics.push(format!("moved {}, kept {}", report.moved.len(), report.kept.len()));
            Outcome::Failure(error.completed(report.completed.iter().map(|id| id.as_str().to_owned()).collect()).diagnostics(diagnostics))
        }
        None => {
            let mut human = human(&report, dry_run);
            for line in &diagnostics { human.push_str(&format!("note: {line}\n")); }
            let mut body = body(&report);
            if !diagnostics.is_empty() { body["diagnostics"] = json!(diagnostics); }
            Outcome::Success { human, document: document("retention run", body) }
        }
    }
}
fn body(report: &RetentionReport) -> serde_json::Value {
    json!({
        "moved": report.moved.iter().map(|item| json!({"store":item.store.key_name(),"path":item.path})).collect::<Vec<_>>(),
        "kept": report.kept.iter().map(|item| json!({"path":item.path,"reason":item.reason.as_str()})).collect::<Vec<_>>(),
    })
}
fn human(report: &RetentionReport, dry_run: bool) -> String {
    let verb = if dry_run { "would move" } else { "moved" };
    let mut lines: Vec<String> = report.moved.iter().map(|item| format!("{verb} {} ({})", item.path.display(), item.store.key_name())).collect();
    lines.extend(report.kept.iter().map(|item| format!("kept {} ({})", item.path.display(), item.reason.as_str())));
    lines.push(format!("{verb} {}, kept {}", report.moved.len(), report.kept.len()));
    lines.join("\n") + "\n"
}
```

Dispatch in `crates/vpt/src/lib.rs`:

```rust
        Verb::RetentionRun { dry_run } => {
            let access = if *dry_run { AccessMode::ReadOnly } else { AccessMode::Mutating };
            match Runtime::load(&environment, invocation.config.as_deref(), access, Operation::Retention) {
                Ok(runtime) => commands::retention::run(&runtime, *dry_run),
                Err(error) => Outcome::Failure(*error),
            }
        }
```

Add `pub(crate) mod retention;` to `commands/mod.rs`. The runtime's private lock remains owned until the
command returns; retention obtains no second lock. A helper version failure ends the command before
reconciliation or new moves. The helper also validates its version on its first Trash call, covering
recovery entered through ingest.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo fmt --all`

```bash
cargo test -p vpt-adapters --test retention_reconcile
cargo test -p vpt --features dev-tools --test retention --test retention_startup --test ingest
cargo test --workspace --features dev-tools
cargo clippy --workspace --all-targets --features dev-tools -- -D warnings
just file-size
```

Expected: the named recovery cases, command cases and ingest regressions pass; no warnings and no file
over its hard limit. Record test names and measured results, not a predicted count.

- [ ] **Step 5: Commit**

Run: `cargo fmt --all`

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(retention): journal Trash moves and retain partial recovery results"
```

______________________________________________________________________

### Task 33: The `vpt-macos` helper package: `--version`, `notify`, `trash`

The Swift package behind `[helper] path`. Stage 1 ships the three subcommands vpt calls now; `transcribe`
arrives with stage 2. The notification poster and the Trash sit behind protocols so the suite reaches no
real destination: the poster is a recording stub and the Trash is a temporary-directory adapter, as spec
section 12 requires. Swift production files stay under 200 lines and test files under 700, the clean-code
Swift rule, enforced by the same `file-size` recipe.

**Files:**

- Create: `helper/vpt-macos/Package.swift`
- Create: `helper/vpt-macos/Sources/VptMacos/Arguments.swift`, `Documents.swift`, `Notify.swift`,
  `Trash.swift`, `Run.swift`
- Create: `helper/vpt-macos/Sources/vpt-macos/main.swift`
- Create: `helper/vpt-macos/Tests/VptMacosTests/ArgumentsTests.swift`, `RunTests.swift`
- Modify: `justfile`, `.github/workflows/ci.yml`, `.gitignore`

**Interfaces:**

- Consumes: nothing from the Rust side; the Rust client of Task 25 reads what this prints.

- Produces (module `VptMacos`): `Command { case version, notify(title:body:), trash(path:) }`,
  `UsageError { message }`, `usage: String`,
  `parse(_ arguments: [String]) -> Result<Command, UsageError>`, `VersionDocument`, `PostedDocument`,
  `TrashedDocument`, `encode<Document: Encodable>(_:) -> String`,
  `protocol NotificationPoster { func post(title: String, body: String) throws }`, `AppleScriptPoster`,
  `escaped(_:) -> String`, `protocol Trasher { func trash(_ url: URL) throws }`, `FileManagerTrasher`,
  `Outcome { stdout, stderr, exitCode }`,
  `run(_ command: Command, poster: NotificationPoster, trasher: Trasher) -> Outcome`.

- Exit codes: 0 done, 1 the poster or the Trash failed, 2 usage.

- [ ] **Step 1: Write the failing tests**

`helper/vpt-macos/Package.swift` (the tests need the package before they can fail to compile):

```swift
// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "vpt-macos",
    platforms: [.macOS(.v15)],
    targets: [
        .target(name: "VptMacos"),
        .executableTarget(name: "vpt-macos", dependencies: ["VptMacos"]),
        .testTarget(name: "VptMacosTests", dependencies: ["VptMacos"])
    ]
)
```

Create both source targets before the red run so Swift Package Manager reaches the test compiler.
`helper/vpt-macos/Sources/VptMacos/Arguments.swift` starts with:

```swift
import Foundation
```

`helper/vpt-macos/Sources/vpt-macos/main.swift` starts with:

```swift
import VptMacos
```

Step 3 replaces both files in full.

`helper/vpt-macos/Tests/VptMacosTests/ArgumentsTests.swift`:

```swift
@testable import VptMacos
import XCTest

final class ArgumentsTests: XCTestCase {
    func testVersionNotifyAndTrashParse() {
        XCTAssertEqual(parse(["--version"]), .success(.version))
        XCTAssertEqual(
            parse(["notify", "--title", "t", "--body", "b"]),
            .success(.notify(title: "t", body: "b"))
        )
        XCTAssertEqual(
            parse(["notify", "--body", "b", "--title", "t"]),
            .success(.notify(title: "t", body: "b"))
        )
        XCTAssertEqual(parse(["trash", "/tmp/x"]), .success(.trash(path: "/tmp/x")))
    }

    func testUnknownAndIncompleteArgumentsAreUsageErrors() {
        XCTAssertEqual(parse([]), .failure(UsageError(message: "unknown subcommand")))
        XCTAssertEqual(
            parse(["transcribe", "x"]),
            .failure(UsageError(message: "unknown subcommand"))
        )
        XCTAssertEqual(
            parse(["--version", "extra"]),
            .failure(UsageError(message: "--version takes no arguments"))
        )
        XCTAssertEqual(
            parse(["notify", "--title"]),
            .failure(UsageError(message: "--title needs a value"))
        )
        XCTAssertEqual(
            parse(["notify", "--title", "t"]),
            .failure(UsageError(message: "notify needs --title and --body"))
        )
        XCTAssertEqual(
            parse(["trash"]),
            .failure(UsageError(message: "trash takes exactly one path"))
        )
    }
}
```

`helper/vpt-macos/Tests/VptMacosTests/RunTests.swift`:

```swift
@testable import VptMacos
import XCTest

final class RecordingPoster: NotificationPoster {
    var posted: [(title: String, body: String)] = []
    var failure: PostFailure?

    func post(title: String, body: String) throws {
        if let failure {
            throw failure
        }
        posted.append((title, body))
    }
}

final class TemporaryTrasher: Trasher {
    let directory: URL

    init(directory: URL) {
        self.directory = directory
    }

    func trash(_ url: URL) throws {
        try FileManager.default.moveItem(
            at: url,
            to: directory.appendingPathComponent(url.lastPathComponent)
        )
    }
}

final class RunTests: XCTestCase {
    private func scratch() throws -> URL {
        let base = FileManager.default.temporaryDirectory
            .appendingPathComponent("vpt-macos-\(UUID().uuidString)")
        try FileManager.default.createDirectory(
            at: base.appendingPathComponent("trash"),
            withIntermediateDirectories: true
        )
        addTeardownBlock { try FileManager.default.removeItem(at: base) }
        return base
    }

    func testVersionPrintsTheHelperDocument() throws {
        let outcome = try VptMacos.run(
            .version,
            poster: RecordingPoster(),
            trasher: TemporaryTrasher(directory: scratch())
        )
        XCTAssertEqual(outcome.exitCode, 0)
        XCTAssertEqual(outcome.stdout, "{\"schema\":\"vpt.helper/1\",\"version\":\"1.0.0\"}")
    }

    func testNotifyPostsThroughTheProtocolAndPrintsPosted() throws {
        let poster = RecordingPoster()
        let outcome = try VptMacos.run(
            .notify(title: "vpt: deferred", body: "3 waiting"),
            poster: poster,
            trasher: TemporaryTrasher(directory: scratch())
        )
        XCTAssertEqual(outcome.exitCode, 0)
        XCTAssertEqual(outcome.stdout, "{\"posted\":true}")
        XCTAssertEqual(poster.posted.count, 1)
        XCTAssertEqual(poster.posted.first?.title, "vpt: deferred")
        XCTAssertEqual(poster.posted.first?.body, "3 waiting")
    }

    func testAFailedPostIsExit1WithNothingOnStdout() throws {
        let poster = RecordingPoster()
        poster.failure = PostFailure(detail: "no session")
        let outcome = try VptMacos.run(
            .notify(title: "t", body: "b"),
            poster: poster,
            trasher: TemporaryTrasher(directory: scratch())
        )
        XCTAssertEqual(outcome.exitCode, 1)
        XCTAssertEqual(outcome.stdout, "")
        XCTAssertTrue(outcome.stderr.contains("no session"), outcome.stderr)
    }

    func testTrashMovesThroughTheAdapterAndPrintsTheRequestedPath() throws {
        let base = try scratch()
        let victim = base.appendingPathComponent("victim.txt")
        try Data("bye".utf8).write(to: victim)
        let outcome = VptMacos.run(
            .trash(path: victim.path),
            poster: RecordingPoster(),
            trasher: TemporaryTrasher(directory: base.appendingPathComponent("trash"))
        )
        XCTAssertEqual(outcome.exitCode, 0)
        XCTAssertEqual(outcome.stdout, "{\"trashed\":\"\(victim.path)\"}")
        XCTAssertFalse(FileManager.default.fileExists(atPath: victim.path))
        XCTAssertTrue(FileManager.default
            .fileExists(atPath: base.appendingPathComponent("trash/victim.txt").path))
    }

    func testAFailedTrashIsExit1AndTheFileStays() throws {
        let base = try scratch()
        let victim = base.appendingPathComponent("victim.txt")
        try Data("bye".utf8).write(to: victim)
        let outcome = VptMacos.run(
            .trash(path: victim.path),
            poster: RecordingPoster(),
            trasher: TemporaryTrasher(directory: base.appendingPathComponent("missing"))
        )
        XCTAssertEqual(outcome.exitCode, 1)
        XCTAssertEqual(outcome.stdout, "")
        XCTAssertTrue(FileManager.default.fileExists(atPath: victim.path))
    }

    func testAppleScriptEscapingCoversQuotesAndBackslashes() {
        XCTAssertEqual(escaped("say \"hi\" \\ now"), "say \\\"hi\\\" \\\\ now")
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd helper/vpt-macos && swift test`

Expected: Swift discovers both source targets, then the test compilation fails with undefined `parse`,
`VptMacos.run`, `NotificationPoster`, `Trasher` and `PostFailure`. An empty-target error does not
establish this red step. The test calls qualify `VptMacos.run` because `XCTestCase` also defines `run`.

- [ ] **Step 3: Write the minimal implementation**

`helper/vpt-macos/Sources/VptMacos/Arguments.swift`:

```swift
/// The three subcommands, parsed from argv with no library.
public enum Command: Equatable {
    case version
    case notify(title: String, body: String)
    case trash(path: String)
}

public struct UsageError: Error, Equatable {
    public let message: String

    public init(message: String) {
        self.message = message
    }
}

public let usage = "usage: vpt-macos --version | notify --title <t> --body <b> | trash <path>"

public func parse(_ arguments: [String]) -> Result<Command, UsageError> {
    switch arguments.first {
    case "--version":
        return arguments
            .count == 1 ? .success(.version) :
            .failure(UsageError(message: "--version takes no arguments"))
    case "notify":
        return parseNotify(Array(arguments.dropFirst()))
    case "trash":
        guard arguments.count == 2
        else { return .failure(UsageError(message: "trash takes exactly one path")) }
        return .success(.trash(path: arguments[1]))
    default:
        return .failure(UsageError(message: "unknown subcommand"))
    }
}

private func parseNotify(_ words: [String]) -> Result<Command, UsageError> {
    var title: String?
    var body: String?
    var index = 0
    while index < words.count {
        guard index + 1 < words.count
        else { return .failure(UsageError(message: "\(words[index]) needs a value")) }
        switch words[index] {
        case "--title": title = words[index + 1]
        case "--body": body = words[index + 1]
        default: return .failure(UsageError(message: "unknown option \(words[index])"))
        }
        index += 2
    }
    guard let title,
          let body else { return .failure(UsageError(message: "notify needs --title and --body")) }
    return .success(.notify(title: title, body: body))
}
```

`helper/vpt-macos/Sources/VptMacos/Documents.swift`:

```swift
import Foundation

public let helperSchema = "vpt.helper/1"
public let helperVersion = "1.0.0"

public struct VersionDocument: Codable, Equatable {
    public var schema = helperSchema
    public var version = helperVersion

    public init() {}
}

public struct PostedDocument: Codable, Equatable {
    public let posted: Bool

    public init(posted: Bool) {
        self.posted = posted
    }
}

public struct TrashedDocument: Codable, Equatable {
    public let trashed: String

    public init(trashed: String) {
        self.trashed = trashed
    }
}

/// One line of JSON with sorted keys and no escaped slashes.
public func encode(_ document: some Encodable) -> String {
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
    guard let data = try? encoder.encode(document),
          let text = String(data: data, encoding: .utf8)
    else {
        return "{}"
    }
    return text
}
```

`helper/vpt-macos/Sources/VptMacos/Notify.swift`:

```swift
import Foundation

public protocol NotificationPoster {
    func post(title: String, body: String) throws
}

public struct PostFailure: Error, Equatable {
    public let detail: String

    public init(detail: String) {
        self.detail = detail
    }
}

/// `display notification` through NSAppleScript, in-process, no signed bundle.
public struct AppleScriptPoster: NotificationPoster {
    public init() {}

    public func post(title: String, body: String) throws {
        let source = "display notification \"\(escaped(body))\" with title \"\(escaped(title))\""
        guard let script = NSAppleScript(source: source) else {
            throw PostFailure(detail: "the script did not compile")
        }
        var error: NSDictionary?
        script.executeAndReturnError(&error)
        if let error {
            throw PostFailure(detail: error.description)
        }
    }
}

public func escaped(_ text: String) -> String {
    text.replacingOccurrences(of: "\\", with: "\\\\").replacingOccurrences(of: "\"", with: "\\\"")
}
```

`helper/vpt-macos/Sources/VptMacos/Trash.swift`:

```swift
import Foundation

public protocol Trasher {
    func trash(_ url: URL) throws
}

public struct TrashFailure: Error, Equatable {
    public let detail: String

    public init(detail: String) {
        self.detail = detail
    }
}

/// The system Trash through FileManager; nothing here unlinks.
public struct FileManagerTrasher: Trasher {
    public init() {}

    public func trash(_ url: URL) throws {
        do {
            try FileManager.default.trashItem(at: url, resultingItemURL: nil)
        } catch {
            throw TrashFailure(detail: error.localizedDescription)
        }
    }
}
```

`helper/vpt-macos/Sources/VptMacos/Run.swift`:

```swift
import Foundation

public struct Outcome: Equatable {
    public let stdout: String
    public let stderr: String
    public let exitCode: Int32

    public init(stdout: String, stderr: String, exitCode: Int32) {
        self.stdout = stdout
        self.stderr = stderr
        self.exitCode = exitCode
    }
}

public func run(_ command: Command, poster: NotificationPoster, trasher: Trasher) -> Outcome {
    switch command {
    case .version:
        return Outcome(stdout: encode(VersionDocument()), stderr: "", exitCode: 0)
    case let .notify(title, body):
        do {
            try poster.post(title: title, body: body)
            return Outcome(stdout: encode(PostedDocument(posted: true)), stderr: "", exitCode: 0)
        } catch {
            return Outcome(stdout: "", stderr: "vpt-macos: notify failed: \(error)", exitCode: 1)
        }
    case let .trash(path):
        do {
            try trasher.trash(URL(fileURLWithPath: path))
            return Outcome(stdout: encode(TrashedDocument(trashed: path)), stderr: "", exitCode: 0)
        } catch {
            return Outcome(stdout: "", stderr: "vpt-macos: trash failed: \(error)", exitCode: 1)
        }
    }
}
```

`helper/vpt-macos/Sources/vpt-macos/main.swift`:

```swift
import Foundation
import VptMacos

switch parse(Array(CommandLine.arguments.dropFirst())) {
case let .failure(error):
    FileHandle.standardError.write(Data("vpt-macos: \(error.message)\n\(usage)\n".utf8))
    exit(2)
case let .success(command):
    let outcome = run(command, poster: AppleScriptPoster(), trasher: FileManagerTrasher())
    if !outcome.stdout.isEmpty {
        print(outcome.stdout)
    }
    if !outcome.stderr.isEmpty {
        FileHandle.standardError.write(Data((outcome.stderr + "\n").utf8))
    }
    exit(outcome.exitCode)
}
```

`justfile` gains three recipes and the Swift sizes in `file-size`, and `ship` grows:

```just
swift-build:
  cd helper/vpt-macos && swift build -c release --explicit-target-dependency-import-check error

swift-test:
  cd helper/vpt-macos && swift test

# Operator-run: one real notification and one real move to the Trash.
smoke: swift-build
  #!/usr/bin/env bash
  set -euo pipefail
  helper=helper/vpt-macos/.build/release/vpt-macos
  "$helper" --version
  "$helper" notify --title "vpt smoke" --body "the helper can post"
  victim="$(mktemp "${TMPDIR:-/tmp}/vpt-smoke.XXXXXX")"
  "$helper" trash "$victim"

ship: fmt clippy test doc gitleaks file-size swift-build swift-test
```

Append to the `file-size` recipe body, before its `exit "$status"`:

```bash
  while IFS= read -r f; do
    total=$(wc -l < "$f")
    limit=200
    [[ "$f" == helper/vpt-macos/Tests/* ]] && limit=700
    if (( total > limit )); then
      printf 'FAIL %5d total (limit %d) %s\n' "$total" "$limit" "$f"
      status=1
    fi
  done < <(git ls-files 'helper/vpt-macos/Sources/*.swift' 'helper/vpt-macos/Sources/**/*.swift' 'helper/vpt-macos/Tests/**/*.swift')
```

`.github/workflows/ci.yml` gains two steps after `File size`:

```yaml
  - name: Swift build
    run: just swift-build
  - name: Swift test
    run: just swift-test
```

`.gitignore` gains `helper/vpt-macos/.build/`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo fmt --all`

Run from `helper/vpt-macos`:

```bash
swiftformat Sources Tests Package.swift --cache ignore --swift-version 6.0 --max-width 100 --trailing-commas never
swiftformat --lint Sources --cache ignore --swift-version 6.0 --max-width 100 --trailing-commas never
swiftformat --lint Tests --cache ignore --swift-version 6.0 --max-width 100 --trailing-commas never
swiftformat --lint Package.swift --cache ignore --swift-version 6.0 --max-width 100 --trailing-commas never
swiftlint lint Sources Tests Package.swift --no-cache
swift test --explicit-target-dependency-import-check error
swift build -c release --explicit-target-dependency-import-check error
.build/release/vpt-macos --version
```

Expected: 8 tests pass; the release build prints `{"schema":"vpt.helper/1","version":"1.0.0"}`.

Run: `just file-size`

Expected: no `FAIL` line; every Swift source is under 200 lines.

Run every gate in CI order before the helper commit:

Run: `just ship`

Expected: each stable gate passes. Record its command, exit code and selected leaf test names.

Run `cargo +nightly test --workspace --features dev-tools -- -Z unstable-options --report-time` using the
installed nightly. Record per-test durations and reject any test reaching one second. Run the required
stable suite separately. Do not infer duration from subprocess counts. If the installed nightly cannot
run this command, record the blocker and stop delivery until equivalent per-test measurements exist.

Record the file-size table:

Run: `just file-size 2>&1 | sort -k2 -n | tail -15`

Expected: the fifteen largest files, none over 500 total, and any `WARN` line listed in the pull request
with the split it would take. The files this plan expects nearest the warning line are
`crates/vpt-adapters/src/config/schema.rs` (the key table),
`crates/vpt-application/src/ingest/candidate.rs`, `crates/vpt/src/compose.rs`,
`crates/vpt/src/doctor/checks.rs` and the two ingest acceptance suites. A file that crosses 500 splits
along the seam its task named before the pull request opens.

Verify the mutants by hand and record the table:

For each row, apply the mutation to a scratch copy of the working tree, run the named test, confirm it
fails, then run the unmutated control and confirm it passes. The table goes in the pull request body.

| Behavior                              | Mutation                                                         | Test that must go red                                                                 |
| ------------------------------------- | ---------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| the wholeness gate                    | `inspect` returns `Ok` when `moov` is missing                    | `a_file_without_moov_is_refused`                                                      |
| the rest gate                         | `>=` becomes `>` on the quiet period                             | `the_rest_gate_needs_the_whole_quiet_period`                                          |
| the size gate                         | the limit comparison drops one byte                              | `the_size_gate_defers_one_byte_over_the_limit_and_accepts_the_limit`                  |
| the source stays read-only            | `open` drops `O_NOFOLLOW`                                        | `open_refuses_a_symbolic_link_and_reads_a_regular_file_by_descriptor`                 |
| the source is unchanged after a sweep | staging writes back one byte to the source handle                | `a_full_sweep_with_titles_leaves_every_container_entry_with_its_size_mtime_and_flags` |
| dry run touches no title copy         | `Mode::DryRun` refreshes the title copy                          | `dry_run_creates_no_state_directory_and_no_title_copy`                                |
| exclusive publication                 | `exclusive` falls back to `rename` when the target exists        | `publish_never_replaces_an_existing_target_and_names_it`                              |
| repair after rename, before clear     | `repair_publications` skips the directory sync before clearing   | `a_failed_directory_sync_leaves_the_entry_pending`                                    |
| target modified refuses               | the digest comparison in repair always matches                   | `any_other_bytes_are_refused_as_target_modified_and_nothing_is_overwritten`           |
| a future schema is refused            | `migrate` accepts any `user_version`                             | `a_future_schema_version_is_refused_with_its_number`                                  |
| the write lock                        | `acquire` returns before `flock` succeeds                        | `a_held_lock_makes_a_second_acquisition_busy_after_its_wait`                          |
| the deadline kills the group          | `terminate` signals the child pid instead of the group           | `the_deadline_terminates_the_whole_process_group_and_reaps_it`                        |
| descendant deadline overrides exit    | retain `Exited(0)` when a pipe-holding descendant times out      | `a_descendant_holding_stdout_does_not_outlive_the_deadline`                           |
| descendant interrupt overrides exit   | retain `Exited(0)` when a pipe-holding descendant is interrupted | `interruption_overrides_an_exited_leader_while_a_descendant_holds_stdout`             |
| surviving descendants get grace       | break the grace loop when only the leader exits                  | `a_surviving_descendant_receives_the_full_grace_after_the_leader_exits`               |
| the bounded reader                    | disable `Node::enter`'s depth guard                              | `depth_is_checked_before_reading_children`                                            |
| helper major version                  | compatibility preflight accepts any major                        | `incompatible_version_never_receives_notify_or_trash`                                 |
| command notify falls back once        | the fallback delivery is removed                                 | `failed_command_is_recorded_before_one_fallback_and_keeps_its_status`                 |
| an untracked file survives retention  | `Retention::run` trashes entries with no ledger owner            | `an_untracked_file_in_a_store_survives_and_is_reported_kept`                          |
| retention target modified             | `reconcile_intents` moves a path whose digest differs            | `replaced_content_is_a_refusal_naming_the_path_and_stays_pending`                     |
| audio excluded by default             | `include_audio` is ignored                                       | `audio_is_excluded_unless_include_audio_is_set`                                       |
| doctor never refuses at startup       | `checks::all` returns early on a config error                    | `without_a_config_the_config_check_fails_and_the_rest_are_reported_as_not_run`        |
| symlink verify writes nothing         | `verify` creates the target when missing                         | `verify_names_a_missing_link_and_a_wrong_target_and_writes_nothing`                   |
| expiration arithmetic                 | cast the hold to `i64` or ignore nanoseconds                     | `a_large_hold_does_not_wrap_and_nanoseconds_delay_the_boundary`                       |
| partial retention                     | append moves after ledger updates                                | `a_ledger_failure_after_the_move_still_reports_its_recording_id`                      |
| partial recovery                      | drop progress when a later intent fails                          | `partial_startup_retains_completed_ids_and_one_event_on_retention_and_ingest`         |
| single recovery event                 | emit recovery and new-move events separately                     | `retention_combines_recovery_and_new_moves_in_one_report_and_event`                   |
| root substitution                     | skip root revalidation before the helper                         | `replacing_the_audio_root_with_a_symlink_never_reaches_trash`                         |
| helper recovery compatibility         | skip the helper version check in Trash                           | `incompatible_helper_blocks_reconciliation_without_a_trash_request`                   |

Confirm the README carries both install steps:

Run: `grep -c 'cargo install --git https://github.com/webdavis/vpt vpt' README.md &&`
`grep -c 'swift build -c release' README.md`

Expected: `1` and `1`.

- [ ] **Step 5: Commit**

Run: `cargo fmt --all`

```bash
git add helper justfile .github .gitignore
SKIP_AI_COMMIT=1 git commit -m "feat(helper): the vpt-macos package with version, notify and trash"
```

______________________________________________________________________
