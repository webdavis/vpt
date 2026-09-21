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
  curated `lib.rs` exports; no `#[cfg(test)]` item above production code.
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
crates/vpt-application/src/ports/artifacts.rs ArtifactFiles, ArtifactRenderer, NoRenderers
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
crates/vpt-application/src/retention/reconcile.rs reconcile_intents
crates/vpt-application/src/inventory.rs      StoreInventory for vpt storage
crates/vpt-protocol/src/lib.rs               curated exports
crates/vpt-protocol/src/result.rs            vpt.result/1
crates/vpt-protocol/src/error.rs             vpt.error/1
crates/vpt-protocol/src/event.rs             vpt.event/1
crates/vpt-protocol/src/helper.rs            vpt.helper/1 and the notify and trash replies
crates/vpt-protocol/src/limits.rs            the bounded JSON reader
crates/vpt-adapters/src/lib.rs               curated exports
crates/vpt-adapters/src/config/schema.rs     the one key table: name, kind, default, comment, secret
crates/vpt-adapters/src/config/render.rs     the setup template
crates/vpt-adapters/src/config/load.rs       read, parse, unknown keys, merge over defaults
crates/vpt-adapters/src/config/validate.rs   value rules by kind
crates/vpt-adapters/src/config/settings.rs   table to Settings, path expansion
crates/vpt-adapters/src/config/roots.rs      root resolution and overlap refusals
crates/vpt-adapters/src/config/paths.rs      config path discovery (VPT_CONFIG, XDG, HOME)
crates/vpt-adapters/src/config/write.rs      setup's filesystem writes
crates/vpt-adapters/src/ledger/mod.rs        LedgerError
crates/vpt-adapters/src/ledger/sqlite/mod.rs SqliteLedger: open, permissions, WAL, busy timeout
crates/vpt-adapters/src/ledger/sqlite/migrations.rs the versioned schema
crates/vpt-adapters/src/ledger/sqlite/recordings.rs RecordingLedger for SQLite
crates/vpt-adapters/src/ledger/sqlite/journal.rs PublicationJournal for SQLite
crates/vpt-adapters/src/ledger/sqlite/retention.rs RetentionJournal for SQLite
crates/vpt-adapters/src/ledger/memory.rs     MemoryLedger, the in-memory twin
crates/vpt-adapters/src/ledger/contract.rs   (cfg(test)) the contract suite both ledgers run
crates/vpt-adapters/src/lock.rs              WriteLock over flock
crates/vpt-adapters/src/clock.rs             SystemClock
crates/vpt-adapters/src/voice_memos/store.rs VoiceMemosStore: listing and read-only descriptors
crates/vpt-adapters/src/voice_memos/titles.rs the private database copy and the title lookup
crates/vpt-adapters/src/archive/mod.rs       ClonefileArchive: staging and digests
crates/vpt-adapters/src/archive/publish.rs   exclusive publication and directory sync
crates/vpt-adapters/src/spawn.rs             bounded process execution
crates/vpt-adapters/src/helper.rs            HelperClient: version, notify, trash
crates/vpt-adapters/src/notify/mod.rs        DesktopNotifier, CommandNotifier, OffNotifier
crates/vpt-adapters/src/stores.rs            FilesystemStores
crates/vpt-adapters/src/symlink.rs           the managed link
crates/vpt-adapters/src/git_tree.rs          the git working tree walk
crates/vpt-adapters/src/prompt.rs            TtyPrompt
crates/vpt/src/main.rs                       fn main() { vpt::run() }
crates/vpt/src/lib.rs                        run(): parse, dispatch, emit, exit
crates/vpt/src/cli/args.rs                   Verb, Invocation, parse, USAGE
crates/vpt/src/cli/output.rs                 Outcome and emission
crates/vpt/src/compose.rs                    Runtime: settings, roots, adapters
crates/vpt/src/documents/record.rs           RecordingRecord to JSON
crates/vpt/src/commands/*.rs                 one file per verb
crates/vpt/src/doctor/mod.rs                 vpt doctor: the verdict
crates/vpt/src/doctor/checks.rs              one function per check
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
        for leaf in ["home", "config", "data", "state", "bin", "recordings"] {
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

Run: `cargo test -p vpt`

Expected: `test result: ok.` for the four unit tests in `args.rs`, the two tests in `version.rs` and the
one in `usage.rs`.

Run: `cargo fmt --all -- --check &&`
`cargo clippy --locked --workspace --all-targets --features dev-tools -- -D warnings`

Expected: no output from fmt; clippy finishes with `Finished` and no warnings.

- [ ] **Step 5: Commit**

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
  `pub ids: Vec<String>, pub completed: Vec<String>, pub checks: Option<Vec<Check>> }` with
  `ErrorDocument::new(kind: ErrorKind, message: impl Into<String>) -> ErrorDocument`, the builder methods
  `rule(self, rule: &str) -> Self`, `ids(self, ids: Vec<String>) -> Self`,
  `completed(self, completed: Vec<String>) -> Self`, `checks(self, checks: Vec<Check>) -> Self`, and
  `exit_code(&self) -> i32`, `to_json(&self) -> serde_json::Value`;
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
}

impl ErrorDocument {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        ErrorDocument { kind, rule: None, message: message.into(), ids: vec![], completed: vec![], checks: None }
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
                format!("vpt: {}\n", error.message)
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

Run: `cargo test -p vpt-protocol -p vpt`

Expected: all tests in `error.rs`, `result.rs`, `args.rs`, `version.rs` and `usage.rs` PASS.

- [ ] **Step 5: Commit**

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

Run: `cargo test -p vpt-adapters config`

Expected: 15 tests PASS.

- [ ] **Step 5: Commit**

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

- Consumes: `schema::{KEYS, spec_for, Kind}`, `load::{ConfigError, get}` (Task 3).

- Produces:
  `vpt_domain::duration::{parse_duration(text: &str) -> Result<u64, DurationError>, DurationError}`
  (seconds); `config::validate::validate(table: &toml::Table) -> Result<(), ConfigError>`, called at the
  end of `load_text`.

- [ ] **Step 1: Write the failing tests**

`crates/vpt-domain/src/duration.rs`, test section:

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

`crates/vpt-adapters/src/config/validate.rs`, test section:

```rust
#[cfg(test)]
mod tests {
    use crate::config::load::{ConfigError, load_text};

    fn with(body: &str) -> Result<toml::Table, ConfigError> {
        load_text(&format!("config_version = 1\n{body}"))
    }

    #[test]
    fn a_ratio_outside_its_range_is_refused_naming_the_key() {
        let error = with("[reconcile]\nconfidence_floor = 1.5\n").unwrap_err();
        assert_eq!(error, ConfigError::OutOfRange { key: "reconcile.confidence_floor".into(), rule: "a ratio in [0, 1]".into() });
    }

    #[test]
    fn the_divergence_ratio_must_be_above_zero() {
        let error = with("[reconcile]\nmax_divergence_ratio = 0.0\n").unwrap_err();
        assert_eq!(error.key(), Some("reconcile.max_divergence_ratio"));
    }

    #[test]
    fn a_positive_integer_refuses_zero() {
        let error = with("[source]\nquiet_period_secs = 0\n").unwrap_err();
        assert_eq!(error, ConfigError::OutOfRange { key: "source.quiet_period_secs".into(), rule: "a positive 32-bit integer".into() });
    }

    #[test]
    fn a_non_negative_integer_accepts_zero_and_refuses_minus_one() {
        assert!(with("[relations]\nsession_gap_minutes = 0\n").is_ok());
        assert_eq!(with("[relations]\nsession_gap_minutes = -1\n").unwrap_err().key(), Some("relations.session_gap_minutes"));
    }

    #[test]
    fn a_32_bit_key_refuses_a_value_above_i32() {
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

Expected: compile error, `parse_duration` not found.

Run: `cargo test -p vpt-adapters validate`

Expected: compile error, `ConfigError::key` not found; after adding it, every test but the two `is_ok`
assertions FAILS because `load_text` accepts anything the table names.

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

`crates/vpt-domain/src/lib.rs`:

```rust
//! Pure policy: identity, the wholeness gate, sweep gates, retention and value types.

pub mod duration;
```

`crates/vpt-adapters/src/config/validate.rs`:

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

fn check(key: &str, kind: Kind, value: &Value) -> Result<(), ConfigError> {
    let wrong = |expected: &str| ConfigError::WrongType { key: key.into(), expected: expected.into() };
    let range = |rule: &str| ConfigError::OutOfRange { key: key.into(), rule: rule.into() };
    match kind {
        Kind::Bool => value.as_bool().map(|_| ()).ok_or_else(|| wrong("boolean")),
        Kind::PositiveInt => match value.as_integer() {
            Some(n) if n >= 1 && n <= i64::from(i32::MAX) => Ok(()),
            Some(_) => Err(range("a positive 32-bit integer")),
            None => Err(wrong("integer")),
        },
        Kind::NonNegativeInt => match value.as_integer() {
            Some(n) if n >= 0 && n <= i64::from(i32::MAX) => Ok(()),
            Some(_) => Err(range("a non-negative 32-bit integer")),
            None => Err(wrong("integer")),
        },
        Kind::PositiveInt64 => match value.as_integer() {
            Some(n) if n >= 1 => Ok(()),
            Some(_) => Err(range("a positive 64-bit integer")),
            None => Err(wrong("integer")),
        },
        Kind::Ratio => match value.as_float() {
            Some(f) if f.is_finite() && (0.0..=1.0).contains(&f) => Ok(()),
            Some(_) => Err(range("a ratio in [0, 1]")),
            None => Err(wrong("number")),
        },
        Kind::RatioAboveZero => match value.as_float() {
            Some(f) if f.is_finite() && f > 0.0 && f <= 1.0 => Ok(()),
            Some(_) => Err(range("a ratio in (0, 1]")),
            None => Err(wrong("number")),
        },
        Kind::Duration => match value {
            Value::Integer(0) => Ok(()),
            Value::String(text) => parse_duration(text).map(|_| ()).map_err(|_| range("0 or <n>s|m|h|d")),
            _ => Err(range("0 or <n>s|m|h|d")),
        },
        Kind::Enum(options) => match value.as_str() {
            Some(text) if options.contains(&text) => Ok(()),
            Some(_) => Err(range(&format!("one of {}", options.join(", ")))),
            None => Err(wrong("string")),
        },
        Kind::Text | Kind::Secret => value.as_str().map(|_| ()).ok_or_else(|| wrong("string")),
        Kind::TextList | Kind::Argv => match value.as_array() {
            Some(items) if items.iter().all(Value::is_str) => Ok(()),
            _ => Err(wrong("list of strings")),
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

`crates/vpt-adapters/src/config/mod.rs` gains `pub mod validate;`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p vpt-domain -p vpt-adapters`

Expected: all PASS, including Task 3's tests.

- [ ] **Step 5: Commit**

```bash
git add crates/vpt-domain crates/vpt-adapters
SKIP_AI_COMMIT=1 git commit -m "feat(config): value rules by kind and the duration syntax"
```

______________________________________________________________________

### Task 5: Settings and root resolution

The validated table becomes typed `Settings` the application reads; roots are expanded, made absolute,
resolved through the home's permitted symlink, and refused when they overlap in a way spec section 4.1
forbids. The home may contain its stores.

**Files:**

- Create: `crates/vpt-domain/src/layout.rs`
- Modify: `crates/vpt-domain/src/lib.rs`
- Create: `crates/vpt-application/src/settings.rs`
- Modify: `crates/vpt-application/src/lib.rs`
- Create: `crates/vpt-adapters/src/config/settings.rs`, `crates/vpt-adapters/src/config/roots.rs`,
  `crates/vpt-adapters/src/config/paths.rs`
- Modify: `crates/vpt-adapters/src/config/mod.rs`

**Interfaces:**

- Consumes: `load::{get, ConfigError}`, `schema::STORE_KEYS`, `parse_duration`.

- Produces:

  - `vpt_domain::layout::{StoreKey, RootName, RootConflict,`
    `check_overlaps(roots: &[(RootName, &Path)]) -> Result<(), RootConflict>}`;
    `StoreKey::{Audio, Transcripts, Analysis, Briefs, EngineOutputs, Drafts, Released}` with
    `fn key_name(self) -> &'static str`, `fn default_leaf(self) -> &'static str`,
    `fn all() -> [StoreKey; 7]`, `fn is_private(self) -> bool` (everything but `Released`).
  - `vpt_application::settings::{Settings, StorePaths, SourceSettings, NotifySettings,`
    `NotifyMode, RetentionSettings}` (fields listed in the code).
  - `config::settings::from_table(table: &toml::Table, home_dir: &Path) -> Result<Settings,`
    `ConfigError>`.
  - `config::roots::{Roots, RootError, resolve(settings: &Settings, creation: Creation) ->`
    `Result<Roots, RootError>}` with `Creation::{CreateLeaves, None}`;
    `Roots { pub home, pub state_dir, pub stores: StorePaths, pub recordings_dir: PathBuf }` (all
    resolved).
  - `config::paths::{config_path(env: &dyn Fn(&str) -> Option<String>) -> PathBuf,`
    `default_state_dir(env) -> String}`.

- [ ] **Step 1: Write the failing tests**

`crates/vpt-domain/src/layout.rs`, test section:

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
    }
}
```

`crates/vpt-adapters/src/config/roots.rs`, test section:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::load::load_text;
    use crate::config::settings::from_table;

    fn settings_in(root: &std::path::Path, extra: &str) -> vpt_application::settings::Settings {
        let text = format!(
            "config_version = 1\n[home]\npath = \"{}\"\nstate_dir = \"{}\"\n[source]\nrecordings_dir = \"{}\"\n{extra}",
            root.join("h").display(),
            root.join("s").display(),
            root.join("vm").display()
        );
        from_table(&load_text(&text).expect("loads"), root).expect("settings")
    }

    #[test]
    fn default_store_leaves_are_created_beneath_an_existing_home() {
        let temp = tempfile::tempdir().expect("temp");
        std::fs::create_dir_all(temp.path().join("h")).expect("home");
        std::fs::create_dir_all(temp.path().join("vm")).expect("voice memos");
        let roots = resolve(&settings_in(temp.path(), ""), Creation::CreateLeaves).expect("resolves");
        assert!(roots.stores.audio.is_dir());
        assert_eq!(roots.stores.audio, temp.path().join("h/audio").canonicalize().expect("canonical"));
    }

    #[test]
    fn a_store_pointed_elsewhere_needs_an_existing_parent() {
        let temp = tempfile::tempdir().expect("temp");
        std::fs::create_dir_all(temp.path().join("h")).expect("home");
        std::fs::create_dir_all(temp.path().join("vm")).expect("voice memos");
        let extra = format!("[stores]\naudio = \"{}\"\n", temp.path().join("missing/audio").display());
        let error = resolve(&settings_in(temp.path(), &extra), Creation::CreateLeaves).unwrap_err();
        assert_eq!(error, RootError::ParentMissing { key: "stores.audio".into() });
    }

    #[test]
    fn a_store_inside_the_voice_memos_container_is_refused_naming_both_keys() {
        let temp = tempfile::tempdir().expect("temp");
        std::fs::create_dir_all(temp.path().join("h")).expect("home");
        std::fs::create_dir_all(temp.path().join("vm")).expect("voice memos");
        let extra = format!("[stores]\ndrafts = \"{}\"\n", temp.path().join("vm/drafts").display());
        let error = resolve(&settings_in(temp.path(), &extra), Creation::CreateLeaves).unwrap_err();
        assert_eq!(error, RootError::Overlap { first: "stores.drafts".into(), second: "source.recordings_dir".into() });
    }

    #[test]
    fn a_relative_root_after_expansion_is_refused() {
        let temp = tempfile::tempdir().expect("temp");
        std::fs::create_dir_all(temp.path().join("h")).expect("home");
        std::fs::create_dir_all(temp.path().join("vm")).expect("voice memos");
        let error = resolve(&settings_in(temp.path(), "[stores]\nbriefs = \"briefs\"\n"), Creation::None).unwrap_err();
        assert_eq!(error, RootError::NotAbsolute { key: "stores.briefs".into() });
    }

    #[test]
    fn a_symlinked_home_is_followed_when_no_target_is_configured() {
        let temp = tempfile::tempdir().expect("temp");
        std::fs::create_dir_all(temp.path().join("real")).expect("real home");
        std::fs::create_dir_all(temp.path().join("vm")).expect("voice memos");
        std::os::unix::fs::symlink(temp.path().join("real"), temp.path().join("h")).expect("link");
        let roots = resolve(&settings_in(temp.path(), ""), Creation::CreateLeaves).expect("resolves");
        assert_eq!(roots.home, temp.path().join("real").canonicalize().expect("canonical"));
    }
}
```

`crates/vpt-adapters/src/config/settings.rs`, test section:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::load::load_text;
    use std::path::Path;
    use vpt_application::settings::NotifyMode;
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
        let table = load_text("config_version = 1\n[notify]\nmode = \"command\"\ncommand = [\"pns\", \"{event}\"]\n").expect("loads");
        let settings = from_table(&table, Path::new("/u")).expect("settings");
        assert_eq!(settings.notify.mode, NotifyMode::Command(vec!["pns".into(), "{event}".into()]));
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

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-domain layout && cargo test -p vpt-adapters config`

Expected: compile errors naming `check_overlaps`, `from_table`, `resolve` and `Settings`.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-domain/src/layout.rs`:

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

A released store overlapping a private store is already covered by the store-versus-store arm. Add
`pub mod layout;` to `crates/vpt-domain/src/lib.rs`.

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

This needs `vpt_domain::retention::Hold` now. Create `crates/vpt-domain/src/retention.rs` with the type
only (its decision function arrives in Task 31):

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

and `pub mod retention;` in the domain `lib.rs`. `crates/vpt-application/src/lib.rs`:

```rust
//! Use cases and the ports they own.

pub mod settings;
```

`crates/vpt-adapters/src/config/settings.rs`:

```rust
//! From the validated table to typed settings, with `~` and `<home>/` expanded.

use super::load::{ConfigError, get};
use std::path::{Path, PathBuf};
use toml::{Table, Value};
use vpt_application::settings::{
    NotifyMode, NotifySettings, RetentionSettings, Settings, SourceSettings, StorePaths,
};
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
        config_version: integer(table, "config_version")? as u32,
        home,
        state_dir: expand(&text(table, "home.state_dir")?, home_dir, None),
        symlink_target,
        helper_path: PathBuf::from(text(table, "helper.path")?),
        stores,
        source: SourceSettings {
            recordings_dir: expand(&text(table, "source.recordings_dir")?, home_dir, None),
            read_titles: boolean(table, "source.read_titles")?,
            quiet_period_secs: integer(table, "source.quiet_period_secs")? as u64,
            deferral_page_threshold: integer(table, "source.deferral_page_threshold")? as u32,
            max_audio_bytes: integer(table, "source.max_audio_bytes")? as u64,
        },
        notify: NotifySettings { mode, aggregate_after: integer(table, "notify.aggregate_after")? as u32 },
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

fn integer(table: &Table, path: &str) -> Result<i64, ConfigError> {
    leaf(table, path)?.as_integer().ok_or_else(|| wrong(path, "integer"))
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

`crates/vpt-adapters/src/config/roots.rs`:

```rust
//! Resolve every configured root once, at startup, and refuse the overlaps of
//! spec section 4.1.

use std::path::{Path, PathBuf};
use vpt_application::settings::{Settings, StorePaths};
use vpt_domain::layout::{RootName, StoreKey, check_overlaps};

#[derive(Debug, PartialEq, Eq)]
pub enum RootError {
    NotAbsolute { key: String },
    ParentMissing { key: String },
    Overlap { first: String, second: String },
    Io { key: String, detail: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Creation {
    CreateLeaves,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Roots {
    pub home: PathBuf,
    pub state_dir: PathBuf,
    pub stores: StorePaths,
    pub recordings_dir: PathBuf,
}

pub fn resolve(settings: &Settings, creation: Creation) -> Result<Roots, RootError> {
    let home = resolved(&settings.home, "home.path", creation)?;
    let state_dir = resolved(&settings.state_dir, "home.state_dir", creation)?;
    let recordings_dir = if settings.source.recordings_dir.is_absolute() {
        settings.source.recordings_dir.canonicalize().unwrap_or_else(|_| settings.source.recordings_dir.clone())
    } else {
        return Err(RootError::NotAbsolute { key: "source.recordings_dir".into() });
    };
    let mut stores = settings.stores.clone();
    for key in StoreKey::all() {
        let path = resolved(settings.stores.get(key), &format!("stores.{}", key.key_name()), creation)?;
        stores.set(key, path);
    }
    let mut roots: Vec<(RootName, &Path)> = vec![
        (RootName::Home, &home),
        (RootName::State, &state_dir),
        (RootName::VoiceMemos, &recordings_dir),
    ];
    for key in StoreKey::all() {
        roots.push((RootName::Store(key), stores.get(key)));
    }
    check_overlaps(&roots)
        .map_err(|conflict| RootError::Overlap { first: conflict.first.key_name(), second: conflict.second.key_name() })?;
    Ok(Roots { home, state_dir, stores, recordings_dir })
}

/// An absolute path, canonical where it exists. A missing leaf under an existing
/// parent is created under `CreateLeaves`; a missing parent is a refusal.
fn resolved(path: &Path, key: &str, creation: Creation) -> Result<PathBuf, RootError> {
    if !path.is_absolute() {
        return Err(RootError::NotAbsolute { key: key.into() });
    }
    if let Ok(canonical) = path.canonicalize() {
        return Ok(canonical);
    }
    let parent = path.parent().ok_or_else(|| RootError::NotAbsolute { key: key.into() })?;
    let parent = parent.canonicalize().map_err(|_| RootError::ParentMissing { key: key.into() })?;
    let leaf = path.file_name().ok_or_else(|| RootError::NotAbsolute { key: key.into() })?;
    let target = parent.join(leaf);
    if creation == Creation::CreateLeaves {
        std::fs::create_dir(&target).map_err(|error| RootError::Io { key: key.into(), detail: error.to_string() })?;
    }
    Ok(target)
}
```

A canonical form of a missing store under the home is what `check_overlaps` compares, which is why the
home is resolved first and `CreateLeaves` creates the leaf under the canonical parent.

`crates/vpt-adapters/src/config/paths.rs`:

```rust
//! Where the configuration file is, from the environment alone.

use std::path::PathBuf;

pub fn config_path(env: &dyn Fn(&str) -> Option<String>) -> PathBuf {
    if let Some(explicit) = env("VPT_CONFIG") {
        return PathBuf::from(explicit);
    }
    if let Some(xdg) = env("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg).join("vpt/config.toml");
    }
    PathBuf::from(env("HOME").unwrap_or_default()).join(".config/vpt/config.toml")
}

/// The `home.state_dir` value `vpt setup` writes.
pub fn default_state_dir(env: &dyn Fn(&str) -> Option<String>) -> String {
    match env("XDG_STATE_HOME") {
        Some(xdg) => format!("{xdg}/vpt"),
        None => "~/.local/state/vpt".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_of(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> + '_ {
        move |name| pairs.iter().find(|(key, _)| *key == name).map(|(_, value)| (*value).to_owned())
    }

    #[test]
    fn vpt_config_wins_over_xdg_which_wins_over_home() {
        assert_eq!(config_path(&env_of(&[("VPT_CONFIG", "/c.toml"), ("XDG_CONFIG_HOME", "/x"), ("HOME", "/h")])), PathBuf::from("/c.toml"));
        assert_eq!(config_path(&env_of(&[("XDG_CONFIG_HOME", "/x"), ("HOME", "/h")])), PathBuf::from("/x/vpt/config.toml"));
        assert_eq!(config_path(&env_of(&[("HOME", "/h")])), PathBuf::from("/h/.config/vpt/config.toml"));
    }

    #[test]
    fn the_state_dir_default_follows_xdg_state_home_when_set() {
        assert_eq!(default_state_dir(&env_of(&[("XDG_STATE_HOME", "/s")])), "/s/vpt");
        assert_eq!(default_state_dir(&env_of(&[])), "~/.local/state/vpt");
    }
}
```

`crates/vpt-adapters/src/config/mod.rs` now lists `load`, `paths`, `render`, `roots`, `schema`,
`settings`, `validate`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --workspace`

Expected: all PASS.

- [ ] **Step 5: Commit**

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(config): typed settings and root resolution with overlap refusals"
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
- Modify: `crates/vpt/src/lib.rs`, `crates/vpt/src/commands/mod.rs`
- Test: `crates/vpt/tests/setup.rs`

**Interfaces:**

- Consumes: `config::render::template`, `config::paths::{config_path, default_state_dir}`,
  `config::settings::from_table`, `config::load::load_text`, `config::roots::{resolve, Creation}`.

- Produces:

  - `vpt_application::ports::prompt::{Prompt, PromptError}`:
    `fn choose(&mut self, question: &str, options: &[&str], preselected: usize) ->`
    `Result<usize, PromptError>`; `PromptError::{NoTerminal, Closed}`.
  - `vpt_application::setup::{Setup, SetupWriter, SetupError, SetupOutcome}`; `SetupWriter` is a port:
    `fn config_exists(&self) -> bool`,
    `fn write_config(&self, text: &str) -> Result<PathBuf, SetupError>`,
    `fn create_private_dir(&self, path: &Path) -> Result<(), SetupError>`;
    `Setup::run(&self, prompt: &mut dyn Prompt, writer: &dyn SetupWriter, force: bool) ->`
    `Result<SetupOutcome, SetupError>` where
    `Setup { pub engines: Vec<String>, pub render: Box<dyn Fn(&str) -> String>,`
    `pub directories: Vec<PathBuf> }` and
    `SetupOutcome { pub written: PathBuf, pub main_engine: String }`;
    `SetupError::{NoTerminal, Exists(PathBuf), Io(String)}`.
  - `vpt_adapters::prompt::TtyPrompt::open() -> Result<TtyPrompt, PromptError>`.
  - `vpt_adapters::config::write::FilesystemSetupWriter::new(config_path: PathBuf) ->`
    `FilesystemSetupWriter`.
  - `vpt::compose::Environment::from_process() -> Environment` with `fn config_path(&self) -> PathBuf`,
    `fn home_dir(&self) -> PathBuf`, `fn var(&self, name: &str) -> Option<String>`.

- [ ] **Step 1: Write the failing tests**

`crates/vpt-application/src/setup.rs`, test section:

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
        written: RefCell<Vec<String>>,
        dirs: RefCell<Vec<PathBuf>>,
    }
    impl SetupWriter for FakeWriter {
        fn config_exists(&self) -> bool {
            self.exists
        }
        fn write_config(&self, text: &str) -> Result<PathBuf, SetupError> {
            self.written.borrow_mut().push(text.to_owned());
            Ok(PathBuf::from("/c/config.toml"))
        }
        fn create_private_dir(&self, path: &Path) -> Result<(), SetupError> {
            self.dirs.borrow_mut().push(path.to_owned());
            Ok(())
        }
    }

    fn setup() -> Setup {
        Setup {
            engines: vec!["apple".into(), "whisply".into()],
            render: Box::new(|engine| format!("main = \"{engine}\"\n")),
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
        assert_eq!(writer.written.borrow().as_slice(), ["main = \"whisply\"\n"]);
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
    fn force_overwrites_an_existing_config() {
        let writer = writer(true);
        assert!(setup().run(&mut FakePrompt(Some(0)), &writer, true).is_ok());
        assert_eq!(writer.written.borrow().len(), 1);
    }

    #[test]
    fn no_terminal_writes_nothing() {
        let writer = writer(false);
        assert_eq!(setup().run(&mut FakePrompt(None), &writer, false).unwrap_err(), SetupError::NoTerminal);
        assert!(writer.written.borrow().is_empty());
    }
}
```

The fake writer needs `config_exists` to know the path; make `SetupError::Exists` carry the path the
writer reports through `fn config_path(&self) -> PathBuf` on `SetupWriter`.

`crates/vpt/tests/setup.rs`:

```rust
mod support;

use std::os::unix::fs::PermissionsExt;
use support::{Sandbox, run, stderr, stdout};

#[test]
fn setup_without_a_terminal_exits_2_and_writes_nothing() {
    let sandbox = Sandbox::new("setup-no-tty");

    let output = run(sandbox.vpt().args(["setup", "--json"]).stdin(std::process::Stdio::null()));

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout(&output), "");
    let document: serde_json::Value = serde_json::from_str(&stderr(&output)).expect("error document");
    assert_eq!(document["error"]["kind"], "usage");
    assert!(!sandbox.config_path().exists());
    assert!(!sandbox.path().join("home/.vpt").exists());
}

#[test]
fn setup_under_a_pty_writes_the_config_home_state_and_store_leaves() {
    let sandbox = Sandbox::new("setup-pty");

    let mut command = std::process::Command::new("/usr/bin/script");
    command.args(["-q", "/dev/null", support::VPT, "setup"]);
    let mut vpt = sandbox.vpt();
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
fn setup_refuses_an_existing_config_without_force() {
    let sandbox = Sandbox::new("setup-exists");
    std::fs::create_dir_all(sandbox.config_path().parent().expect("dir")).expect("config dir");
    std::fs::write(sandbox.config_path(), "config_version = 1\n").expect("existing");

    let output = run(sandbox.vpt().args(["setup"]).stdin(std::process::Stdio::null()));

    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("exists"), "{}", stderr(&output));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-application setup`

Expected: compile error, `Setup` not found.

Run: `cargo test -p vpt --test setup`

Expected: `setup_without_a_terminal_exits_2_and_writes_nothing` passes by accident (the verb is
unimplemented and exits 2 with kind usage), so tighten it: assert the message equals
`"vpt setup needs a controlling terminal"`. It then FAILS on the message. The pty test FAILS on the
missing config.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-application/src/ports/mod.rs`:

```rust
//! The external capabilities the use cases consume, one trait each.

pub mod prompt;
```

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

`crates/vpt-application/src/setup.rs`:

```rust
//! `vpt setup`: one prompt, one rendered file, the directories the spec names.

use crate::ports::prompt::{Prompt, PromptError};
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq)]
pub enum SetupError {
    NoTerminal,
    Exists(PathBuf),
    Io(String),
}

pub trait SetupWriter {
    fn config_path(&self) -> PathBuf;
    fn config_exists(&self) -> bool;
    fn write_config(&self, text: &str) -> Result<PathBuf, SetupError>;
    fn create_private_dir(&self, path: &Path) -> Result<(), SetupError>;
}

pub struct SetupOutcome {
    pub written: PathBuf,
    pub main_engine: String,
}

pub struct Setup {
    pub engines: Vec<String>,
    pub render: Box<dyn Fn(&str) -> String>,
    pub directories: Vec<PathBuf>,
}

impl Setup {
    pub fn run(&self, prompt: &mut dyn Prompt, writer: &dyn SetupWriter, force: bool) -> Result<SetupOutcome, SetupError> {
        if writer.config_exists() && !force {
            return Err(SetupError::Exists(writer.config_path()));
        }
        let options: Vec<&str> = self.engines.iter().map(String::as_str).collect();
        let chosen = prompt.choose("Main transcription engine", &options, 0).map_err(|error| match error {
            PromptError::NoTerminal => SetupError::NoTerminal,
            PromptError::Closed => SetupError::Io("the terminal closed before an answer".into()),
        })?;
        let main_engine = self.engines.get(chosen).cloned().unwrap_or_else(|| self.engines[0].clone());
        let written = writer.write_config(&(self.render)(&main_engine))?;
        for directory in &self.directories {
            writer.create_private_dir(directory)?;
        }
        Ok(SetupOutcome { written, main_engine })
    }
}
```

The fake in the test above must gain
`fn config_path(&self) -> PathBuf { PathBuf::from("/c/config.toml") }`.

`crates/vpt-application/src/lib.rs` adds `pub mod ports;` and `pub mod setup;`.

`crates/vpt-adapters/src/prompt.rs`:

```rust
//! The controlling terminal, opened by name so `--json` keeps stdout clean.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use vpt_application::ports::prompt::{Prompt, PromptError};

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
        reader.read_line(&mut answer).map_err(|_| PromptError::Closed)?;
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

`crates/vpt-adapters/src/config/write.rs`:

```rust
//! What `vpt setup` writes: a 0600 file in a 0700 directory, and 0700 directories.

use std::fs::{DirBuilder, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use vpt_application::setup::{SetupError, SetupWriter};

pub struct FilesystemSetupWriter {
    config_path: PathBuf,
}

impl FilesystemSetupWriter {
    pub fn new(config_path: PathBuf) -> Self {
        FilesystemSetupWriter { config_path }
    }
}

fn io(error: std::io::Error) -> SetupError {
    SetupError::Io(error.to_string())
}

impl SetupWriter for FilesystemSetupWriter {
    fn config_path(&self) -> PathBuf {
        self.config_path.clone()
    }

    fn config_exists(&self) -> bool {
        self.config_path.exists()
    }

    fn write_config(&self, text: &str) -> Result<PathBuf, SetupError> {
        if let Some(parent) = self.config_path.parent() {
            self.create_private_dir(parent)?;
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&self.config_path)
            .map_err(io)?;
        file.write_all(text.as_bytes()).map_err(io)?;
        file.sync_all().map_err(io)?;
        Ok(self.config_path.clone())
    }

    fn create_private_dir(&self, path: &Path) -> Result<(), SetupError> {
        DirBuilder::new().recursive(true).mode(0o700).create(path).map_err(io)
    }
}
```

`crates/vpt/src/compose.rs`:

```rust
//! The composition root: the environment, the settings and the adapters every
//! command shares. Nothing here decides policy.

use std::path::PathBuf;
use vpt_adapters::config::paths::{config_path, default_state_dir};

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
        config_path(&|name| self.var(name))
    }

    pub fn home_dir(&self) -> PathBuf {
        PathBuf::from(self.var("HOME").unwrap_or_default())
    }

    pub fn state_dir_default(&self) -> String {
        default_state_dir(&|name| self.var(name))
    }
}
```

`crates/vpt/src/commands/setup.rs`:

```rust
//! `vpt setup`.

use crate::cli::output::Outcome;
use crate::compose::Environment;
use serde_json::json;
use vpt_adapters::config::load::load_text;
use vpt_adapters::config::render::template;
use vpt_adapters::config::settings::from_table;
use vpt_adapters::config::write::FilesystemSetupWriter;
use vpt_adapters::prompt::TtyPrompt;
use vpt_application::ports::prompt::PromptError;
use vpt_application::setup::{Setup, SetupError};
use vpt_domain::layout::StoreKey;
use vpt_protocol::error::{ErrorDocument, ErrorKind};
use vpt_protocol::result::document;

pub fn run(environment: &Environment, force: bool) -> Outcome {
    let state_dir = environment.state_dir_default();
    let home_dir = environment.home_dir();
    let rendered_defaults = template("apple", &state_dir);
    let directories = match load_text(&rendered_defaults).map_err(|e| format!("{e:?}")).and_then(|table| {
        from_table(&table, &home_dir).map_err(|e| format!("{e:?}"))
    }) {
        Ok(settings) => {
            let mut directories = vec![settings.home.clone(), settings.state_dir.clone()];
            directories.extend(StoreKey::all().iter().map(|key| settings.stores.get(*key).to_path_buf()));
            directories
        }
        Err(detail) => return Outcome::Failure(ErrorDocument::new(ErrorKind::Config, detail)),
    };
    let setup = Setup {
        engines: vec!["apple".into(), "whisply".into()],
        render: Box::new(move |engine| template(engine, &state_dir)),
        directories,
    };
    let writer = FilesystemSetupWriter::new(environment.config_path());
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
        Err(SetupError::NoTerminal) => Outcome::Failure(ErrorDocument::new(ErrorKind::Usage, "vpt setup needs a controlling terminal")),
        Err(SetupError::Exists(path)) => Outcome::Failure(ErrorDocument::new(
            ErrorKind::Usage,
            format!("{} exists; pass --force to overwrite it", path.display()),
        )),
        Err(SetupError::Io(detail)) => Outcome::Failure(ErrorDocument::new(ErrorKind::Store, detail)),
    }
}
```

Setup checks for the existing config before opening the terminal so the refusal needs no tty: move the
`writer.config_exists() && !force` check ahead of `TtyPrompt::open()` in `run` (mirror the use case's
first line). `crates/vpt/src/lib.rs` gains `pub mod compose;` and the dispatch arm:

```rust
        Verb::Setup { force } => commands::setup::run(&Environment::from_process(), *force),
```

with `use compose::Environment;` at the top, and `commands/mod.rs` lists `setup`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --workspace --features dev-tools`

Expected: all PASS, the pty test included.

- [ ] **Step 5: Commit**

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
  `FileTime { pub secs: i64, pub nanos: u32 }, Civil { year, month, day, hour, minute,` `second }}` with
  `UtcInstant::rfc3339(&self) -> String`, `UtcInstant::rfc3339_with(&self, offset: UtcOffset) -> String`,
  `UtcInstant::civil(&self, offset: UtcOffset) -> Civil`, `UtcOffset::label(&self) -> String`;
  `vpt_domain::digest::Sha256Digest(pub [u8; 32])` with `hex()`, `hash12()`, `hash8()`,
  `from_hex(&str) -> Option<Sha256Digest>`.

- [ ] **Step 1: Write the failing tests**

`crates/vpt-domain/src/time.rs`, test section:

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
        let captured = UtcInstant { secs: 1_787_690_856 };
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
    fn a_file_time_ages_by_whole_seconds() {
        let mtime = FileTime { secs: 100, nanos: 999_999_999 };
        assert_eq!(mtime.age_secs(UtcInstant { secs: 130 }), 30);
    }
}
```

`crates/vpt-domain/src/digest.rs`, test section:

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

Run: `cargo test -p vpt-domain time digest`

Expected: compile errors naming the missing types.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-domain/src/time.rs`:

```rust
//! Instants, offsets and file times, with the civil-date arithmetic the
//! identity and the notes need. No clock lives here.

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
        let local = self.secs + i64::from(offset.secs);
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

impl FileTime {
    pub fn age_secs(self, now: UtcInstant) -> i64 {
        now.secs - self.secs
    }
}

/// Days since 1970-01-01 to a proleptic Gregorian date (Howard Hinnant's algorithm).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}
```

`crates/vpt-domain/src/digest.rs`:

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

`crates/vpt-domain/src/lib.rs` lists `digest`, `duration`, `layout`, `retention`, `time`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p vpt-domain`

Expected: all PASS.

- [ ] **Step 5: Commit**

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

- Consumes: `time::{UtcInstant, UtcOffset}`, `digest::Sha256Digest`.

- Produces: `vpt_domain::identity::{RecordingId, IdentityError,`
  `local_timestamp(captured: UtcInstant, offset: UtcOffset) -> String}`;
  `RecordingId::derive(captured: UtcInstant, offset: UtcOffset, digest: &Sha256Digest) -> RecordingId`,
  `RecordingId::parse(text: &str) -> Result<RecordingId, IdentityError>`, `fn as_str(&self) -> &str`,
  `fn capture_date(&self) -> &str`, `fn hash12(&self) -> &str`.

- [ ] **Step 1: Write the failing tests**

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
        let id = RecordingId::derive(UtcInstant { secs: 1_787_690_856 }, UtcOffset { secs: -21_600 }, &digest());
        assert_eq!(id.as_str(), "2026-08-24T144736-4f3ab19c02de");
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
        assert_eq!(RecordingId::parse("../2026-08-24T144736-4f3ab19c02de"), Err(IdentityError::Malformed));
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-domain identity`

Expected: compile error, `RecordingId` not found.

- [ ] **Step 3: Write the minimal implementation**

```rust
//! `<local-capture-timestamp>-<hash12>`: both halves come from the file.

use crate::digest::Sha256Digest;
use crate::time::{UtcInstant, UtcOffset};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RecordingId(String);

#[derive(Debug, PartialEq, Eq)]
pub enum IdentityError {
    Malformed,
}

pub fn local_timestamp(captured: UtcInstant, offset: UtcOffset) -> String {
    let c = captured.civil(offset);
    format!("{:04}-{:02}-{:02}T{:02}{:02}{:02}", c.year, c.month, c.day, c.hour, c.minute, c.second)
}

impl RecordingId {
    pub fn derive(captured: UtcInstant, offset: UtcOffset, digest: &Sha256Digest) -> RecordingId {
        RecordingId(format!("{}-{}", local_timestamp(captured, offset), digest.hash12()))
    }

    pub fn parse(text: &str) -> Result<RecordingId, IdentityError> {
        let bytes = text.as_bytes();
        let shape_ok = bytes.len() == 30
            && bytes[4] == b'-'
            && bytes[7] == b'-'
            && bytes[10] == b'T'
            && bytes[17] == b'-'
            && bytes[..4].iter().chain(&bytes[5..7]).chain(&bytes[8..10]).chain(&bytes[11..17]).all(u8::is_ascii_digit)
            && bytes[18..].iter().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b));
        if shape_ok { Ok(RecordingId(text.to_owned())) } else { Err(IdentityError::Malformed) }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn capture_date(&self) -> &str {
        &self.0[..10]
    }

    pub fn hash12(&self) -> &str {
        &self.0[18..]
    }
}

impl std::fmt::Display for RecordingId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
```

Add `pub mod identity;` to `lib.rs`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p vpt-domain identity`

Expected: 3 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/vpt-domain
SKIP_AI_COMMIT=1 git commit -m "feat(domain): the recording identity"
```

______________________________________________________________________

### Task 9: The MPEG-4 wholeness gate

The gate walks the top-level boxes reading bounded headers with checked offsets, never the whole file;
the sum of box lengths must equal the file size exactly and a `moov` box with an `mvhd` must be present.
The `mvhd` supplies the capture time (seconds since 1904-01-01 UTC) and the duration.

**Files:**

- Create: `crates/vpt-domain/src/container.rs`, `crates/vpt-domain/src/fixtures.rs`
- Modify: `crates/vpt-domain/src/lib.rs`

**Interfaces:**

- Consumes: `time::UtcInstant`.

- Produces: `vpt_domain::container::{BoxReader, ReadFailure,`
  `Container { pub creation_time: UtcInstant, pub duration_secs: u64 }, ContainerError,`
  `inspect(reader: &mut dyn BoxReader) -> Result<Container, ContainerError>}`;
  `BoxReader { fn len(&self) -> u64; fn read_exact_at(&mut self, offset: u64,`
  `buf: &mut [u8]) -> Result<(), ReadFailure>; }`;
  `ContainerError::{PastEnd { offset: u64 }, InvalidLength { offset: u64 }, Overflow,`
  `MissingMoov, MissingMvhd, InvalidMvhd, Unreadable}`;
  `vpt_domain::fixtures::{m4a(creation_unix: i64, duration_secs: u32, payload: &[u8]) ->`
  `Vec<u8>, box_of(kind: &[u8; 4], body: &[u8]) -> Vec<u8>, mvhd(creation_unix: i64,`
  `duration_secs: u32) -> Vec<u8>, mvhd_v1(creation_unix: i64, duration_secs: u64) ->` `Vec<u8>, Bytes}`
  behind the `fixtures` feature, where `Bytes(pub Vec<u8>)` implements `BoxReader`.

- [ ] **Step 1: Write the failing tests**

`crates/vpt-domain/src/container.rs`, test section:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::{Bytes, box_of, m4a, mvhd, mvhd_v1};

    const CAPTURED: i64 = 1_787_690_856;

    #[test]
    fn a_whole_file_yields_its_capture_time_and_duration() {
        let container = inspect(&mut Bytes(m4a(CAPTURED, 612, b"audio"))).expect("whole");
        assert_eq!(container.creation_time, UtcInstant { secs: CAPTURED });
        assert_eq!(container.duration_secs, 612);
    }

    #[test]
    fn a_truncated_download_loses_moov_and_is_refused_past_the_end() {
        let mut bytes = m4a(CAPTURED, 612, b"audio");
        bytes.truncate(bytes.len() - 10);
        assert!(matches!(inspect(&mut Bytes(bytes)), Err(ContainerError::PastEnd { .. })));
    }

    #[test]
    fn a_file_without_moov_is_refused() {
        let mut bytes = box_of(b"ftyp", b"M4A ");
        bytes.extend(box_of(b"mdat", b"audio"));
        assert_eq!(inspect(&mut Bytes(bytes)), Err(ContainerError::MissingMoov));
    }

    #[test]
    fn a_moov_without_mvhd_is_refused() {
        let mut bytes = box_of(b"ftyp", b"M4A ");
        bytes.extend(box_of(b"moov", &box_of(b"udta", b"")));
        assert_eq!(inspect(&mut Bytes(bytes)), Err(ContainerError::MissingMvhd));
    }

    #[test]
    fn a_box_length_below_its_header_is_invalid() {
        let mut bytes = vec![0, 0, 0, 4];
        bytes.extend(b"ftyp");
        assert_eq!(inspect(&mut Bytes(bytes)), Err(ContainerError::InvalidLength { offset: 0 }));
    }

    #[test]
    fn a_largesize_that_overflows_is_refused_not_wrapped() {
        let mut bytes = vec![0, 0, 0, 1];
        bytes.extend(b"mdat");
        bytes.extend(u64::MAX.to_be_bytes());
        bytes.extend([0u8; 8]);
        assert_eq!(inspect(&mut Bytes(bytes)), Err(ContainerError::Overflow));
    }

    #[test]
    fn a_version_one_mvhd_is_read_with_its_64_bit_fields() {
        let mut bytes = box_of(b"ftyp", b"M4A ");
        bytes.extend(box_of(b"moov", &mvhd_v1(CAPTURED, 7_200)));
        let container = inspect(&mut Bytes(bytes)).expect("v1");
        assert_eq!(container.creation_time.secs, CAPTURED);
        assert_eq!(container.duration_secs, 7_200);
    }

    #[test]
    fn a_zero_timescale_is_an_invalid_mvhd() {
        let mut body = mvhd(CAPTURED, 1);
        body[8 + 12..8 + 16].copy_from_slice(&[0, 0, 0, 0]);
        let mut bytes = box_of(b"ftyp", b"M4A ");
        bytes.extend(box_of(b"moov", &body));
        assert_eq!(inspect(&mut Bytes(bytes)), Err(ContainerError::InvalidMvhd));
    }

    #[test]
    fn only_bounded_headers_are_read_never_the_payload() {
        struct Counting(Bytes, u64);
        impl BoxReader for Counting {
            fn len(&self) -> u64 {
                self.0.len()
            }
            fn read_exact_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), ReadFailure> {
                self.1 += buf.len() as u64;
                self.0.read_exact_at(offset, buf)
            }
        }
        let mut reader = Counting(Bytes(m4a(CAPTURED, 1, &[0u8; 100_000])), 0);
        inspect(&mut reader).expect("whole");
        assert!(reader.1 < 1_000, "read {} bytes", reader.1);
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-domain --features fixtures container`

Expected: compile error, `inspect` and the fixtures not found.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-domain/src/fixtures.rs`:

```rust
//! Assembled MPEG-4 bytes for tests. Behind the `fixtures` feature so other
//! crates' tests can build the same files.

use crate::container::{BoxReader, ReadFailure};

const MAC_EPOCH_OFFSET: i64 = 2_082_844_800;

pub struct Bytes(pub Vec<u8>);

impl BoxReader for Bytes {
    fn len(&self) -> u64 {
        self.0.len() as u64
    }

    fn read_exact_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), ReadFailure> {
        let start = usize::try_from(offset).map_err(|_| ReadFailure)?;
        let end = start.checked_add(buf.len()).ok_or(ReadFailure)?;
        let slice = self.0.get(start..end).ok_or(ReadFailure)?;
        buf.copy_from_slice(slice);
        Ok(())
    }
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

`crates/vpt-domain/src/container.rs`:

```rust
//! The wholeness gate: bounded box headers with checked offsets, the sum of
//! box lengths equal to the file size, `moov` present, `mvhd` inside it.

use crate::time::UtcInstant;

const MAC_EPOCH_OFFSET: i64 = 2_082_844_800;

#[derive(Debug, PartialEq, Eq)]
pub struct ReadFailure;

pub trait BoxReader {
    fn len(&self) -> u64;
    fn read_exact_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), ReadFailure>;
}

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

pub fn inspect(reader: &mut dyn BoxReader) -> Result<Container, ContainerError> {
    let size = reader.len();
    let mut offset = 0;
    let mut moov = None;
    while offset < size {
        let header = read_header(reader, offset, size)?;
        if &header.kind == b"moov" {
            moov = Some((offset + header.header_len, offset + header.length));
        }
        offset = offset.checked_add(header.length).ok_or(ContainerError::Overflow)?;
    }
    let (start, end) = moov.ok_or(ContainerError::MissingMoov)?;
    let mut child = start;
    while child < end {
        let header = read_header(reader, child, end)?;
        if &header.kind == b"mvhd" {
            return read_mvhd(reader, child + header.header_len, header.length - header.header_len);
        }
        child = child.checked_add(header.length).ok_or(ContainerError::Overflow)?;
    }
    Err(ContainerError::MissingMvhd)
}

fn read_header(reader: &mut dyn BoxReader, offset: u64, end: u64) -> Result<Header, ContainerError> {
    if offset.checked_add(8).ok_or(ContainerError::Overflow)? > end {
        return Err(ContainerError::PastEnd { offset });
    }
    let mut head = [0u8; 8];
    reader.read_exact_at(offset, &mut head).map_err(|_| ContainerError::Unreadable)?;
    let size32 = u32::from_be_bytes([head[0], head[1], head[2], head[3]]);
    let kind = [head[4], head[5], head[6], head[7]];
    let (length, header_len) = match size32 {
        0 => (end - offset, 8),
        1 => {
            if offset + 16 > end {
                return Err(ContainerError::PastEnd { offset });
            }
            let mut large = [0u8; 8];
            reader.read_exact_at(offset + 8, &mut large).map_err(|_| ContainerError::Unreadable)?;
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

fn read_mvhd(reader: &mut dyn BoxReader, body: u64, body_len: u64) -> Result<Container, ContainerError> {
    let mut version = [0u8; 1];
    if body_len < 1 {
        return Err(ContainerError::InvalidMvhd);
    }
    reader.read_exact_at(body, &mut version).map_err(|_| ContainerError::Unreadable)?;
    let (creation, timescale, duration) = match version[0] {
        0 => {
            if body_len < 20 {
                return Err(ContainerError::InvalidMvhd);
            }
            let mut fields = [0u8; 16];
            reader.read_exact_at(body + 4, &mut fields).map_err(|_| ContainerError::Unreadable)?;
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
            reader.read_exact_at(body + 4, &mut fields).map_err(|_| ContainerError::Unreadable)?;
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

`crates/vpt-domain/src/lib.rs`:

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

Run: `cargo test -p vpt-domain --features fixtures container`

Expected: 9 tests PASS. Run `cargo clippy -p vpt-domain --all-targets --features fixtures -- -D warnings`
and expect no warnings.

- [ ] **Step 5: Commit**

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
  `pre_open, size_gate, rest_gate}`;
  `DeferralReason::{Dataless, AudioTooLarge, InvalidContainer, NotAtRest, ChangedDuringRead}` with
  `fn as_str(self) -> &'static str` and `fn parse(text: &str) -> Option<DeferralReason>`;
  `PreOpen::{Dataless, Unchanged, Open}`;
  `pre_open(candidate: &CandidateFacts, seen: Option<&SeenFacts>) -> PreOpen`;
  `size_gate(size: u64, limits: &SweepLimits) -> Result<(), DeferralReason>`;
  `rest_gate(mtime: FileTime, size: u64, now: UtcInstant, seen: Option<&SeenFacts>,`
  `limits: &SweepLimits) -> Result<(), DeferralReason>`.

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const LIMITS: SweepLimits = SweepLimits { max_audio_bytes: 2_147_483_648, quiet_period_secs: 30 };

    fn candidate(size: u64, secs: i64, dataless: bool) -> CandidateFacts {
        CandidateFacts { size, mtime: FileTime { secs, nanos: 0 }, dataless }
    }

    #[test]
    fn a_dataless_entry_is_never_opened() {
        assert_eq!(pre_open(&candidate(10, 0, true), None), PreOpen::Dataless);
    }

    #[test]
    fn an_unchanged_ingested_triple_is_skipped_and_a_changed_one_is_opened() {
        let seen = SeenFacts { size: 10, mtime: FileTime { secs: 5, nanos: 0 }, ingested: true, deferred_size: None };
        assert_eq!(pre_open(&candidate(10, 5, false), Some(&seen)), PreOpen::Unchanged);
        assert_eq!(pre_open(&candidate(11, 5, false), Some(&seen)), PreOpen::Open);
        let deferred = SeenFacts { ingested: false, ..seen };
        assert_eq!(pre_open(&candidate(10, 5, false), Some(&deferred)), PreOpen::Open);
    }

    #[test]
    fn the_size_gate_defers_one_byte_over_the_limit_and_accepts_the_limit() {
        assert_eq!(size_gate(2_147_483_648, &LIMITS), Ok(()));
        assert_eq!(size_gate(2_147_483_649, &LIMITS), Err(DeferralReason::AudioTooLarge));
    }

    #[test]
    fn the_rest_gate_needs_the_whole_quiet_period() {
        let now = UtcInstant { secs: 1_000 };
        assert_eq!(rest_gate(FileTime { secs: 971, nanos: 0 }, 10, now, None, &LIMITS), Err(DeferralReason::NotAtRest));
        assert_eq!(rest_gate(FileTime { secs: 970, nanos: 0 }, 10, now, None, &LIMITS), Ok(()));
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

Expected: compile error, `pre_open` and friends not found.

- [ ] **Step 3: Write the minimal implementation**

```rust
//! The sweep gates of spec section 5.2, cheapest first, as pure decisions.

use crate::time::{FileTime, UtcInstant};

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
    pub dataless: bool,
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
    if candidate.dataless {
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

Add `pub mod sweep;` to `lib.rs`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p vpt-domain sweep`

Expected: 6 tests PASS.

- [ ] **Step 5: Commit**

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

**Files:**

- Create: `crates/vpt-application/src/ports/ledger.rs` (the error type only in this task)
- Modify: `crates/vpt-application/src/ports/mod.rs`
- Create: `crates/vpt-adapters/src/ledger/mod.rs`, `crates/vpt-adapters/src/ledger/sqlite/mod.rs`,
  `crates/vpt-adapters/src/ledger/sqlite/migrations.rs`
- Modify: `crates/vpt-adapters/src/lib.rs`

**Interfaces:**

- Consumes: nothing.

- Produces: `vpt_application::ports::ledger::LedgerError::{Busy, UnsupportedSchema(u32),`
  `Conflict(String), Corrupt(String), Io(String)}`; `vpt_adapters::ledger::sqlite::SqliteLedger` with
  `pub const BUSY_TIMEOUT: Duration = Duration::from_secs(5)`,
  `pub fn open(state_dir: &Path) -> Result<SqliteLedger, LedgerError>`,
  `pub fn open_with_timeout(state_dir: &Path, busy: Duration) -> Result<SqliteLedger, LedgerError>`,
  `pub fn database_path(&self) -> &Path`, `pub fn schema_version(&self) -> Result<u32, LedgerError>`,
  `pub(crate) fn transaction<T>(&self, op: impl FnOnce(&rusqlite::Transaction) ->`
  `Result<T, LedgerError>) -> Result<T, LedgerError>`,
  `pub(crate) fn read<T>(&self, op: impl FnOnce(&rusqlite::Connection) -> Result<T,`
  `LedgerError>) -> Result<T, LedgerError>`;
  `vpt_adapters::ledger::sqlite::migrations::VERSION: u32 = 1`;
  `pub(crate) fn map(error: rusqlite::Error) -> LedgerError`.

- [ ] **Step 1: Write the failing tests**

`crates/vpt-adapters/src/ledger/sqlite/mod.rs`, test section:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn open_creates_a_0600_database_in_a_0700_directory_at_schema_version_1() {
        let temp = tempfile::tempdir().expect("temp");
        let state = temp.path().join("state/vpt");

        let ledger = SqliteLedger::open(&state).expect("opens");

        assert_eq!(std::fs::metadata(&state).expect("dir").permissions().mode() & 0o777, 0o700);
        assert_eq!(std::fs::metadata(ledger.database_path()).expect("db").permissions().mode() & 0o777, 0o600);
        assert_eq!(ledger.schema_version().expect("version"), 1);
        let mode: String = ledger.read(|c| c.query_row("PRAGMA journal_mode", [], |row| row.get(0)).map_err(map)).expect("mode");
        assert_eq!(mode, "wal");
    }

    #[test]
    fn opening_twice_is_idempotent_and_every_table_exists() {
        let temp = tempfile::tempdir().expect("temp");
        SqliteLedger::open(temp.path()).expect("first");
        let ledger = SqliteLedger::open(temp.path()).expect("second");
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
        let temp = tempfile::tempdir().expect("temp");
        let ledger = SqliteLedger::open(temp.path()).expect("opens");
        ledger.read(|c| c.pragma_update(None, "user_version", 99).map_err(map)).expect("bump");
        assert_eq!(SqliteLedger::open(temp.path()).unwrap_err(), LedgerError::UnsupportedSchema(99));
    }

    #[test]
    fn a_held_write_lock_surfaces_as_busy_after_the_bounded_wait() {
        let temp = tempfile::tempdir().expect("temp");
        let ledger = SqliteLedger::open_with_timeout(temp.path(), Duration::from_millis(50)).expect("opens");
        let mut holder = rusqlite::Connection::open(ledger.database_path()).expect("second connection");
        let held = holder.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).expect("held");

        let outcome = ledger.transaction(|t| t.execute("DELETE FROM seen", []).map(|_| ()).map_err(map));

        assert_eq!(outcome.unwrap_err(), LedgerError::Busy);
        drop(held);
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-adapters ledger`

Expected: compile error, `SqliteLedger` not found.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-application/src/ports/ledger.rs` (this task's part; Task 12 adds the rows and traits):

```rust
//! The ledger repositories a use case reads and writes.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerError {
    Busy,
    UnsupportedSchema(u32),
    Conflict(String),
    Corrupt(String),
    Io(String),
}
```

`crates/vpt-application/src/ports/mod.rs` adds `pub mod ledger;`.

`crates/vpt-adapters/src/ledger/mod.rs`:

```rust
//! The ledger: one SQLite type implementing every repository, and an
//! in-memory twin that runs the same contract.

pub mod sqlite;
```

`crates/vpt-adapters/src/ledger/sqlite/migrations.rs`:

```rust
//! The versioned schema. Version 1 is the whole ledger of spec section 4.4;
//! a later stage adds columns with a new version, never by editing this one.

use rusqlite::{Connection, TransactionBehavior};
use vpt_application::ports::ledger::LedgerError;

pub const VERSION: u32 = 1;

const SCHEMA_V1: &str = "
CREATE TABLE seen (
  path TEXT PRIMARY KEY,
  file_name TEXT NOT NULL,
  size INTEGER NOT NULL,
  mtime_secs INTEGER NOT NULL,
  mtime_nanos INTEGER NOT NULL,
  dataless INTEGER NOT NULL,
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

fn current(connection: &Connection) -> Result<u32, LedgerError> {
    let version: u32 = connection.pragma_query_value(None, "user_version", |row| row.get(0)).map_err(super::map)?;
    if version > VERSION {
        return Err(LedgerError::UnsupportedSchema(version));
    }
    Ok(version)
}
```

`crates/vpt-adapters/src/ledger/sqlite/mod.rs`:

```rust
//! One SQLite database: WAL, a bounded busy timeout, restrictive modes, and
//! versioned migrations at open.

pub mod migrations;

use rusqlite::{Connection, ErrorCode, OpenFlags, Transaction, TransactionBehavior};
use std::fs::DirBuilder;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::Duration;
use vpt_application::ports::ledger::LedgerError;

pub struct SqliteLedger {
    path: PathBuf,
    busy_timeout: Duration,
}

impl SqliteLedger {
    pub const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

    pub fn open(state_dir: &Path) -> Result<SqliteLedger, LedgerError> {
        Self::open_with_timeout(state_dir, Self::BUSY_TIMEOUT)
    }

    pub fn open_with_timeout(state_dir: &Path, busy_timeout: Duration) -> Result<SqliteLedger, LedgerError> {
        DirBuilder::new().recursive(true).mode(0o700).create(state_dir).map_err(io)?;
        let ledger = SqliteLedger { path: state_dir.join("vpt.db"), busy_timeout };
        let mut connection = ledger.connect()?;
        std::fs::set_permissions(&ledger.path, std::fs::Permissions::from_mode(0o600)).map_err(io)?;
        migrations::migrate(&mut connection)?;
        Ok(ledger)
    }

    pub fn database_path(&self) -> &Path {
        &self.path
    }

    pub fn schema_version(&self) -> Result<u32, LedgerError> {
        self.read(|c| c.pragma_query_value(None, "user_version", |row| row.get(0)).map_err(map))
    }

    pub(crate) fn connect(&self) -> Result<Connection, LedgerError> {
        let flags = OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let connection = Connection::open_with_flags(&self.path, flags).map_err(map)?;
        connection.busy_timeout(self.busy_timeout).map_err(map)?;
        connection.pragma_update(None, "journal_mode", "WAL").map_err(map)?;
        connection.pragma_update(None, "foreign_keys", "ON").map_err(map)?;
        Ok(connection)
    }

    pub(crate) fn transaction<T>(&self, operation: impl FnOnce(&Transaction<'_>) -> Result<T, LedgerError>) -> Result<T, LedgerError> {
        let mut connection = self.connect()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(map)?;
        let value = operation(&transaction)?;
        transaction.commit().map_err(map)?;
        Ok(value)
    }

    pub(crate) fn read<T>(&self, operation: impl FnOnce(&Connection) -> Result<T, LedgerError>) -> Result<T, LedgerError> {
        let connection = self.connect()?;
        operation(&connection)
    }
}

pub(crate) fn map(error: rusqlite::Error) -> LedgerError {
    match &error {
        rusqlite::Error::SqliteFailure(failure, _) if failure.code == ErrorCode::DatabaseBusy => LedgerError::Busy,
        rusqlite::Error::SqliteFailure(failure, _) if failure.code == ErrorCode::ConstraintViolation => {
            LedgerError::Conflict(error.to_string())
        }
        rusqlite::Error::SqliteFailure(failure, _)
            if failure.code == ErrorCode::DatabaseCorrupt || failure.code == ErrorCode::NotADatabase =>
        {
            LedgerError::Corrupt(error.to_string())
        }
        _ => LedgerError::Io(error.to_string()),
    }
}

fn io(error: std::io::Error) -> LedgerError {
    LedgerError::Io(error.to_string())
}
```

`crates/vpt-adapters/src/lib.rs` gains `pub mod ledger;`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p vpt-adapters ledger`

Expected: 4 tests PASS. The busy test finishes in well under a second: the wait is 50 ms.

- [ ] **Step 5: Commit**

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(ledger): SQLite open with WAL, permissions and the version 1 schema"
```

______________________________________________________________________

### Task 12: The recording ledger and its in-memory twin, under one contract

**Files:**

- Modify: `crates/vpt-application/src/ports/ledger.rs`
- Create: `crates/vpt-adapters/src/ledger/sqlite/recordings.rs`,
  `crates/vpt-adapters/src/ledger/memory.rs`, `crates/vpt-adapters/src/ledger/contract.rs`
- Modify: `crates/vpt-adapters/src/ledger/mod.rs`, `crates/vpt-adapters/src/ledger/sqlite/mod.rs`

**Interfaces:**

- Consumes: `SqliteLedger::{transaction, read}`, `map`; domain `RecordingId`, `Sha256Digest`,
  `UtcInstant`, `UtcOffset`, `FileTime`, `DeferralReason`.

- Produces, in `vpt_application::ports::ledger`:

  - `SeenRow { pub path: PathBuf, pub file_name: String, pub size: u64, pub mtime: FileTime,`
    `pub dataless: bool, pub first_seen: UtcInstant, pub last_seen: UtcInstant,`
    `pub deferral_count: u32, pub deferral_reason: Option<DeferralReason>,`
    `pub deferred_size: Option<u64>, pub source_gone_at: Option<UtcInstant>,`
    `pub recording: Option<RecordingId> }`
  - `StageState::{Pending, Succeeded, Failed, Disabled, Expired}` with `as_str`/`parse`;
    `StageStates { pub transcribe: StageState, pub note: StageState, pub synthesis: StageState }` with
    `StageStates::fresh()` (all `Pending`).
  - `TitleOrigin::{VoiceMemos, Unavailable}` with `as_str`/`parse` (named apart from the recorder port's
    `TitleSource` trait of Task 15).
  - `RecordingRecord { pub id: RecordingId, pub source_path: Option<PathBuf>,`
    `pub digest: Sha256Digest, pub captured_at: UtcInstant, pub captured_offset: UtcOffset,`
    `pub duration_secs: u64, pub title: Option<String>, pub title_source: TitleOrigin,`
    `pub ingested_at: UtcInstant, pub audio_path: PathBuf, pub stages: StageStates,`
    `pub audio_trashed_at: Option<UtcInstant> }`
  - `trait RecordingLedger { fn seen(&self, path: &Path) -> Result<Option<SeenRow>,`
    `LedgerError>; fn seen_all(&self) -> Result<Vec<SeenRow>, LedgerError>;`
    `fn record_seen(&self, row: &SeenRow) -> Result<(), LedgerError>; fn by_digest(&self,`
    `digest: &Sha256Digest) -> Result<Option<RecordingRecord>, LedgerError>; fn by_id(&self,`
    `id: &RecordingId) -> Result<Option<RecordingRecord>, LedgerError>;`
    `fn recordings(&self) -> Result<Vec<RecordingRecord>, LedgerError>;`
    `fn commit_ingest(&self, recording: &RecordingRecord, seen: &SeenRow) -> Result<(),`
    `LedgerError>; fn commit_recovered(&self, recording: &RecordingRecord) -> Result<(),`
    `LedgerError>; fn set_source_path(&self, id: &RecordingId, path: &Path) -> Result<(),`
    `LedgerError>; fn set_audio_trashed(&self, id: &RecordingId, at: UtcInstant) ->`
    `Result<(), LedgerError>; }`

- `vpt_adapters::ledger::memory::MemoryLedger::new() -> MemoryLedger` (implements every ledger trait,
  state behind one `std::sync::Mutex`).

- `crates/vpt-adapters/src/ledger/contract.rs`: `pub(crate) mod recording` with one function per scenario
  taking `&dyn RecordingLedger`, and the macro `recording_ledger_contract!(make)` that emits one
  `#[test]` per scenario.

- [ ] **Step 1: Write the failing tests**

`crates/vpt-adapters/src/ledger/contract.rs`:

```rust
//! The behavioral contract every ledger implementation runs.

#![cfg(test)]

use std::path::{Path, PathBuf};
use vpt_application::ports::ledger::*;
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
        mtime: FileTime { secs: 1_787_690_856, nanos: 17 },
        dataless: false,
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
    let captured_at = UtcInstant { secs: 1_787_690_856 };
    let captured_offset = UtcOffset { secs: -21_600 };
    RecordingRecord {
        id: RecordingId::derive(captured_at, captured_offset, &digest),
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

pub mod recording {
    use super::*;

    pub fn seen_is_absent_until_recorded_and_updated_by_a_second_record(ledger: &dyn RecordingLedger) {
        assert_eq!(ledger.seen(Path::new("/vm/Recordings/a.m4a")).expect("read"), None);
        let mut row = seen_row("/vm/Recordings/a.m4a");
        ledger.record_seen(&row).expect("record");
        assert_eq!(ledger.seen(&row.path).expect("read"), Some(row.clone()));
        row.deferral_count = 1;
        row.deferral_reason = Some(DeferralReason::NotAtRest);
        row.deferred_size = Some(1_024);
        ledger.record_seen(&row).expect("update");
        assert_eq!(ledger.seen(&row.path).expect("read"), Some(row));
        assert_eq!(ledger.seen_all().expect("all").len(), 1);
    }

    pub fn commit_ingest_writes_the_recording_and_its_seen_row_together(ledger: &dyn RecordingLedger) {
        let recording = record(1);
        let mut seen = seen_row("/vm/Recordings/1.m4a");
        seen.recording = Some(recording.id.clone());
        ledger.commit_ingest(&recording, &seen).expect("commit");
        assert_eq!(ledger.by_id(&recording.id).expect("read"), Some(recording.clone()));
        assert_eq!(ledger.by_digest(&recording.digest).expect("read"), Some(recording.clone()));
        assert_eq!(ledger.seen(&seen.path).expect("read").and_then(|row| row.recording), Some(recording.id.clone()));
        assert_eq!(ledger.recordings().expect("list"), vec![recording]);
    }

    pub fn a_second_identity_for_one_digest_is_a_conflict_and_writes_nothing(ledger: &dyn RecordingLedger) {
        let first = record(2);
        ledger.commit_ingest(&first, &seen_row("/vm/Recordings/2.m4a")).expect("first");
        let mut second = record(2);
        second.id = RecordingId::parse("2026-08-24T144736-000000000002").expect("id");
        second.source_path = Some(PathBuf::from("/vm/Recordings/2b.m4a"));
        let outcome = ledger.commit_ingest(&second, &seen_row("/vm/Recordings/2b.m4a"));
        assert!(matches!(outcome, Err(LedgerError::Conflict(_))), "{outcome:?}");
        assert_eq!(ledger.seen(Path::new("/vm/Recordings/2b.m4a")).expect("read"), None);
        assert_eq!(ledger.recordings().expect("list").len(), 1);
    }

    pub fn the_source_path_and_the_audio_trashed_instant_are_updatable(ledger: &dyn RecordingLedger) {
        let recording = record(3);
        ledger.commit_ingest(&recording, &seen_row("/vm/Recordings/3.m4a")).expect("commit");
        ledger.set_source_path(&recording.id, Path::new("/vm/Recordings/renamed.m4a")).expect("path");
        ledger.set_audio_trashed(&recording.id, UtcInstant { secs: 1_787_800_000 }).expect("trashed");
        let stored = ledger.by_id(&recording.id).expect("read").expect("present");
        assert_eq!(stored.source_path, Some(PathBuf::from("/vm/Recordings/renamed.m4a")));
        assert_eq!(stored.audio_trashed_at, Some(UtcInstant { secs: 1_787_800_000 }));
    }

    pub fn an_unknown_identity_reads_as_absent(ledger: &dyn RecordingLedger) {
        let id = RecordingId::parse("2026-08-24T144736-ffffffffffff").expect("id");
        assert_eq!(ledger.by_id(&id).expect("read"), None);
        assert_eq!(ledger.by_digest(&digest(9)).expect("read"), None);
    }

    pub fn commit_recovered_writes_a_recording_with_no_source_and_no_seen_row(ledger: &dyn RecordingLedger) {
        let mut recording = record(4);
        recording.source_path = None;
        ledger.commit_recovered(&recording).expect("recovered");
        assert_eq!(ledger.by_id(&recording.id).expect("read"), Some(recording));
        assert!(ledger.seen_all().expect("all").is_empty());
    }
}

macro_rules! recording_ledger_contract {
    ($make:expr) => {
        mod recording_ledger_contract {
            use crate::ledger::contract::recording::*;

            #[test]
            fn seen_is_absent_until_recorded_and_updated_by_a_second_record_() {
                let (_guard, ledger) = $make();
                seen_is_absent_until_recorded_and_updated_by_a_second_record(&*ledger);
            }
            #[test]
            fn commit_ingest_writes_the_recording_and_its_seen_row_together_() {
                let (_guard, ledger) = $make();
                commit_ingest_writes_the_recording_and_its_seen_row_together(&*ledger);
            }
            #[test]
            fn a_second_identity_for_one_digest_is_a_conflict_and_writes_nothing_() {
                let (_guard, ledger) = $make();
                a_second_identity_for_one_digest_is_a_conflict_and_writes_nothing(&*ledger);
            }
            #[test]
            fn the_source_path_and_the_audio_trashed_instant_are_updatable_() {
                let (_guard, ledger) = $make();
                the_source_path_and_the_audio_trashed_instant_are_updatable(&*ledger);
            }
            #[test]
            fn an_unknown_identity_reads_as_absent_() {
                let (_guard, ledger) = $make();
                an_unknown_identity_reads_as_absent(&*ledger);
            }
            #[test]
            fn commit_recovered_writes_a_recording_with_no_source_and_no_seen_row_() {
                let (_guard, ledger) = $make();
                commit_recovered_writes_a_recording_with_no_source_and_no_seen_row(&*ledger);
            }
        }
    };
}
pub(crate) use recording_ledger_contract;
```

`$make` returns `(guard, Box<dyn RecordingLedger>)`; the guard keeps a temporary directory alive for the
SQLite case and is `()` for the memory case.

At the bottom of `crates/vpt-adapters/src/ledger/sqlite/mod.rs` (inside the existing `tests` module,
after the four tests):

```rust
    crate::ledger::contract::recording_ledger_contract!(|| {
        let temp = tempfile::tempdir().expect("temp");
        let ledger = SqliteLedger::open(temp.path()).expect("opens");
        (temp, Box::new(ledger) as Box<dyn vpt_application::ports::ledger::RecordingLedger>)
    });
```

`crates/vpt-adapters/src/ledger/memory.rs`, test section:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    crate::ledger::contract::recording_ledger_contract!(|| {
        ((), Box::new(MemoryLedger::new()) as Box<dyn vpt_application::ports::ledger::RecordingLedger>)
    });
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-adapters ledger`

Expected: compile errors: `SeenRow`, `RecordingRecord`, `RecordingLedger` and `MemoryLedger` not found.

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
    pub dataless: bool,
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

/// Where a title came from; named apart from the recorder port's `TitleSource` trait.
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

pub trait RecordingLedger {
    fn seen(&self, path: &Path) -> Result<Option<SeenRow>, LedgerError>;
    fn seen_all(&self) -> Result<Vec<SeenRow>, LedgerError>;
    fn record_seen(&self, row: &SeenRow) -> Result<(), LedgerError>;
    fn by_digest(&self, digest: &Sha256Digest) -> Result<Option<RecordingRecord>, LedgerError>;
    fn by_id(&self, id: &RecordingId) -> Result<Option<RecordingRecord>, LedgerError>;
    fn recordings(&self) -> Result<Vec<RecordingRecord>, LedgerError>;
    fn commit_ingest(&self, recording: &RecordingRecord, seen: &SeenRow) -> Result<(), LedgerError>;
    fn commit_recovered(&self, recording: &RecordingRecord) -> Result<(), LedgerError>;
    fn set_source_path(&self, id: &RecordingId, path: &Path) -> Result<(), LedgerError>;
    fn set_audio_trashed(&self, id: &RecordingId, at: UtcInstant) -> Result<(), LedgerError>;
}
```

`crates/vpt-adapters/src/ledger/sqlite/recordings.rs`:

```rust
//! `RecordingLedger` over SQLite: the `seen` and `recordings` tables.

use super::{SqliteLedger, map};
use rusqlite::{Connection, OptionalExtension, Row, params};
use std::path::{Path, PathBuf};
use vpt_application::ports::ledger::*;
use vpt_domain::digest::Sha256Digest;
use vpt_domain::identity::RecordingId;
use vpt_domain::sweep::DeferralReason;
use vpt_domain::time::{FileTime, UtcInstant, UtcOffset};

const RECORDING_COLUMNS: &str = "id, source_path, digest, captured_at, captured_offset, duration_secs, title, \
    title_source, ingested_at, audio_path, stage_transcribe, stage_note, stage_synthesis, audio_trashed_at";

fn corrupt(what: &str) -> LedgerError {
    LedgerError::Corrupt(format!("unreadable {what}"))
}

fn seen_from_row(row: &Row<'_>) -> Result<SeenRow, LedgerError> {
    let reason: Option<String> = row.get(9).map_err(map)?;
    let recording: Option<String> = row.get(12).map_err(map)?;
    Ok(SeenRow {
        path: PathBuf::from(row.get::<_, String>(0).map_err(map)?),
        file_name: row.get(1).map_err(map)?,
        size: row.get::<_, i64>(2).map_err(map)? as u64,
        mtime: FileTime { secs: row.get(3).map_err(map)?, nanos: row.get::<_, i64>(4).map_err(map)? as u32 },
        dataless: row.get::<_, i64>(5).map_err(map)? != 0,
        first_seen: UtcInstant { secs: row.get(6).map_err(map)? },
        last_seen: UtcInstant { secs: row.get(7).map_err(map)? },
        deferral_count: row.get::<_, i64>(8).map_err(map)? as u32,
        deferral_reason: reason.map(|text| DeferralReason::parse(&text).ok_or_else(|| corrupt("deferral reason"))).transpose()?,
        deferred_size: row.get::<_, Option<i64>>(10).map_err(map)?.map(|n| n as u64),
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
        captured_offset: UtcOffset { secs: row.get(4).map_err(map)? },
        duration_secs: row.get::<_, i64>(5).map_err(map)? as u64,
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
            "INSERT INTO seen (path, file_name, size, mtime_secs, mtime_nanos, dataless, first_seen, last_seen, \
             deferral_count, deferral_reason, deferred_size, source_gone_at, recording) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13) \
             ON CONFLICT(path) DO UPDATE SET file_name = excluded.file_name, size = excluded.size, \
             mtime_secs = excluded.mtime_secs, mtime_nanos = excluded.mtime_nanos, dataless = excluded.dataless, \
             last_seen = excluded.last_seen, deferral_count = excluded.deferral_count, \
             deferral_reason = excluded.deferral_reason, deferred_size = excluded.deferred_size, \
             source_gone_at = excluded.source_gone_at, recording = excluded.recording",
            params![
                row.path.to_string_lossy(),
                row.file_name,
                row.size as i64,
                row.mtime.secs,
                i64::from(row.mtime.nanos),
                i64::from(row.dataless),
                row.first_seen.secs,
                row.last_seen.secs,
                i64::from(row.deferral_count),
                row.deferral_reason.map(DeferralReason::as_str),
                row.deferred_size.map(|n| n as i64),
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
                recording.duration_secs as i64,
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

impl RecordingLedger for SqliteLedger {
    fn seen(&self, path: &Path) -> Result<Option<SeenRow>, LedgerError> {
        self.read(|c| {
            c.query_row(
                "SELECT path, file_name, size, mtime_secs, mtime_nanos, dataless, first_seen, last_seen, deferral_count, \
                 deferral_reason, deferred_size, source_gone_at, recording FROM seen WHERE path = ?1",
                [path.to_string_lossy()],
                |row| Ok(seen_from_row(row)),
            )
            .optional()
            .map_err(map)?
            .transpose()
        })
    }

    fn seen_all(&self) -> Result<Vec<SeenRow>, LedgerError> {
        self.read(|c| {
            let mut statement = c
                .prepare(
                    "SELECT path, file_name, size, mtime_secs, mtime_nanos, dataless, first_seen, last_seen, deferral_count, \
                     deferral_reason, deferred_size, source_gone_at, recording FROM seen ORDER BY path",
                )
                .map_err(map)?;
            let rows = statement.query_map([], |row| Ok(seen_from_row(row))).map_err(map)?;
            rows.map(|row| row.map_err(map)?).collect()
        })
    }

    fn record_seen(&self, row: &SeenRow) -> Result<(), LedgerError> {
        self.transaction(|t| upsert_seen(t, row))
    }

    fn by_digest(&self, digest: &Sha256Digest) -> Result<Option<RecordingRecord>, LedgerError> {
        self.read(|c| {
            c.query_row(&format!("SELECT {RECORDING_COLUMNS} FROM recordings WHERE digest = ?1"), [digest.hex()], |row| {
                Ok(record_from_row(row))
            })
            .optional()
            .map_err(map)?
            .transpose()
        })
    }

    fn by_id(&self, id: &RecordingId) -> Result<Option<RecordingRecord>, LedgerError> {
        self.read(|c| {
            c.query_row(&format!("SELECT {RECORDING_COLUMNS} FROM recordings WHERE id = ?1"), [id.as_str()], |row| {
                Ok(record_from_row(row))
            })
            .optional()
            .map_err(map)?
            .transpose()
        })
    }

    fn recordings(&self) -> Result<Vec<RecordingRecord>, LedgerError> {
        self.read(|c| {
            let mut statement = c.prepare(&format!("SELECT {RECORDING_COLUMNS} FROM recordings ORDER BY captured_at, id")).map_err(map)?;
            let rows = statement.query_map([], |row| Ok(record_from_row(row))).map_err(map)?;
            rows.map(|row| row.map_err(map)?).collect()
        })
    }

    fn commit_ingest(&self, recording: &RecordingRecord, seen: &SeenRow) -> Result<(), LedgerError> {
        self.transaction(|t| {
            insert_recording(t, recording)?;
            upsert_seen(t, seen)
        })
    }

    fn commit_recovered(&self, recording: &RecordingRecord) -> Result<(), LedgerError> {
        self.transaction(|t| insert_recording(t, recording))
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

Add `pub mod recordings;` to `crates/vpt-adapters/src/ledger/sqlite/mod.rs` and, in the `tests` module
there, the `recording_ledger_contract!` invocation shown in Step 1.

`crates/vpt-adapters/src/ledger/memory.rs`:

```rust
//! The in-memory ledger: the same contract as SQLite, for use-case tests.

use std::path::Path;
use std::sync::Mutex;
use vpt_application::ports::ledger::*;
use vpt_domain::digest::Sha256Digest;
use vpt_domain::identity::RecordingId;
use vpt_domain::time::UtcInstant;

#[derive(Default)]
struct State {
    seen: Vec<SeenRow>,
    recordings: Vec<RecordingRecord>,
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

fn upsert(seen: &mut Vec<SeenRow>, row: &SeenRow) {
    match seen.iter_mut().find(|existing| existing.path == row.path) {
        Some(existing) => *existing = row.clone(),
        None => seen.push(row.clone()),
    }
}

impl RecordingLedger for MemoryLedger {
    fn seen(&self, path: &Path) -> Result<Option<SeenRow>, LedgerError> {
        Ok(self.with(|s| s.seen.iter().find(|row| row.path == path).cloned()))
    }

    fn seen_all(&self) -> Result<Vec<SeenRow>, LedgerError> {
        Ok(self.with(|s| s.seen.clone()))
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

    fn commit_ingest(&self, recording: &RecordingRecord, seen: &SeenRow) -> Result<(), LedgerError> {
        self.with(|s| {
            if s.recordings.iter().any(|r| r.digest == recording.digest || r.id == recording.id) {
                return Err(LedgerError::Conflict("digest or identity already recorded".into()));
            }
            s.recordings.push(recording.clone());
            upsert(&mut s.seen, seen);
            Ok(())
        })
    }

    fn commit_recovered(&self, recording: &RecordingRecord) -> Result<(), LedgerError> {
        self.with(|s| {
            if s.recordings.iter().any(|r| r.digest == recording.digest || r.id == recording.id) {
                return Err(LedgerError::Conflict("digest or identity already recorded".into()));
            }
            s.recordings.push(recording.clone());
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
```

`crates/vpt-adapters/src/ledger/mod.rs`:

```rust
//! The ledger: one SQLite type implementing every repository, and an
//! in-memory twin that runs the same contract.

#[cfg(test)]
pub(crate) mod contract;
pub mod memory;
pub mod sqlite;
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p vpt-adapters ledger`

Expected: the four open tests plus five contract tests per implementation, all PASS.

- [ ] **Step 5: Commit**

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(ledger): the recording ledger over SQLite and in memory under one contract"
```

______________________________________________________________________

### Task 13: The write lock

**Files:**

- Create: `crates/vpt-adapters/src/lock.rs`
- Modify: `crates/vpt-adapters/src/lib.rs`

**Interfaces:**

- Consumes: nothing.

- Produces: `vpt_adapters::lock::{WriteLock, LockError::{Busy, Io(String)}}` with
  `WriteLock::acquire(state_dir: &Path, wait: Duration) -> Result<WriteLock, LockError>`; dropping the
  value releases the lock.

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn a_second_acquisition_waits_the_bounded_time_then_reports_busy() {
        let temp = tempfile::tempdir().expect("temp");
        let held = WriteLock::acquire(temp.path(), Duration::from_millis(50)).expect("first");

        let started = Instant::now();
        let outcome = WriteLock::acquire(temp.path(), Duration::from_millis(100));

        assert_eq!(outcome.err(), Some(LockError::Busy));
        assert!(started.elapsed() >= Duration::from_millis(100));
        assert!(started.elapsed() < Duration::from_millis(900));
        drop(held);
    }

    #[test]
    fn dropping_the_lock_lets_the_next_acquisition_through() {
        let temp = tempfile::tempdir().expect("temp");
        drop(WriteLock::acquire(temp.path(), Duration::from_millis(50)).expect("first"));
        assert!(WriteLock::acquire(temp.path(), Duration::from_millis(50)).is_ok());
        assert!(temp.path().join("write.lock").exists());
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-adapters lock`

Expected: compile error, `WriteLock` not found.

- [ ] **Step 3: Write the minimal implementation**

```rust
//! The advisory write lock every mutating command holds: `flock` on
//! `<state_dir>/write.lock`, close-on-exec, a bounded wait.

use std::fs::{File, OpenOptions};
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::time::{Duration, Instant};

#[derive(Debug, PartialEq, Eq)]
pub enum LockError {
    Busy,
    Io(String),
}

pub struct WriteLock {
    file: File,
}

impl WriteLock {
    pub fn acquire(state_dir: &Path, wait: Duration) -> Result<WriteLock, LockError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(state_dir.join("write.lock"))
            .map_err(|error| LockError::Io(error.to_string()))?;
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
            std::thread::sleep(Duration::from_millis(25));
        }
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

Add `pub mod lock;` to `lib.rs`. Rust opens files with `O_CLOEXEC`, which is the close-on-exec the spec
asks for.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p vpt-adapters lock`

Expected: 2 tests PASS, each in under 200 ms.

- [ ] **Step 5: Commit**

```bash
git add crates/vpt-adapters
SKIP_AI_COMMIT=1 git commit -m "feat(ledger): the bounded advisory write lock"
```

______________________________________________________________________

### Task 14: The publication journal and repair

Every mutating command repairs unfinished publications before new work (spec section 4.4). Stage 1
publishes no rendered artifact of its own, so its composition installs an empty renderer registry; stage
3 registers the note renderer. The journal rows, the repair decision and the filesystem operations are
all built and tested here.

**Files:**

- Modify: `crates/vpt-application/src/ports/ledger.rs`
- Create: `crates/vpt-application/src/ports/artifacts.rs`, `crates/vpt-application/src/publication.rs`
- Modify: `crates/vpt-application/src/ports/mod.rs`, `crates/vpt-application/src/lib.rs`
- Create: `crates/vpt-adapters/src/ledger/sqlite/journal.rs`, `crates/vpt-adapters/src/stores.rs`
- Modify: `crates/vpt-adapters/src/ledger/memory.rs`, `crates/vpt-adapters/src/ledger/contract.rs`,
  `crates/vpt-adapters/src/ledger/sqlite/mod.rs`, `crates/vpt-adapters/src/lib.rs`

**Interfaces:**

- Consumes: `LedgerError`, `SqliteLedger::{transaction, read}`, `MemoryLedger`.

- Produces:

  - `vpt_application::ports::ledger::{DirtyPublication { pub target: PathBuf,`
    `pub expected_previous: Option<Sha256Digest>, pub intended: Sha256Digest,`
    `pub recorded_at: UtcInstant }, trait PublicationJournal { fn record_publication(&self,`
    `entry: &DirtyPublication) -> Result<(), LedgerError>; fn pending_publications(&self) ->`
    `Result<Vec<DirtyPublication>, LedgerError>; fn clear_publication(&self,`
    `target: &Path) -> Result<(), LedgerError>; }}`
  - `vpt_application::ports::artifacts::{ArtifactError(String),`
    `trait ArtifactFiles { fn digest_of(&self, path: &Path) -> Result<Option<Sha256Digest>,`
    `ArtifactError>; fn publish(&self, target: &Path, bytes: &[u8]) -> Result<(),`
    `ArtifactError>; fn sync_directory_of(&self, path: &Path) -> Result<(), ArtifactError>;`
    `}, RenderError::NoRendererFor(PathBuf), trait ArtifactRenderer { fn render(&self,`
    `target: &Path) -> Result<Vec<u8>, RenderError>; }, NoRenderers}` (the empty registry, implements
    `ArtifactRenderer`).
  - `vpt_application::publication::{repair_publications(journal: &dyn PublicationJournal,`
    `files: &dyn ArtifactFiles, renderer: &dyn ArtifactRenderer) -> Result<RepairReport,`
    `RepairError>, RepairReport { pub completed: Vec<PathBuf>,`
    `pub republished: Vec<PathBuf> }, RepairError::{TargetModified(PathBuf),`
    `Sync { path: PathBuf, detail: String }, Render(PathBuf), Ledger(LedgerError),`
    `Files(ArtifactError)}}`.
  - `vpt_adapters::stores::FilesystemStores` implementing `ArtifactFiles` (and, from Task 26 on,
    `Stores`).

- [ ] **Step 1: Write the failing tests**

`crates/vpt-application/src/publication.rs`, test section:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
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
        fn record_publication(&self, entry: &DirtyPublication) -> Result<(), LedgerError> {
            self.0.borrow_mut().push(entry.clone());
            Ok(())
        }
        fn pending_publications(&self) -> Result<Vec<DirtyPublication>, LedgerError> {
            Ok(self.0.borrow().clone())
        }
        fn clear_publication(&self, target: &Path) -> Result<(), LedgerError> {
            self.0.borrow_mut().retain(|entry| entry.target != target);
            Ok(())
        }
    }

    struct FakeFiles {
        contents: RefCell<HashMap<PathBuf, Vec<u8>>>,
        synced: RefCell<Vec<PathBuf>>,
        sync_fails: bool,
    }
    impl ArtifactFiles for FakeFiles {
        fn digest_of(&self, path: &Path) -> Result<Option<Sha256Digest>, ArtifactError> {
            Ok(self.contents.borrow().get(path).map(|bytes| digest_of_bytes(bytes)))
        }
        fn publish(&self, target: &Path, bytes: &[u8]) -> Result<(), ArtifactError> {
            self.contents.borrow_mut().insert(target.to_path_buf(), bytes.to_vec());
            Ok(())
        }
        fn sync_directory_of(&self, path: &Path) -> Result<(), ArtifactError> {
            if self.sync_fails {
                return Err(ArtifactError("disk gone".into()));
            }
            self.synced.borrow_mut().push(path.to_path_buf());
            Ok(())
        }
    }

    struct FakeRenderer(Vec<u8>);
    impl ArtifactRenderer for FakeRenderer {
        fn render(&self, _target: &Path) -> Result<Vec<u8>, RenderError> {
            Ok(self.0.clone())
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

    fn files(contents: &[(&str, &[u8])], sync_fails: bool) -> FakeFiles {
        FakeFiles {
            contents: RefCell::new(contents.iter().map(|(p, b)| (PathBuf::from(p), b.to_vec())).collect()),
            synced: RefCell::new(vec![]),
            sync_fails,
        }
    }

    #[test]
    fn a_target_already_holding_the_intended_bytes_is_completed_after_a_directory_sync() {
        let journal = FakeJournal(RefCell::new(vec![entry("/t/a.md", Some(b"old"), b"new")]));
        let files = files(&[("/t/a.md", b"new")], false);
        let report = repair_publications(&journal, &files, &NoRenderers).expect("repaired");
        assert_eq!(report.completed, vec![PathBuf::from("/t/a.md")]);
        assert_eq!(files.synced.borrow().as_slice(), [PathBuf::from("/t/a.md")]);
        assert!(journal.0.borrow().is_empty());
    }

    #[test]
    fn a_failed_directory_sync_leaves_the_entry_pending() {
        let journal = FakeJournal(RefCell::new(vec![entry("/t/a.md", Some(b"old"), b"new")]));
        let files = files(&[("/t/a.md", b"new")], true);
        let error = repair_publications(&journal, &files, &NoRenderers).unwrap_err();
        assert!(matches!(error, RepairError::Sync { .. }), "{error:?}");
        assert_eq!(journal.0.borrow().len(), 1);
    }

    #[test]
    fn a_target_holding_the_expected_previous_bytes_is_published_over_from_the_renderer() {
        let journal = FakeJournal(RefCell::new(vec![entry("/t/a.md", Some(b"old"), b"new")]));
        let files = files(&[("/t/a.md", b"old")], false);
        let report = repair_publications(&journal, &files, &FakeRenderer(b"new".to_vec())).expect("repaired");
        assert_eq!(report.republished, vec![PathBuf::from("/t/a.md")]);
        assert_eq!(files.contents.borrow()[Path::new("/t/a.md")], b"new");
        assert!(journal.0.borrow().is_empty());
    }

    #[test]
    fn an_absent_target_expected_absent_is_published() {
        let journal = FakeJournal(RefCell::new(vec![entry("/t/b.md", None, b"fresh")]));
        let files = files(&[], false);
        let report = repair_publications(&journal, &files, &FakeRenderer(b"fresh".to_vec())).expect("repaired");
        assert_eq!(report.republished, vec![PathBuf::from("/t/b.md")]);
    }

    #[test]
    fn any_other_bytes_are_refused_as_target_modified_and_nothing_is_overwritten() {
        let journal = FakeJournal(RefCell::new(vec![entry("/t/a.md", Some(b"old"), b"new")]));
        let files = files(&[("/t/a.md", b"someone else's prose")], false);
        let error = repair_publications(&journal, &files, &FakeRenderer(b"new".to_vec())).unwrap_err();
        assert_eq!(error, RepairError::TargetModified(PathBuf::from("/t/a.md")));
        assert_eq!(files.contents.borrow()[Path::new("/t/a.md")], b"someone else's prose");
        assert_eq!(journal.0.borrow().len(), 1);
    }

    #[test]
    fn a_target_no_renderer_knows_is_reported_and_left_pending() {
        let journal = FakeJournal(RefCell::new(vec![entry("/t/a.md", Some(b"old"), b"new")]));
        let files = files(&[("/t/a.md", b"old")], false);
        let error = repair_publications(&journal, &files, &NoRenderers).unwrap_err();
        assert_eq!(error, RepairError::Render(PathBuf::from("/t/a.md")));
    }
}
```

Contract scenarios appended to `crates/vpt-adapters/src/ledger/contract.rs`:

```rust
pub mod journal {
    use super::*;

    pub fn a_recorded_publication_is_pending_until_cleared(ledger: &dyn PublicationJournal) {
        let entry = DirtyPublication {
            target: PathBuf::from("/h/transcripts/a.md"),
            expected_previous: None,
            intended: digest(4),
            recorded_at: UtcInstant { secs: 1 },
        };
        ledger.record_publication(&entry).expect("record");
        assert_eq!(ledger.pending_publications().expect("pending"), vec![entry.clone()]);
        ledger.clear_publication(&entry.target).expect("clear");
        assert_eq!(ledger.pending_publications().expect("pending"), vec![]);
    }

    pub fn recording_the_same_target_twice_keeps_the_latest_entry(ledger: &dyn PublicationJournal) {
        let mut entry = DirtyPublication {
            target: PathBuf::from("/h/transcripts/a.md"),
            expected_previous: Some(digest(5)),
            intended: digest(6),
            recorded_at: UtcInstant { secs: 1 },
        };
        ledger.record_publication(&entry).expect("first");
        entry.intended = digest(7);
        ledger.record_publication(&entry).expect("second");
        assert_eq!(ledger.pending_publications().expect("pending"), vec![entry]);
    }
}

macro_rules! publication_journal_contract {
    ($make:expr) => {
        mod publication_journal_contract {
            use crate::ledger::contract::journal::*;

            #[test]
            fn a_recorded_publication_is_pending_until_cleared_() {
                let (_guard, ledger) = $make();
                a_recorded_publication_is_pending_until_cleared(&*ledger);
            }
            #[test]
            fn recording_the_same_target_twice_keeps_the_latest_entry_() {
                let (_guard, ledger) = $make();
                recording_the_same_target_twice_keeps_the_latest_entry(&*ledger);
            }
        }
    };
}
pub(crate) use publication_journal_contract;
```

with the invocation in both implementations' test modules, the closure boxing as
`Box<dyn vpt_application::ports::ledger::PublicationJournal>`.

`crates/vpt-adapters/src/stores.rs`, test section:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn digest_of_reads_the_file_and_reports_absence_as_none() {
        let temp = tempfile::tempdir().expect("temp");
        let path = temp.path().join("a.md");
        std::fs::write(&path, b"hello").expect("write");
        let stores = FilesystemStores;
        let digest = stores.digest_of(&path).expect("digest").expect("present");
        assert_eq!(digest.hex(), "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824");
        assert_eq!(stores.digest_of(&temp.path().join("missing")).expect("absent"), None);
    }

    #[test]
    fn publish_replaces_the_target_atomically_with_mode_0644_and_leaves_no_temporary_name() {
        let temp = tempfile::tempdir().expect("temp");
        let target = temp.path().join("a.md");
        std::fs::write(&target, b"old").expect("old");
        FilesystemStores.publish(&target, b"new").expect("publish");
        assert_eq!(std::fs::read(&target).expect("read"), b"new");
        let names: Vec<_> = std::fs::read_dir(temp.path()).expect("dir").map(|e| e.expect("entry").file_name()).collect();
        assert_eq!(names, vec![std::ffi::OsString::from("a.md")]);
        assert_eq!(std::fs::metadata(&target).expect("meta").permissions().mode() & 0o777, 0o644);
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-application publication && cargo test -p vpt-adapters`

Expected: compile errors naming `DirtyPublication`, `PublicationJournal`, `repair_publications`,
`FilesystemStores`.

- [ ] **Step 3: Write the minimal implementation**

Append to `crates/vpt-application/src/ports/ledger.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirtyPublication {
    pub target: PathBuf,
    pub expected_previous: Option<Sha256Digest>,
    pub intended: Sha256Digest,
    pub recorded_at: UtcInstant,
}

pub trait PublicationJournal {
    fn record_publication(&self, entry: &DirtyPublication) -> Result<(), LedgerError>;
    fn pending_publications(&self) -> Result<Vec<DirtyPublication>, LedgerError>;
    fn clear_publication(&self, target: &Path) -> Result<(), LedgerError>;
}
```

`crates/vpt-application/src/ports/artifacts.rs`:

```rust
//! The filesystem operations a publication needs, and the renderer registry
//! that re-creates an artifact from committed state.

use std::path::{Path, PathBuf};
use vpt_domain::digest::Sha256Digest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactError(pub String);

pub trait ArtifactFiles {
    /// `None` when the path is absent.
    fn digest_of(&self, path: &Path) -> Result<Option<Sha256Digest>, ArtifactError>;
    /// Write to a temporary name beside the target, sync, rename over the target.
    fn publish(&self, target: &Path, bytes: &[u8]) -> Result<(), ArtifactError>;
    fn sync_directory_of(&self, path: &Path) -> Result<(), ArtifactError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderError {
    NoRendererFor(PathBuf),
}

pub trait ArtifactRenderer {
    fn render(&self, target: &Path) -> Result<Vec<u8>, RenderError>;
}

/// The registry with no artifact kinds registered: stage 1 publishes no
/// rendered artifact, so every target is unknown to it.
pub struct NoRenderers;

impl ArtifactRenderer for NoRenderers {
    fn render(&self, target: &Path) -> Result<Vec<u8>, RenderError> {
        Err(RenderError::NoRendererFor(target.to_path_buf()))
    }
}
```

`crates/vpt-application/src/publication.rs`:

```rust
//! Repair unfinished publications before new work, per spec section 4.4.

use crate::ports::artifacts::{ArtifactError, ArtifactFiles, ArtifactRenderer, RenderError};
use crate::ports::ledger::{DirtyPublication, LedgerError, PublicationJournal};
use std::path::{Path, PathBuf};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct RepairReport {
    pub completed: Vec<PathBuf>,
    pub republished: Vec<PathBuf>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RepairError {
    TargetModified(PathBuf),
    Sync { path: PathBuf, detail: String },
    Render(PathBuf),
    Ledger(LedgerError),
    Files(ArtifactError),
}

pub fn repair_publications(
    journal: &dyn PublicationJournal,
    files: &dyn ArtifactFiles,
    renderer: &dyn ArtifactRenderer,
) -> Result<RepairReport, RepairError> {
    let mut report = RepairReport::default();
    for entry in journal.pending_publications().map_err(RepairError::Ledger)? {
        let current = files.digest_of(&entry.target).map_err(RepairError::Files)?;
        if current == Some(entry.intended) {
            sync(files, &entry.target)?;
            journal.clear_publication(&entry.target).map_err(RepairError::Ledger)?;
            report.completed.push(entry.target);
        } else if current == entry.expected_previous {
            let bytes = renderer.render(&entry.target).map_err(|RenderError::NoRendererFor(path)| RepairError::Render(path))?;
            files.publish(&entry.target, &bytes).map_err(RepairError::Files)?;
            sync(files, &entry.target)?;
            journal.clear_publication(&entry.target).map_err(RepairError::Ledger)?;
            report.republished.push(entry.target);
        } else {
            return Err(RepairError::TargetModified(entry.target));
        }
    }
    Ok(report)
}

fn sync(files: &dyn ArtifactFiles, path: &Path) -> Result<(), RepairError> {
    files
        .sync_directory_of(path)
        .map_err(|ArtifactError(detail)| RepairError::Sync { path: path.to_path_buf(), detail })
}
```

`crates/vpt-application/src/ports/mod.rs` lists `artifacts`, `ledger`, `prompt`;
`crates/vpt-application/src/lib.rs` adds `pub mod publication;`.

`crates/vpt-adapters/src/ledger/sqlite/journal.rs`:

```rust
//! `PublicationJournal` over SQLite: the `dirty_publications` table.

use super::{SqliteLedger, map};
use rusqlite::params;
use std::path::{Path, PathBuf};
use vpt_application::ports::ledger::{DirtyPublication, LedgerError, PublicationJournal};
use vpt_domain::digest::Sha256Digest;
use vpt_domain::time::UtcInstant;

impl PublicationJournal for SqliteLedger {
    fn record_publication(&self, entry: &DirtyPublication) -> Result<(), LedgerError> {
        self.transaction(|t| {
            t.execute(
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
        })
    }

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
                    expected_previous: previous
                        .map(|hex| Sha256Digest::from_hex(&hex).ok_or_else(|| LedgerError::Corrupt("expected digest".into())))
                        .transpose()?,
                    intended: Sha256Digest::from_hex(&intended).ok_or_else(|| LedgerError::Corrupt("intended digest".into()))?,
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

Add `pub mod journal;` to the sqlite `mod.rs`. In `memory.rs`, add `publications: Vec<DirtyPublication>`
to `State` and:

```rust
impl PublicationJournal for MemoryLedger {
    fn record_publication(&self, entry: &DirtyPublication) -> Result<(), LedgerError> {
        self.with(|s| {
            s.publications.retain(|existing| existing.target != entry.target);
            s.publications.push(entry.clone());
        });
        Ok(())
    }

    fn pending_publications(&self) -> Result<Vec<DirtyPublication>, LedgerError> {
        Ok(self.with(|s| s.publications.clone()))
    }

    fn clear_publication(&self, target: &Path) -> Result<(), LedgerError> {
        self.with(|s| s.publications.retain(|existing| existing.target != target));
        Ok(())
    }
}
```

`crates/vpt-adapters/src/stores.rs`:

```rust
//! Filesystem operations over the stores: digests, atomic publication,
//! directory syncs.

use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use vpt_application::ports::artifacts::{ArtifactError, ArtifactFiles};
use vpt_domain::digest::Sha256Digest;

pub struct FilesystemStores;

pub const BUFFER: usize = 64 * 1024;

pub fn digest_file(path: &Path) -> Result<Sha256Digest, std::io::Error> {
    let mut file = File::open(path)?;
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

fn io(error: std::io::Error) -> ArtifactError {
    ArtifactError(error.to_string())
}

impl ArtifactFiles for FilesystemStores {
    fn digest_of(&self, path: &Path) -> Result<Option<Sha256Digest>, ArtifactError> {
        match digest_file(path) {
            Ok(digest) => Ok(Some(digest)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(io(error)),
        }
    }

    fn publish(&self, target: &Path, bytes: &[u8]) -> Result<(), ArtifactError> {
        let directory = target.parent().ok_or_else(|| ArtifactError("target has no parent".into()))?;
        let name = target.file_name().ok_or_else(|| ArtifactError("target has no name".into()))?;
        let temporary = directory.join(format!(".{}.vpt-{}", name.to_string_lossy(), std::process::id()));
        let mut file = OpenOptions::new().write(true).create_new(true).mode(0o644).open(&temporary).map_err(io)?;
        file.write_all(bytes).map_err(io)?;
        file.sync_all().map_err(io)?;
        std::fs::rename(&temporary, target).map_err(io)?;
        Ok(())
    }

    fn sync_directory_of(&self, path: &Path) -> Result<(), ArtifactError> {
        let directory = path.parent().ok_or_else(|| ArtifactError("path has no parent".into()))?;
        File::open(directory).and_then(|dir| dir.sync_all()).map_err(io)
    }
}
```

Add `pub mod stores;` to `crates/vpt-adapters/src/lib.rs`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --workspace --features dev-tools`

Expected: all PASS, the two journal contract tests per implementation included.

- [ ] **Step 5: Commit**

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(ledger): the publication journal and the repair before new work"
```

______________________________________________________________________

### Task 15: The Voice Memos store, read-only

**Files:**

- Create: `crates/vpt-application/src/ports/recorder.rs`
- Modify: `crates/vpt-application/src/ports/mod.rs`
- Create: `crates/vpt-adapters/src/voice_memos/mod.rs`, `crates/vpt-adapters/src/voice_memos/store.rs`
- Modify: `crates/vpt-adapters/src/lib.rs`

**Interfaces:**

- Consumes: `vpt_domain::container::{BoxReader, ReadFailure}`, `vpt_domain::time::FileTime`.

- Produces, in `vpt_application::ports::recorder`:

  - `Candidate { pub path: PathBuf, pub file_name: String, pub size: u64,`
    `pub mtime: FileTime, pub dataless: bool }`;
    `SourceMetadata { pub device: u64, pub inode: u64, pub size: u64, pub mtime: FileTime }`;
    `RecorderError::{Unreadable(String), NotRegular(PathBuf), NotFound(PathBuf), Io(String)}`;
    `CloneKind::{CopyOnWrite, ByteCopy}`; `CloneError::{NoSpace, Io(String)}`;
    `TitleLookup::{Titled(String), Unavailable}`.
  - `trait SourceHandle: BoxReader { fn metadata(&self) -> Result<SourceMetadata,`
    `RecorderError>; fn clone_into(&self, destination: &Path) -> Result<CloneKind,` `CloneError>; }`
  - `trait TitleSource { fn title(&self, file_name: &str) -> TitleLookup; }`
  - `trait RecorderStore { fn candidates(&self) -> Result<Vec<Candidate>, RecorderError>;`
    `fn candidate(&self, path: &Path) -> Result<Candidate, RecorderError>; fn open(&self,`
    `path: &Path) -> Result<Box<dyn SourceHandle>, RecorderError>; fn titles(&self) ->`
    `Box<dyn TitleSource>; fn subdirectory_counts(&self) -> Vec<(String, Option<u64>)>; }`

- `vpt_adapters::voice_memos::store::VoiceMemosStore::new(recordings_dir: PathBuf,`
  `state_dir: PathBuf, read_titles: bool) -> VoiceMemosStore`;
  `pub const APPLE_SUBDIRECTORIES: [&str; 4]`.

- [ ] **Step 1: Write the failing tests**

`crates/vpt-adapters/src/voice_memos/store.rs`, test section:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use vpt_domain::fixtures::m4a;

    fn store_with(files: &[(&str, &[u8])]) -> (tempfile::TempDir, VoiceMemosStore) {
        let temp = tempfile::tempdir().expect("temp");
        let recordings = temp.path().join("Recordings");
        std::fs::create_dir_all(&recordings).expect("recordings");
        for subdirectory in APPLE_SUBDIRECTORIES {
            std::fs::create_dir_all(recordings.join(subdirectory)).expect("subdirectory");
            std::fs::write(recordings.join(subdirectory).join("inner.m4a"), b"never listed").expect("inner");
        }
        for (name, bytes) in files {
            std::fs::write(recordings.join(name), bytes).expect("fixture");
        }
        let store = VoiceMemosStore::new(recordings, temp.path().join("state"), true);
        (temp, store)
    }

    #[test]
    fn candidates_are_the_m4a_entries_at_depth_one_and_nothing_else() {
        let (_temp, store) = store_with(&[("a.m4a", b"aaa"), ("b.M4A", b"bbb"), ("a.waveform", b"w"), ("notes.txt", b"n")]);
        let mut names: Vec<String> = store.candidates().expect("list").into_iter().map(|c| c.file_name).collect();
        names.sort();
        assert_eq!(names, vec!["a.m4a"]);
    }

    #[test]
    fn a_candidate_carries_size_mtime_and_a_false_dataless_flag_for_a_plain_file() {
        let (_temp, store) = store_with(&[("a.m4a", b"aaaa")]);
        let candidate = store.candidates().expect("list").remove(0);
        assert_eq!(candidate.size, 4);
        assert!(candidate.mtime.secs > 0);
        assert!(!candidate.dataless);
    }

    #[test]
    fn open_refuses_a_symbolic_link_and_reads_a_regular_file_by_descriptor() {
        let (temp, store) = store_with(&[("a.m4a", b"hello world")]);
        let link = temp.path().join("Recordings/link.m4a");
        std::os::unix::fs::symlink(temp.path().join("Recordings/a.m4a"), &link).expect("link");
        assert!(matches!(store.open(&link), Err(RecorderError::NotRegular(_))));
        let mut handle = store.open(&temp.path().join("Recordings/a.m4a")).expect("open");
        let mut buffer = [0u8; 5];
        handle.read_exact_at(6, &mut buffer).expect("read");
        assert_eq!(&buffer, b"world");
        assert_eq!(handle.len(), 11);
        let metadata = handle.metadata().expect("metadata");
        assert_eq!(metadata.size, 11);
        assert!(metadata.inode > 0);
    }

    #[test]
    fn clone_into_reproduces_the_bytes_and_reports_copy_on_write_on_the_same_volume() {
        let (temp, store) = store_with(&[("a.m4a", &m4a(1_787_690_856, 3, b"payload"))]);
        let handle = store.open(&temp.path().join("Recordings/a.m4a")).expect("open");
        let destination = temp.path().join("staged");
        let kind = handle.clone_into(&destination).expect("clone");
        assert_eq!(std::fs::read(&destination).expect("read"), m4a(1_787_690_856, 3, b"payload"));
        assert_eq!(kind, CloneKind::CopyOnWrite);
    }

    #[test]
    fn subdirectory_counts_report_each_apple_directory_without_entering_it_for_candidates() {
        let (_temp, store) = store_with(&[]);
        let counts = store.subdirectory_counts();
        assert_eq!(counts.len(), 4);
        assert!(counts.iter().all(|(_, count)| *count == Some(1)), "{counts:?}");
    }

    #[test]
    fn a_missing_recordings_directory_is_unreadable() {
        let store = VoiceMemosStore::new(PathBuf::from("/nonexistent/vpt-test"), PathBuf::from("/tmp"), true);
        assert!(matches!(store.candidates(), Err(RecorderError::Unreadable(_))));
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-adapters voice_memos`

Expected: compile error, `VoiceMemosStore` not found.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-application/src/ports/recorder.rs`:

```rust
//! The recorder store: list, open read-only, clone from a descriptor, titles.

use std::path::{Path, PathBuf};
use vpt_domain::container::BoxReader;
use vpt_domain::time::FileTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub path: PathBuf,
    pub file_name: String,
    pub size: u64,
    pub mtime: FileTime,
    pub dataless: bool,
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
    Io(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TitleLookup {
    Titled(String),
    Unavailable,
}

pub trait SourceHandle: BoxReader {
    fn metadata(&self) -> Result<SourceMetadata, RecorderError>;
    fn clone_into(&self, destination: &Path) -> Result<CloneKind, CloneError>;
}

pub trait TitleSource {
    fn title(&self, file_name: &str) -> TitleLookup;
}

pub trait RecorderStore {
    fn candidates(&self) -> Result<Vec<Candidate>, RecorderError>;
    fn candidate(&self, path: &Path) -> Result<Candidate, RecorderError>;
    fn open(&self, path: &Path) -> Result<Box<dyn SourceHandle>, RecorderError>;
    /// Performs the private database copy; a copy that fails answers `Unavailable`.
    fn titles(&self) -> Box<dyn TitleSource>;
    fn subdirectory_counts(&self) -> Vec<(String, Option<u64>)>;
}
```

`crates/vpt-adapters/src/voice_memos/mod.rs`:

```rust
//! Apple's Voice Memos store, read-only: the listing, the descriptors and
//! the private copy of its database.

pub mod store;
pub mod titles;
```

(`titles` arrives in the next task; until then leave the line out.)

`crates/vpt-adapters/src/voice_memos/store.rs`:

```rust
//! The listing at depth one and read-only descriptors that never follow a link.

use std::fs::{File, OpenOptions};
use std::os::macos::fs::MetadataExt;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use vpt_application::ports::recorder::*;
use vpt_domain::container::{BoxReader, ReadFailure};
use vpt_domain::time::FileTime;

pub const APPLE_SUBDIRECTORIES: [&str; 4] = ["Capture", "CaptureRecovery", "CloudRecordings_ckAssets", "EncryptedCloudRecordings"];

/// The `st_flags` bit macOS sets on a file whose bytes are still in iCloud.
const SF_DATALESS: u32 = 0x4000_0000;

pub struct VoiceMemosStore {
    recordings_dir: PathBuf,
    state_dir: PathBuf,
    read_titles: bool,
}

impl VoiceMemosStore {
    pub fn new(recordings_dir: PathBuf, state_dir: PathBuf, read_titles: bool) -> VoiceMemosStore {
        VoiceMemosStore { recordings_dir, state_dir, read_titles }
    }

    pub fn recordings_dir(&self) -> &Path {
        &self.recordings_dir
    }

    fn candidate_from(&self, path: PathBuf) -> Result<Option<Candidate>, RecorderError> {
        let file_name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if !file_name.ends_with(".m4a") {
            return Ok(None);
        }
        let metadata = std::fs::symlink_metadata(&path).map_err(|error| RecorderError::Io(error.to_string()))?;
        if !metadata.file_type().is_file() {
            return Ok(None);
        }
        Ok(Some(Candidate {
            file_name,
            size: metadata.len(),
            mtime: FileTime { secs: metadata.st_mtime(), nanos: metadata.st_mtime_nsec() as u32 },
            dataless: metadata.st_flags() & SF_DATALESS != 0,
            path,
        }))
    }
}

impl RecorderStore for VoiceMemosStore {
    fn candidates(&self) -> Result<Vec<Candidate>, RecorderError> {
        let entries = std::fs::read_dir(&self.recordings_dir).map_err(|error| RecorderError::Unreadable(error.to_string()))?;
        let mut candidates = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| RecorderError::Unreadable(error.to_string()))?;
            if let Some(candidate) = self.candidate_from(entry.path())? {
                candidates.push(candidate);
            }
        }
        candidates.sort_by(|a, b| a.file_name.cmp(&b.file_name));
        Ok(candidates)
    }

    fn candidate(&self, path: &Path) -> Result<Candidate, RecorderError> {
        match self.candidate_from(path.to_path_buf())? {
            Some(candidate) => Ok(candidate),
            None => Err(RecorderError::NotFound(path.to_path_buf())),
        }
    }

    fn open(&self, path: &Path) -> Result<Box<dyn SourceHandle>, RecorderError> {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)
            .map_err(|error| {
                if error.raw_os_error() == Some(libc::ELOOP) {
                    RecorderError::NotRegular(path.to_path_buf())
                } else {
                    RecorderError::Io(error.to_string())
                }
            })?;
        let metadata = file.metadata().map_err(|error| RecorderError::Io(error.to_string()))?;
        if !metadata.file_type().is_file() {
            return Err(RecorderError::NotRegular(path.to_path_buf()));
        }
        Ok(Box::new(FileHandle { file }))
    }

    fn titles(&self) -> Box<dyn TitleSource> {
        Box::new(super::titles::TitleCopy::refresh(&self.recordings_dir, &self.state_dir, self.read_titles))
    }

    fn subdirectory_counts(&self) -> Vec<(String, Option<u64>)> {
        APPLE_SUBDIRECTORIES
            .iter()
            .map(|name| {
                let count = std::fs::read_dir(self.recordings_dir.join(name)).ok().map(|entries| entries.count() as u64);
                ((*name).to_owned(), count)
            })
            .collect()
    }
}

pub struct FileHandle {
    file: File,
}

impl BoxReader for FileHandle {
    fn len(&self) -> u64 {
        self.file.metadata().map(|m| m.len()).unwrap_or(0)
    }

    fn read_exact_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), ReadFailure> {
        let mut done = 0usize;
        while done < buf.len() {
            let position = i64::try_from(offset + done as u64).map_err(|_| ReadFailure)?;
            // SAFETY: `buf` outlives the call and its length bounds the read; the
            // descriptor is open for the lifetime of `self.file`.
            let read = unsafe {
                libc::pread(self.file.as_raw_fd(), buf[done..].as_mut_ptr().cast(), buf.len() - done, position)
            };
            if read <= 0 {
                return Err(ReadFailure);
            }
            done += read as usize;
        }
        Ok(())
    }
}

impl SourceHandle for FileHandle {
    fn metadata(&self) -> Result<SourceMetadata, RecorderError> {
        let metadata = self.file.metadata().map_err(|error| RecorderError::Io(error.to_string()))?;
        Ok(SourceMetadata {
            device: metadata.st_dev() as u64,
            inode: metadata.st_ino(),
            size: metadata.len(),
            mtime: FileTime { secs: metadata.st_mtime(), nanos: metadata.st_mtime_nsec() as u32 },
        })
    }

    fn clone_into(&self, destination: &Path) -> Result<CloneKind, CloneError> {
        let target = std::ffi::CString::new(destination.as_os_str().as_encoded_bytes())
            .map_err(|_| CloneError::Io("destination holds a NUL byte".into()))?;
        // SAFETY: `target` is a NUL-terminated path that outlives the call; the
        // source descriptor is open; AT_FDCWD resolves an absolute destination.
        let outcome = unsafe { libc::fclonefileat(self.file.as_raw_fd(), libc::AT_FDCWD, target.as_ptr(), 0) };
        if outcome == 0 {
            return Ok(CloneKind::CopyOnWrite);
        }
        let error = std::io::Error::last_os_error();
        match error.raw_os_error() {
            Some(libc::EXDEV) | Some(libc::ENOTSUP) => self.byte_copy(destination),
            Some(libc::ENOSPC) => Err(CloneError::NoSpace),
            _ => Err(CloneError::Io(error.to_string())),
        }
    }
}

impl FileHandle {
    fn byte_copy(&self, destination: &Path) -> Result<CloneKind, CloneError> {
        use std::io::Write;
        let mut out = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(destination)
            .map_err(|error| CloneError::Io(error.to_string()))?;
        let total = self.len();
        let mut offset = 0u64;
        let mut buffer = vec![0u8; 64 * 1024];
        let mut reader = FileHandle { file: self.file.try_clone().map_err(|error| CloneError::Io(error.to_string()))? };
        while offset < total {
            let chunk = usize::try_from((total - offset).min(buffer.len() as u64)).unwrap_or(buffer.len());
            reader.read_exact_at(offset, &mut buffer[..chunk]).map_err(|_| CloneError::Io("read failed".into()))?;
            out.write_all(&buffer[..chunk]).map_err(|error| {
                if error.raw_os_error() == Some(libc::ENOSPC) { CloneError::NoSpace } else { CloneError::Io(error.to_string()) }
            })?;
            offset += chunk as u64;
        }
        Ok(CloneKind::ByteCopy)
    }
}
```

`crates/vpt-application/src/ports/mod.rs` adds `pub mod recorder;`; `crates/vpt-adapters/src/lib.rs` adds
`pub mod voice_memos;`. Until Task 16 lands, make `titles()` return a source that always answers
`Unavailable` by pointing the module at a two-line `titles.rs` holding `pub struct TitleCopy;` with
`refresh` returning it and `title` returning `TitleLookup::Unavailable`; Task 16 replaces that file.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p vpt-adapters voice_memos`

Expected: 6 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(source): the Voice Memos store listed at depth one and read by descriptor"
```

______________________________________________________________________

### Task 16: The private title copy

**Files:**

- Create: `crates/vpt-adapters/src/voice_memos/titles.rs` (replacing the two-line stand-in)

**Interfaces:**

- Consumes: `TitleSource`, `TitleLookup`.

- Produces: `vpt_adapters::voice_memos::titles::TitleCopy::refresh(recordings_dir: &Path,`
  `state_dir: &Path, enabled: bool) -> TitleCopy` (implements `TitleSource`);
  `pub const COPY_DIRECTORY: &str = "title-copy"`.

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use vpt_application::ports::recorder::TitleLookup;

    fn apple_store(rows: &[(&str, &str)], with_columns: bool) -> tempfile::TempDir {
        let temp = tempfile::tempdir().expect("temp");
        std::fs::create_dir_all(temp.path().join("Recordings")).expect("recordings");
        let database = rusqlite::Connection::open(temp.path().join("CloudRecordings.db")).expect("db");
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
        temp
    }

    #[test]
    fn the_title_is_read_from_the_copy_by_file_name() {
        let temp = apple_store(&[("20260824 144736-4F3AB19C.m4a", "Invoice call")], true);
        let titles = TitleCopy::refresh(&temp.path().join("Recordings"), &temp.path().join("state"), true);
        assert_eq!(titles.title("20260824 144736-4F3AB19C.m4a"), TitleLookup::Titled("Invoice call".into()));
        assert_eq!(titles.title("other.m4a"), TitleLookup::Unavailable);
    }

    #[test]
    fn the_copy_lives_under_the_state_directory_with_restrictive_modes() {
        let temp = apple_store(&[("a.m4a", "A")], true);
        let state = temp.path().join("state");
        TitleCopy::refresh(&temp.path().join("Recordings"), &state, true);
        let copy_dir = state.join(COPY_DIRECTORY);
        assert_eq!(std::fs::metadata(&copy_dir).expect("dir").permissions().mode() & 0o777, 0o700);
        for name in ["CloudRecordings.db", "CloudRecordings.db-wal", "CloudRecordings.db-shm"] {
            let path = copy_dir.join(name);
            assert!(path.exists(), "{name} missing");
            assert_eq!(std::fs::metadata(&path).expect("file").permissions().mode() & 0o777, 0o600);
        }
    }

    #[test]
    fn a_changed_schema_yields_unavailable() {
        let temp = apple_store(&[], false);
        let titles = TitleCopy::refresh(&temp.path().join("Recordings"), &temp.path().join("state"), true);
        assert_eq!(titles.title("a.m4a"), TitleLookup::Unavailable);
    }

    #[test]
    fn disabled_reading_copies_nothing_and_answers_unavailable() {
        let temp = apple_store(&[("a.m4a", "A")], true);
        let state = temp.path().join("state");
        let titles = TitleCopy::refresh(&temp.path().join("Recordings"), &state, false);
        assert_eq!(titles.title("a.m4a"), TitleLookup::Unavailable);
        assert!(!state.join(COPY_DIRECTORY).exists());
    }

    #[test]
    fn a_missing_live_database_yields_unavailable() {
        let temp = tempfile::tempdir().expect("temp");
        std::fs::create_dir_all(temp.path().join("Recordings")).expect("recordings");
        let titles = TitleCopy::refresh(&temp.path().join("Recordings"), &temp.path().join("state"), true);
        assert_eq!(titles.title("a.m4a"), TitleLookup::Unavailable);
    }

    #[test]
    fn the_live_database_directory_is_left_with_no_new_entries() {
        let temp = apple_store(&[("a.m4a", "A")], true);
        let before: Vec<_> = std::fs::read_dir(temp.path()).expect("dir").map(|e| e.expect("entry").file_name()).collect();
        TitleCopy::refresh(&temp.path().join("Recordings"), &temp.path().join("state"), true);
        let after: Vec<_> = std::fs::read_dir(temp.path()).expect("dir").map(|e| e.expect("entry").file_name()).collect();
        assert_eq!(before, after);
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-adapters titles`

Expected: the stand-in answers `Unavailable` everywhere, so
`the_title_is_read_from_the_copy_by_file_name` and the modes test FAIL; the others pass on the stand-in
and stay as regressions guards.

- [ ] **Step 3: Write the minimal implementation**

```rust
//! The database is confined to one thing, the human title, and it is read
//! from a private copy so SQLite never creates a journal inside Apple's
//! directory. The copy is overwritten in place and never removed.

use rusqlite::{Connection, OpenFlags, OptionalExtension};
use std::fs::DirBuilder;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::Path;
use vpt_application::ports::recorder::{TitleLookup, TitleSource};

pub const COPY_DIRECTORY: &str = "title-copy";
const DATABASE: &str = "CloudRecordings.db";
const SIDE_FILES: [&str; 3] = ["CloudRecordings.db", "CloudRecordings.db-wal", "CloudRecordings.db-shm"];

pub struct TitleCopy {
    connection: Option<Connection>,
    table: Option<String>,
}

impl TitleCopy {
    pub fn refresh(recordings_dir: &Path, state_dir: &Path, enabled: bool) -> TitleCopy {
        if !enabled {
            return TitleCopy { connection: None, table: None };
        }
        let live_dir = match recordings_dir.parent() {
            Some(parent) => parent,
            None => return TitleCopy { connection: None, table: None },
        };
        if !live_dir.join(DATABASE).is_file() {
            return TitleCopy { connection: None, table: None };
        }
        let copy_dir = state_dir.join(COPY_DIRECTORY);
        if DirBuilder::new().recursive(true).mode(0o700).create(&copy_dir).is_err() {
            return TitleCopy { connection: None, table: None };
        }
        for name in SIDE_FILES {
            let source = live_dir.join(name);
            let target = copy_dir.join(name);
            let copied = if source.is_file() {
                std::fs::copy(&source, &target).map(|_| ())
            } else {
                std::fs::write(&target, b"")
            };
            if copied.is_err() || std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o600)).is_err() {
                return TitleCopy { connection: None, table: None };
            }
        }
        let connection = Connection::open_with_flags(copy_dir.join(DATABASE), OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX).ok();
        let table = connection.as_ref().and_then(table_with_title_columns);
        TitleCopy { connection, table }
    }
}

/// The table holding both `ZPATH` and `ZCUSTOMLABEL`, found by inspection so a
/// renamed table degrades to untitled rather than to a wrong guess.
fn table_with_title_columns(connection: &Connection) -> Option<String> {
    let mut statement = connection.prepare("SELECT name FROM sqlite_master WHERE type = 'table'").ok()?;
    let names: Vec<String> = statement.query_map([], |row| row.get(0)).ok()?.filter_map(Result::ok).collect();
    names.into_iter().find(|name| {
        let mut columns = match connection.prepare(&format!("PRAGMA table_info(\"{}\")", name.replace('"', "\"\""))) {
            Ok(statement) => statement,
            Err(_) => return false,
        };
        let found: Vec<String> = columns.query_map([], |row| row.get::<_, String>(1)).ok().map(|rows| rows.filter_map(Result::ok).collect()).unwrap_or_default();
        found.iter().any(|c| c == "ZPATH") && found.iter().any(|c| c == "ZCUSTOMLABEL")
    })
}

impl TitleSource for TitleCopy {
    fn title(&self, file_name: &str) -> TitleLookup {
        let (Some(connection), Some(table)) = (&self.connection, &self.table) else {
            return TitleLookup::Unavailable;
        };
        let query = format!(
            "SELECT ZCUSTOMLABEL FROM \"{}\" WHERE ZPATH = ?1 OR ZPATH LIKE ?2 LIMIT 1",
            table.replace('"', "\"\"")
        );
        let suffix = format!("%/{}", file_name.replace('%', "\\%").replace('_', "\\_"));
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
```

The `LIKE` escape needs `ESCAPE '\'` in the query; append `ESCAPE '\\'` after `?2` in the query string.
Add `pub mod titles;` to `voice_memos/mod.rs`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p vpt-adapters voice_memos`

Expected: all PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/vpt-adapters
SKIP_AI_COMMIT=1 git commit -m "feat(source): titles from a private copy of the Voice Memos database"
```

______________________________________________________________________

### Task 17: The archive: staging clones and digests

**Files:**

- Create: `crates/vpt-application/src/ports/archive.rs`
- Modify: `crates/vpt-application/src/ports/mod.rs`
- Create: `crates/vpt-adapters/src/archive/mod.rs`
- Modify: `crates/vpt-adapters/src/lib.rs`

**Interfaces:**

- Consumes: `SourceHandle`, `CloneKind`, `CloneError`, `BoxReader`, `stores::{digest_file, BUFFER}`.

- Produces, in `vpt_application::ports::archive`:
  `Staged { pub path: PathBuf, pub digest: Sha256Digest, pub size: u64, pub copy_on_write: bool }`;
  `Published::{Placed(PathBuf), Exists(PathBuf)}`; `ArchiveError::{NoSpace, Sync(String), Io(String)}`;
  `trait Archive { fn stage(&self, source: &dyn SourceHandle) -> Result<Staged,`
  `ArchiveError>; fn open(&self, path: &Path) -> Result<Box<dyn BoxReader>, ArchiveError>;`
  `fn digest(&self, path: &Path) -> Result<Sha256Digest, ArchiveError>; fn publish(&self,`
  `staged: &Path, target_name: &str) -> Result<Published, ArchiveError>;`
  `fn sync_existing(&self, path: &Path) -> Result<(), ArchiveError>; fn target(&self,`
  `name: &str) -> PathBuf; fn archived(&self) -> Result<Vec<PathBuf>, ArchiveError>;`
  `fn staged_leftovers(&self) -> Result<Vec<PathBuf>, ArchiveError>; }`;
  `vpt_adapters::archive::ClonefileArchive::new(audio_store: PathBuf) -> ClonefileArchive`;
  `pub const STAGING_PREFIX: &str = ".vpt-staging-"`.

- [ ] **Step 1: Write the failing tests**

`crates/vpt-adapters/src/archive/mod.rs`, test section:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::voice_memos::store::VoiceMemosStore;
    use std::os::unix::fs::PermissionsExt;
    use vpt_application::ports::recorder::RecorderStore;
    use vpt_domain::container::inspect;
    use vpt_domain::fixtures::m4a;

    fn source(bytes: &[u8]) -> (tempfile::TempDir, Box<dyn vpt_application::ports::recorder::SourceHandle>) {
        let temp = tempfile::tempdir().expect("temp");
        std::fs::create_dir_all(temp.path().join("Recordings")).expect("recordings");
        std::fs::write(temp.path().join("Recordings/a.m4a"), bytes).expect("fixture");
        let store = VoiceMemosStore::new(temp.path().join("Recordings"), temp.path().join("state"), false);
        let handle = store.open(&temp.path().join("Recordings/a.m4a")).expect("open");
        (temp, handle)
    }

    #[test]
    fn staging_produces_a_0600_private_copy_with_the_bytes_digest_and_size() {
        let bytes = m4a(1_787_690_856, 5, b"payload");
        let (temp, handle) = source(&bytes);
        let audio = temp.path().join("audio");
        std::fs::create_dir_all(&audio).expect("audio");
        let archive = ClonefileArchive::new(audio.clone());

        let staged = archive.stage(&*handle).expect("staged");

        assert!(staged.path.starts_with(&audio));
        assert!(staged.path.file_name().expect("name").to_string_lossy().starts_with(STAGING_PREFIX));
        assert_eq!(std::fs::read(&staged.path).expect("read"), bytes);
        assert_eq!(std::fs::metadata(&staged.path).expect("meta").permissions().mode() & 0o777, 0o600);
        assert_eq!(staged.size, bytes.len() as u64);
        assert_eq!(staged.digest, crate::stores::digest_file(&staged.path).expect("digest"));
        assert!(staged.copy_on_write);
    }

    #[test]
    fn a_staged_file_can_be_opened_for_the_wholeness_gate() {
        let (temp, handle) = source(&m4a(1_787_690_856, 5, b"payload"));
        let archive = ClonefileArchive::new(temp.path().join("audio"));
        std::fs::create_dir_all(temp.path().join("audio")).expect("audio");
        let staged = archive.stage(&*handle).expect("staged");
        let container = inspect(&mut *archive.open(&staged.path).expect("open")).expect("whole");
        assert_eq!(container.duration_secs, 5);
    }

    #[test]
    fn archived_lists_only_m4a_names_and_leftovers_only_staging_names() {
        let temp = tempfile::tempdir().expect("temp");
        let audio = temp.path().join("audio");
        std::fs::create_dir_all(&audio).expect("audio");
        std::fs::write(audio.join("2026-08-24T144736-4f3ab19c02de.m4a"), b"x").expect("archived");
        std::fs::write(audio.join(format!("{STAGING_PREFIX}123.m4a")), b"y").expect("leftover");
        std::fs::write(audio.join("notes.txt"), b"z").expect("other");
        let archive = ClonefileArchive::new(audio.clone());
        assert_eq!(archive.archived().expect("list"), vec![audio.join("2026-08-24T144736-4f3ab19c02de.m4a")]);
        assert_eq!(archive.staged_leftovers().expect("list"), vec![audio.join(format!("{STAGING_PREFIX}123.m4a"))]);
        assert_eq!(archive.target("abc.m4a"), audio.join("abc.m4a"));
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-adapters archive`

Expected: compile error, `ClonefileArchive` not found.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-application/src/ports/archive.rs`:

```rust
//! The audio archive: staging from a descriptor, exclusive publication, digests.

use crate::ports::recorder::SourceHandle;
use std::path::{Path, PathBuf};
use vpt_domain::container::BoxReader;
use vpt_domain::digest::Sha256Digest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Staged {
    pub path: PathBuf,
    pub digest: Sha256Digest,
    pub size: u64,
    pub copy_on_write: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Published {
    Placed(PathBuf),
    Exists(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchiveError {
    NoSpace,
    Sync(String),
    Io(String),
}

pub trait Archive {
    fn stage(&self, source: &dyn SourceHandle) -> Result<Staged, ArchiveError>;
    fn open(&self, path: &Path) -> Result<Box<dyn BoxReader>, ArchiveError>;
    fn digest(&self, path: &Path) -> Result<Sha256Digest, ArchiveError>;
    /// Sync the staged file, move it to `target_name` without replacing an
    /// existing target, sync the directory.
    fn publish(&self, staged: &Path, target_name: &str) -> Result<Published, ArchiveError>;
    /// Sync an archive file and its directory (duplicate recovery).
    fn sync_existing(&self, path: &Path) -> Result<(), ArchiveError>;
    fn target(&self, name: &str) -> PathBuf;
    fn archived(&self) -> Result<Vec<PathBuf>, ArchiveError>;
    fn staged_leftovers(&self) -> Result<Vec<PathBuf>, ArchiveError>;
}
```

`crates/vpt-adapters/src/archive/mod.rs`:

```rust
//! The archive on disk: copy-on-write staging from the source descriptor,
//! digests in 64 KiB buffers, the exclusive publish of `publish.rs`.

pub mod publish;

use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use vpt_application::ports::archive::*;
use vpt_application::ports::recorder::{CloneError, SourceHandle};
use vpt_domain::container::{BoxReader, ReadFailure};
use vpt_domain::digest::Sha256Digest;

pub const STAGING_PREFIX: &str = ".vpt-staging-";

static STAGING_COUNTER: AtomicU64 = AtomicU64::new(0);

pub struct ClonefileArchive {
    audio_store: PathBuf,
}

impl ClonefileArchive {
    pub fn new(audio_store: PathBuf) -> ClonefileArchive {
        ClonefileArchive { audio_store }
    }

    fn staging_name(&self) -> PathBuf {
        let counter = STAGING_COUNTER.fetch_add(1, Ordering::Relaxed);
        self.audio_store.join(format!("{STAGING_PREFIX}{}-{counter}.m4a", std::process::id()))
    }

    fn list(&self, keep: impl Fn(&str) -> bool) -> Result<Vec<PathBuf>, ArchiveError> {
        let entries = std::fs::read_dir(&self.audio_store).map_err(io)?;
        let mut paths = Vec::new();
        for entry in entries {
            let entry = entry.map_err(io)?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if keep(&name) && entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                paths.push(entry.path());
            }
        }
        paths.sort();
        Ok(paths)
    }
}

pub(crate) fn io(error: std::io::Error) -> ArchiveError {
    if error.raw_os_error() == Some(libc::ENOSPC) { ArchiveError::NoSpace } else { ArchiveError::Io(error.to_string()) }
}

struct ArchivedFile {
    file: File,
}

impl BoxReader for ArchivedFile {
    fn len(&self) -> u64 {
        self.file.metadata().map(|m| m.len()).unwrap_or(0)
    }

    fn read_exact_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), ReadFailure> {
        use std::os::unix::fs::FileExt;
        self.file.read_exact_at(buf, offset).map_err(|_| ReadFailure)
    }
}

impl Archive for ClonefileArchive {
    fn stage(&self, source: &dyn SourceHandle) -> Result<Staged, ArchiveError> {
        let path = self.staging_name();
        let kind = source.clone_into(&path).map_err(|error| match error {
            CloneError::NoSpace => ArchiveError::NoSpace,
            CloneError::Io(detail) => ArchiveError::Io(detail),
        })?;
        std::fs::set_permissions(&path, std::os::unix::fs::PermissionsExt::from_mode(0o600)).map_err(io)?;
        let digest = crate::stores::digest_file(&path).map_err(io)?;
        let size = std::fs::metadata(&path).map_err(io)?.len();
        Ok(Staged { path, digest, size, copy_on_write: kind == vpt_application::ports::recorder::CloneKind::CopyOnWrite })
    }

    fn open(&self, path: &Path) -> Result<Box<dyn BoxReader>, ArchiveError> {
        Ok(Box::new(ArchivedFile { file: File::open(path).map_err(io)? }))
    }

    fn digest(&self, path: &Path) -> Result<Sha256Digest, ArchiveError> {
        crate::stores::digest_file(path).map_err(io)
    }

    fn publish(&self, staged: &Path, target_name: &str) -> Result<Published, ArchiveError> {
        publish::exclusive(staged, &self.target(target_name))
    }

    fn sync_existing(&self, path: &Path) -> Result<(), ArchiveError> {
        publish::sync_file_and_directory(path)
    }

    fn target(&self, name: &str) -> PathBuf {
        self.audio_store.join(name)
    }

    fn archived(&self) -> Result<Vec<PathBuf>, ArchiveError> {
        self.list(|name| name.ends_with(".m4a") && !name.starts_with('.'))
    }

    fn staged_leftovers(&self) -> Result<Vec<PathBuf>, ArchiveError> {
        self.list(|name| name.starts_with(STAGING_PREFIX))
    }
}
```

`publish.rs` arrives in the next task; for this task's tests to compile, create it with the two
signatures and bodies that return `Err(ArchiveError::Io("not built yet".into()))`, then replace them in
Task 18. Add `pub mod archive;` to `lib.rs` and `pub mod archive;` to `ports/mod.rs`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p vpt-adapters archive`

Expected: 3 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(archive): copy-on-write staging with a private name and a digest"
```

______________________________________________________________________

### Task 18: Exclusive publication, verified on the platform first

The spec leaves the primitive to the plan and asks that the plan verify it before choosing it. The red
test of Step 1 is that verification: it exercises `renamex_np` with `RENAME_EXCL` on the test volume and
proves that an existing target is refused with `EEXIST` and that an absent one is placed. If that probe
fails on the executor's volume (a non-APFS filesystem), the fallback that the same tests must then pass
is `link(2)` of the staged name onto the target, which is exclusive on every POSIX filesystem, followed
by `unlink` of the staged name; record the choice in the commit message.

**Files:**

- Create: `crates/vpt-adapters/src/archive/publish.rs` (replacing the stand-in)

**Interfaces:**

- Consumes: `ArchiveError`, `Published`.

- Produces: `vpt_adapters::archive::publish::{exclusive(staged: &Path, target: &Path) ->`
  `Result<Published, ArchiveError>, sync_file_and_directory(path: &Path) -> Result<(),` `ArchiveError>}`.

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_platform_primitive_refuses_an_existing_target_with_eexist() {
        let temp = tempfile::tempdir().expect("temp");
        let from = temp.path().join("from");
        let to = temp.path().join("to");
        std::fs::write(&from, b"new").expect("from");
        std::fs::write(&to, b"old").expect("to");
        let from_c = std::ffi::CString::new(from.as_os_str().as_encoded_bytes()).expect("path");
        let to_c = std::ffi::CString::new(to.as_os_str().as_encoded_bytes()).expect("path");
        // SAFETY: both strings are NUL-terminated and outlive the call.
        let outcome = unsafe { libc::renamex_np(from_c.as_ptr(), to_c.as_ptr(), libc::RENAME_EXCL) };
        assert_eq!(outcome, -1);
        assert_eq!(std::io::Error::last_os_error().raw_os_error(), Some(libc::EEXIST));
        assert_eq!(std::fs::read(&to).expect("untouched"), b"old");
    }

    #[test]
    fn publish_places_an_absent_target_and_removes_the_staged_name() {
        let temp = tempfile::tempdir().expect("temp");
        let staged = temp.path().join(".vpt-staging-1.m4a");
        std::fs::write(&staged, b"bytes").expect("staged");
        let target = temp.path().join("id.m4a");
        assert_eq!(exclusive(&staged, &target).expect("published"), Published::Placed(target.clone()));
        assert_eq!(std::fs::read(&target).expect("read"), b"bytes");
        assert!(!staged.exists());
    }

    #[test]
    fn publish_never_replaces_an_existing_target_and_names_it() {
        let temp = tempfile::tempdir().expect("temp");
        let staged = temp.path().join(".vpt-staging-2.m4a");
        std::fs::write(&staged, b"new").expect("staged");
        let target = temp.path().join("id.m4a");
        std::fs::write(&target, b"old").expect("existing");
        assert_eq!(exclusive(&staged, &target).expect("refused softly"), Published::Exists(target.clone()));
        assert_eq!(std::fs::read(&target).expect("read"), b"old");
        assert!(staged.exists(), "the staged duplicate stays for the caller to trash");
    }

    #[test]
    fn syncing_a_missing_file_is_an_error_and_an_existing_one_succeeds() {
        let temp = tempfile::tempdir().expect("temp");
        assert!(matches!(sync_file_and_directory(&temp.path().join("missing")), Err(ArchiveError::Sync(_))));
        let present = temp.path().join("present");
        std::fs::write(&present, b"x").expect("present");
        assert_eq!(sync_file_and_directory(&present), Ok(()));
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-adapters publish`

Expected: the probe PASSES on APFS (it is the verification and asserts on the platform, not on vpt);
`publish_places_an_absent_target_and_removes_the_staged_name`,
`publish_never_replaces_an_existing_target_and_names_it` and the sync test FAIL on the stand-in's
`Io("not built yet")`.

- [ ] **Step 3: Write the minimal implementation**

```rust
//! Publication never replaces an existing target. On this platform that is one
//! rename with the exclusive flag, verified by the probe test below; the file
//! is synced before and the directory after, so a committed row always has
//! its archive on disk.

use std::fs::File;
use std::path::Path;
use vpt_application::ports::archive::{ArchiveError, Published};

pub fn exclusive(staged: &Path, target: &Path) -> Result<Published, ArchiveError> {
    File::open(staged).and_then(|file| file.sync_all()).map_err(|error| ArchiveError::Sync(error.to_string()))?;
    let from = c_path(staged)?;
    let to = c_path(target)?;
    // SAFETY: both strings are NUL-terminated and outlive the call; RENAME_EXCL
    // makes the kernel refuse an existing destination atomically.
    let outcome = unsafe { libc::renamex_np(from.as_ptr(), to.as_ptr(), libc::RENAME_EXCL) };
    if outcome != 0 {
        let error = std::io::Error::last_os_error();
        return match error.raw_os_error() {
            Some(libc::EEXIST) => Ok(Published::Exists(target.to_path_buf())),
            Some(libc::ENOSPC) => Err(ArchiveError::NoSpace),
            _ => Err(ArchiveError::Io(error.to_string())),
        };
    }
    sync_directory_of(target)?;
    Ok(Published::Placed(target.to_path_buf()))
}

pub fn sync_file_and_directory(path: &Path) -> Result<(), ArchiveError> {
    File::open(path).and_then(|file| file.sync_all()).map_err(|error| ArchiveError::Sync(error.to_string()))?;
    sync_directory_of(path)
}

fn sync_directory_of(path: &Path) -> Result<(), ArchiveError> {
    let directory = path.parent().ok_or_else(|| ArchiveError::Sync("no parent directory".into()))?;
    File::open(directory).and_then(|dir| dir.sync_all()).map_err(|error| ArchiveError::Sync(error.to_string()))
}

fn c_path(path: &Path) -> Result<std::ffi::CString, ArchiveError> {
    std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).map_err(|_| ArchiveError::Io("path holds a NUL byte".into()))
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p vpt-adapters archive`

Expected: all PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/vpt-adapters
SKIP_AI_COMMIT=1 git commit -m "feat(archive): exclusive publication with the file and directory synced"
```

______________________________________________________________________

### Task 19: The ingest use case, happy path

The sweep runs over real adapters in these tests (a fixture directory, the clone archive and the SQLite
ledger in a temporary directory) with three small fakes for the clock, the Trash and the notifier, so the
tests exercise the code that ships. They live as integration tests of the adapters crate.

**Files:**

- Create: `crates/vpt-domain/src/notification.rs`
- Modify: `crates/vpt-domain/src/lib.rs`
- Create: `crates/vpt-application/src/ports/clock.rs`, `crates/vpt-application/src/ports/trash.rs`,
  `crates/vpt-application/src/ports/notifier.rs`, `crates/vpt-application/src/ingest/mod.rs`,
  `crates/vpt-application/src/ingest/candidate.rs`, `crates/vpt-application/src/ingest/publish.rs`
- Modify: `crates/vpt-application/src/ports/mod.rs`, `crates/vpt-application/src/lib.rs`
- Test: `crates/vpt-adapters/tests/support/mod.rs`, `crates/vpt-adapters/tests/ingest_sweep.rs`

**Interfaces:**

- Consumes: every port so far, `RecordingLedger` (with `seen_all` and `commit_recovered`), the domain
  gates and `inspect`.

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
  - `vpt_application::ports::clock::Clock { fn now(&self) -> UtcInstant; fn offset_at(&self,`
    `at: UtcInstant) -> UtcOffset; }`;
    `ports::trash::{TrashError::{HelperAbsent, Failed(String), Unknown(String)},`
    `Trash { fn trash(&self, path: &Path) -> Result<PathBuf, TrashError>; }}`;
    `ports::notifier::{DeliveryOutcome::{Delivered, Suppressed, Failed(String)},`
    `Notifier { fn deliver(&self, notification: &Notification) -> DeliveryOutcome; }}`.
  - `vpt_application::ingest::{Ingest<'a> { pub recorder: &'a dyn RecorderStore,`
    `pub archive: &'a dyn Archive, pub ledger: &'a dyn RecordingLedger,`
    `pub clock: &'a dyn Clock, pub trash: &'a dyn Trash, pub notifier: &'a dyn Notifier,`
    `pub settings: &'a SourceSettings }, IngestReport { pub ingested: Vec<RecordingRecord>,`
    `pub already_ingested: Vec<RecordingId>, pub recovered: Vec<RecordingId>,`
    `pub deferred: Vec<Deferred>, pub skipped: u64, pub would_ingest: Vec<WouldIngest>,`
    `pub log: Vec<String> }, Deferred { pub path: PathBuf, pub reason: DeferralReason },`
    `WouldIngest { pub path: PathBuf, pub title: Option<String>,`
    `pub title_source: TitleOrigin }, IngestFailure,`
    `IngestError { pub failure: IngestFailure, pub completed: Vec<RecordingId> }}` and
    `Ingest::run(&self) -> Result<IngestReport, IngestError>` (Task 23 changes the signature to take a
    `Mode`).
  - Test support: `support::Fixture::new() -> Fixture` with fields `temp`, `recordings`, `audio`, `state`
    and methods `add_recording(&self, name: &str, bytes: &[u8]) -> PathBuf` (mtime set 60 s in the past),
    `store(&self) -> VoiceMemosStore`, `archive(&self) -> ClonefileArchive`,
    `ledger(&self) -> SqliteLedger`, `settings(&self) -> SourceSettings`,
    `entries(&self) -> Vec<(String, u64, i64, u32)>` (name, size, mtime, flags of every entry in the
    recordings directory); `support::FixedClock { pub now: UtcInstant, pub offset: UtcOffset }`;
    `support::RecordingTrash::new(dir: PathBuf) -> RecordingTrash` with
    `pub moved: RefCell<Vec<PathBuf>>` and `pub absent: Cell<bool>`;
    `support::RecordingNotifier(pub RefCell<Vec<Notification>>)`; `support::CAPTURED: i64`.

- [ ] **Step 1: Write the failing tests**

`crates/vpt-adapters/tests/support/mod.rs`:

```rust
//! Real adapters over a temporary directory, plus the three fakes the sweep
//! needs for the clock, the Trash and the notifier.

#![allow(dead_code)]

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use vpt_adapters::archive::ClonefileArchive;
use vpt_adapters::ledger::sqlite::SqliteLedger;
use vpt_adapters::voice_memos::store::VoiceMemosStore;
use vpt_application::ports::clock::Clock;
use vpt_application::ports::notifier::{DeliveryOutcome, Notifier};
use vpt_application::ports::trash::{Trash, TrashError};
use vpt_application::settings::SourceSettings;
use vpt_domain::notification::Notification;
use vpt_domain::time::{UtcInstant, UtcOffset};

pub const CAPTURED: i64 = 1_787_690_856;

pub struct Fixture {
    pub temp: tempfile::TempDir,
    pub recordings: PathBuf,
    pub audio: PathBuf,
    pub state: PathBuf,
}

impl Fixture {
    pub fn new() -> Fixture {
        let temp = tempfile::tempdir().expect("temp");
        let recordings = temp.path().join("Recordings");
        let audio = temp.path().join("home/audio");
        let state = temp.path().join("state");
        for dir in [&recordings, &audio, &state] {
            std::fs::create_dir_all(dir).expect("fixture dir");
        }
        Fixture { temp, recordings, audio, state }
    }

    pub fn add_recording(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.recordings.join(name);
        std::fs::write(&path, bytes).expect("recording");
        let file = std::fs::File::options().write(true).open(&path).expect("open for times");
        file.set_times(std::fs::FileTimes::new().set_modified(SystemTime::now() - Duration::from_secs(60))).expect("mtime");
        path
    }

    pub fn store(&self) -> VoiceMemosStore {
        VoiceMemosStore::new(self.recordings.clone(), self.state.clone(), false)
    }

    pub fn archive(&self) -> ClonefileArchive {
        ClonefileArchive::new(self.audio.clone())
    }

    pub fn ledger(&self) -> SqliteLedger {
        SqliteLedger::open(&self.state).expect("ledger")
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

    pub fn entries(&self) -> Vec<(String, u64, i64, u32)> {
        use std::os::macos::fs::MetadataExt;
        let mut entries: Vec<_> = std::fs::read_dir(&self.recordings)
            .expect("dir")
            .map(|entry| {
                let entry = entry.expect("entry");
                let metadata = entry.metadata().expect("metadata");
                (entry.file_name().to_string_lossy().into_owned(), metadata.len(), metadata.st_mtime(), metadata.st_flags())
            })
            .collect();
        entries.sort();
        entries
    }
}

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
        std::fs::rename(path, &target).map_err(|error| TrashError::Failed(error.to_string()))?;
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
use support::{CAPTURED, Fixture, RecordingNotifier, RecordingTrash, clock};
use vpt_application::ingest::Ingest;
use vpt_application::ports::ledger::{RecordingLedger, StageStates, TitleOrigin};
use vpt_domain::fixtures::m4a;

#[test]
fn a_whole_recording_is_archived_under_its_identity_and_recorded_in_the_ledger() {
    let fixture = Fixture::new();
    let source = fixture.add_recording("20260824 144736-4F3AB19C.m4a", &m4a(CAPTURED, 612, b"audio bytes"));
    let (store, archive, ledger) = (fixture.store(), fixture.archive(), fixture.ledger());
    let (trash, notifier, settings) = (RecordingTrash::new(fixture.temp.path().join("trash")), RecordingNotifier::default(), fixture.settings());
    let ingest = Ingest { recorder: &store, archive: &archive, ledger: &ledger, clock: &clock(), trash: &trash, notifier: &notifier, settings: &settings };

    let report = ingest.run().expect("sweep");

    assert_eq!(report.ingested.len(), 1);
    let record = &report.ingested[0];
    assert!(record.id.as_str().starts_with("2026-08-24T144736-"), "{}", record.id);
    assert_eq!(record.source_path.as_deref(), Some(source.as_path()));
    assert_eq!(record.duration_secs, 612);
    assert_eq!(record.title, None);
    assert_eq!(record.title_source, TitleOrigin::Unavailable);
    assert_eq!(record.stages, StageStates::fresh());
    assert_eq!(record.audio_path, fixture.audio.join(format!("{}.m4a", record.id)));
    assert_eq!(std::fs::read(&record.audio_path).expect("archive"), m4a(CAPTURED, 612, b"audio bytes"));
    assert_eq!(std::fs::metadata(&record.audio_path).expect("meta").permissions().mode() & 0o777, 0o600);
    assert_eq!(ledger.by_id(&record.id).expect("read"), Some(record.clone()));
    assert_eq!(ledger.seen(&source).expect("seen").and_then(|row| row.recording), Some(record.id.clone()));
    assert!(archive.staged_leftovers().expect("leftovers").is_empty());
    assert!(notifier.0.borrow().is_empty());
}

#[test]
fn a_full_sweep_leaves_every_source_entry_with_its_size_mtime_and_flags() {
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"a"));
    fixture.add_recording("b.m4a", &m4a(CAPTURED + 60, 2, b"bb"));
    std::fs::write(fixture.recordings.join("a.waveform"), b"w").expect("sidecar");
    let before = fixture.entries();
    let (store, archive, ledger) = (fixture.store(), fixture.archive(), fixture.ledger());
    let (trash, notifier, settings) = (RecordingTrash::new(fixture.temp.path().join("trash")), RecordingNotifier::default(), fixture.settings());
    let ingest = Ingest { recorder: &store, archive: &archive, ledger: &ledger, clock: &clock(), trash: &trash, notifier: &notifier, settings: &settings };

    let report = ingest.run().expect("sweep");

    assert_eq!(report.ingested.len(), 2);
    assert_eq!(fixture.entries(), before);
}

#[test]
fn a_second_sweep_skips_the_unchanged_entries() {
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"a"));
    let (store, archive, ledger) = (fixture.store(), fixture.archive(), fixture.ledger());
    let (trash, notifier, settings) = (RecordingTrash::new(fixture.temp.path().join("trash")), RecordingNotifier::default(), fixture.settings());
    let ingest = Ingest { recorder: &store, archive: &archive, ledger: &ledger, clock: &clock(), trash: &trash, notifier: &notifier, settings: &settings };
    ingest.run().expect("first");

    let report = ingest.run().expect("second");

    assert!(report.ingested.is_empty());
    assert_eq!(report.skipped, 1);
    assert_eq!(ledger.recordings().expect("list").len(), 1);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-adapters --test ingest_sweep`

Expected: compile error, `vpt_application::ingest` not found.

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
    Failed(String),
    Unknown(String),
}

pub trait Trash {
    fn trash(&self, path: &Path) -> Result<PathBuf, TrashError>;
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
}
```

`crates/vpt-application/src/ports/mod.rs` lists `archive`, `artifacts`, `clock`, `ledger`, `notifier`,
`prompt`, `recorder`, `trash`.

`crates/vpt-application/src/ingest/mod.rs`:

```rust
//! `vpt ingest`: the sweep of spec section 5, composed from the ports.

mod candidate;
mod publish;

use crate::ports::archive::Archive;
use crate::ports::clock::Clock;
use crate::ports::ledger::{LedgerError, RecordingLedger, RecordingRecord, TitleOrigin};
use crate::ports::notifier::Notifier;
use crate::ports::recorder::RecorderStore;
use crate::ports::trash::Trash;
use crate::settings::SourceSettings;
use std::path::PathBuf;
use vpt_domain::digest::Sha256Digest;
use vpt_domain::identity::RecordingId;
use vpt_domain::notification::{EventKind, Notification};
use vpt_domain::sweep::DeferralReason;

pub struct Ingest<'a> {
    pub recorder: &'a dyn RecorderStore,
    pub archive: &'a dyn Archive,
    pub ledger: &'a dyn RecordingLedger,
    pub clock: &'a dyn Clock,
    pub trash: &'a dyn Trash,
    pub notifier: &'a dyn Notifier,
    pub settings: &'a SourceSettings,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngestFailure {
    StoreUnreadable(String),
    EmptyStore,
    NoSpace,
    Sync(String),
    ArchiveCollision { target: PathBuf, staged: Sha256Digest, existing: Sha256Digest },
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
}

impl Ingest<'_> {
    pub fn run(&self) -> Result<IngestReport, IngestError> {
        let mut report = IngestReport::default();
        match self.sweep(&mut report) {
            Ok(()) => Ok(report),
            Err(failure) => {
                let at = self.clock.now();
                self.notifier.deliver(&Notification::failed(EventKind::IngestFailed, failure.message(), at));
                Err(IngestError { failure, completed: report.ingested.iter().map(|record| record.id.clone()).collect() })
            }
        }
    }

    fn sweep(&self, report: &mut IngestReport) -> Result<(), IngestFailure> {
        let candidates = self.recorder.candidates().map_err(|error| IngestFailure::StoreUnreadable(format!("{error:?}")))?;
        let titles = if self.settings.read_titles { Some(self.recorder.titles()) } else { None };
        for candidate in &candidates {
            self.process(candidate, titles.as_deref(), report)?;
        }
        Ok(())
    }
}
```

`crates/vpt-application/src/ingest/candidate.rs`:

```rust
//! One candidate: the pre-open decision, the descriptor, then staging.

use super::{Ingest, IngestFailure, IngestReport};
use crate::ports::ledger::{LedgerError, SeenRow};
use crate::ports::recorder::{Candidate, TitleSource};
use vpt_domain::sweep::{CandidateFacts, PreOpen, SeenFacts, pre_open};
use vpt_domain::time::UtcInstant;

impl Ingest<'_> {
    pub(super) fn process(
        &self,
        candidate: &Candidate,
        titles: Option<&dyn TitleSource>,
        report: &mut IngestReport,
    ) -> Result<(), IngestFailure> {
        let now = self.clock.now();
        let seen = self.ledger.seen(&candidate.path).map_err(IngestFailure::Ledger)?;
        let facts = CandidateFacts { size: candidate.size, mtime: candidate.mtime, dataless: candidate.dataless };
        let seen_facts = seen.as_ref().map(|row| SeenFacts {
            size: row.size,
            mtime: row.mtime,
            ingested: row.recording.is_some(),
            deferred_size: row.deferred_size,
        });
        if pre_open(&facts, seen_facts.as_ref()) == PreOpen::Unchanged {
            report.skipped += 1;
            return Ok(());
        }
        let mut handle = self.recorder.open(&candidate.path).map_err(|error| IngestFailure::Recorder(format!("{error:?}")))?;
        let metadata = handle.metadata().map_err(|error| IngestFailure::Recorder(format!("{error:?}")))?;
        self.stage_and_publish(candidate, &mut *handle, &metadata, seen, titles, now, report)
    }
}

pub(super) fn fresh_seen(candidate: &Candidate, now: UtcInstant) -> SeenRow {
    SeenRow {
        path: candidate.path.clone(),
        file_name: candidate.file_name.clone(),
        size: candidate.size,
        mtime: candidate.mtime,
        dataless: candidate.dataless,
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
//! Stage, verify, hash, publish, commit.

use super::candidate::{fresh_seen, ledger_failure};
use super::{Ingest, IngestFailure, IngestReport};
use crate::ports::archive::{ArchiveError, Published};
use crate::ports::ledger::{RecordingRecord, SeenRow, StageStates, TitleOrigin};
use crate::ports::recorder::{Candidate, SourceHandle, SourceMetadata, TitleLookup, TitleSource};
use std::path::Path;
use vpt_domain::container::inspect;
use vpt_domain::identity::RecordingId;
use vpt_domain::time::UtcInstant;

impl Ingest<'_> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn stage_and_publish(
        &self,
        candidate: &Candidate,
        handle: &mut dyn SourceHandle,
        metadata: &SourceMetadata,
        seen: Option<SeenRow>,
        titles: Option<&dyn TitleSource>,
        now: UtcInstant,
        report: &mut IngestReport,
    ) -> Result<(), IngestFailure> {
        let staged = self.archive.stage(&*handle).map_err(archive_failure)?;
        let after = handle.metadata().map_err(|error| IngestFailure::Recorder(format!("{error:?}")))?;
        if after != *metadata {
            self.discard(&staged.path, report);
            return Err(IngestFailure::Recorder("the source changed while it was staged".into()));
        }
        let mut reader = self.archive.open(&staged.path).map_err(archive_failure)?;
        let container = inspect(&mut *reader).map_err(|error| IngestFailure::Archive(format!("staged container invalid: {error:?}")))?;
        drop(reader);
        let offset = self.clock.offset_at(container.creation_time);
        let id = RecordingId::derive(container.creation_time, offset, &staged.digest);
        if !staged.copy_on_write {
            report.log.push(format!("{}: the archive is not copy-on-write", candidate.file_name));
        }
        match self.archive.publish(&staged.path, &format!("{id}.m4a")).map_err(archive_failure)? {
            Published::Placed(audio_path) => {
                let (title, title_source) = lookup_title(titles, &candidate.file_name);
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
                let row = ingested_seen(candidate, seen, &record.id, now);
                self.ledger.commit_ingest(&record, &row).map_err(ledger_failure)?;
                report.ingested.push(record);
                Ok(())
            }
            Published::Exists(audio_path) => {
                self.discard(&staged.path, report);
                Err(IngestFailure::Archive(format!("{} exists", audio_path.display())))
            }
        }
    }

    pub(super) fn discard(&self, staged: &Path, report: &mut IngestReport) {
        if let Err(error) = self.trash.trash(staged) {
            report.log.push(format!("{}: staged file kept ({error:?})", staged.display()));
        }
    }
}

pub(super) fn archive_failure(error: ArchiveError) -> IngestFailure {
    match error {
        ArchiveError::NoSpace => IngestFailure::NoSpace,
        ArchiveError::Sync(detail) => IngestFailure::Sync(detail),
        ArchiveError::Io(detail) => IngestFailure::Archive(detail),
    }
}

pub(super) fn lookup_title(titles: Option<&dyn TitleSource>, file_name: &str) -> (Option<String>, TitleOrigin) {
    match titles.map(|source| source.title(file_name)) {
        Some(TitleLookup::Titled(title)) => (Some(title), TitleOrigin::VoiceMemos),
        _ => (None, TitleOrigin::Unavailable),
    }
}

pub(super) fn ingested_seen(candidate: &Candidate, seen: Option<SeenRow>, id: &RecordingId, now: UtcInstant) -> SeenRow {
    let mut row = seen.unwrap_or_else(|| fresh_seen(candidate, now));
    row.size = candidate.size;
    row.mtime = candidate.mtime;
    row.dataless = candidate.dataless;
    row.last_seen = now;
    row.deferral_count = 0;
    row.deferral_reason = None;
    row.deferred_size = None;
    row.source_gone_at = None;
    row.recording = Some(id.clone());
    row
}
```

`crates/vpt-application/src/lib.rs` adds `pub mod ingest;`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p vpt-adapters --test ingest_sweep`

Expected: 3 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(ingest): archive a whole recording under its identity"
```

______________________________________________________________________

### Task 20: The gates: deferrals, retries and the deferred page

**Files:**

- Modify: `crates/vpt-application/src/ingest/candidate.rs`,
  `crates/vpt-application/src/ingest/publish.rs`
- Test: `crates/vpt-adapters/tests/ingest_gates.rs`

**Interfaces:**

- Consumes: `vpt_domain::sweep::{size_gate, rest_gate, SweepLimits, DeferralReason}`, `inspect`.

- Produces: `Ingest::defer(&self, candidate: &Candidate, seen: Option<SeenRow>,`
  `reason: DeferralReason, now: UtcInstant, report: &mut IngestReport) -> Result<(),` `IngestFailure>`
  (records the deferral, raises the `deferred` event at the threshold, appends to `report.deferred`).

- [ ] **Step 1: Write the failing tests**

```rust
mod support;

use std::cell::RefCell;
use std::path::Path;
use support::{CAPTURED, Fixture, RecordingNotifier, RecordingTrash, clock};
use vpt_application::ingest::Ingest;
use vpt_application::ports::ledger::RecordingLedger;
use vpt_application::ports::recorder::*;
use vpt_domain::fixtures::m4a;
use vpt_domain::notification::EventKind;
use vpt_domain::sweep::DeferralReason;

/// A recorder that reports what a real store would, with one flag forced.
struct DatalessStore<'a> {
    inner: &'a dyn RecorderStore,
    dataless: RefCell<Vec<String>>,
}

impl RecorderStore for DatalessStore<'_> {
    fn candidates(&self) -> Result<Vec<Candidate>, RecorderError> {
        let mut candidates = self.inner.candidates()?;
        for candidate in &mut candidates {
            candidate.dataless = self.dataless.borrow().contains(&candidate.file_name);
        }
        Ok(candidates)
    }
    fn candidate(&self, path: &Path) -> Result<Candidate, RecorderError> {
        self.inner.candidate(path)
    }
    fn open(&self, path: &Path) -> Result<Box<dyn SourceHandle>, RecorderError> {
        if self.dataless.borrow().iter().any(|name| path.ends_with(name)) {
            panic!("a dataless entry was opened");
        }
        self.inner.open(path)
    }
    fn titles(&self) -> Box<dyn TitleSource> {
        self.inner.titles()
    }
    fn subdirectory_counts(&self) -> Vec<(String, Option<u64>)> {
        self.inner.subdirectory_counts()
    }
}

#[test]
fn a_dataless_entry_is_deferred_without_being_opened() {
    let fixture = Fixture::new();
    let source = fixture.add_recording("cloud.m4a", &m4a(CAPTURED, 1, b"x"));
    let real = fixture.store();
    let store = DatalessStore { inner: &real, dataless: RefCell::new(vec!["cloud.m4a".into()]) };
    let (archive, ledger, settings) = (fixture.archive(), fixture.ledger(), fixture.settings());
    let (trash, notifier) = (RecordingTrash::new(fixture.temp.path().join("trash")), RecordingNotifier::default());
    let ingest = Ingest { recorder: &store, archive: &archive, ledger: &ledger, clock: &clock(), trash: &trash, notifier: &notifier, settings: &settings };

    let report = ingest.run().expect("sweep");

    assert_eq!(report.deferred.len(), 1);
    assert_eq!(report.deferred[0].reason, DeferralReason::Dataless);
    assert_eq!(ledger.seen(&source).expect("seen").map(|row| (row.deferral_count, row.deferral_reason)), Some((1, Some(DeferralReason::Dataless))));
    assert!(report.ingested.is_empty());
}

#[test]
fn a_truncated_download_is_deferred_as_an_invalid_container_and_retried_when_whole() {
    let fixture = Fixture::new();
    let whole = m4a(CAPTURED, 1, b"payload");
    let source = fixture.add_recording("a.m4a", &whole[..whole.len() - 12]);
    let (store, archive, ledger, settings) = (fixture.store(), fixture.archive(), fixture.ledger(), fixture.settings());
    let (trash, notifier) = (RecordingTrash::new(fixture.temp.path().join("trash")), RecordingNotifier::default());
    let ingest = Ingest { recorder: &store, archive: &archive, ledger: &ledger, clock: &clock(), trash: &trash, notifier: &notifier, settings: &settings };

    let first = ingest.run().expect("first");
    assert_eq!(first.deferred[0].reason, DeferralReason::InvalidContainer);
    assert!(archive.archived().expect("archived").is_empty(), "nothing is cloned before every gate passes");

    fixture.add_recording("a.m4a", &whole);
    let second = ingest.run().expect("second");
    assert_eq!(second.ingested.len(), 1);
    assert_eq!(ledger.seen(&source).expect("seen").map(|row| row.deferral_count), Some(0));
}

#[test]
fn a_file_younger_than_the_quiet_period_is_not_at_rest() {
    let fixture = Fixture::new();
    let path = fixture.add_recording("fresh.m4a", &m4a(CAPTURED, 1, b"x"));
    let now = std::time::SystemTime::now();
    std::fs::File::options().write(true).open(&path).expect("open").set_times(std::fs::FileTimes::new().set_modified(now)).expect("mtime");
    let (store, archive, ledger, settings) = (fixture.store(), fixture.archive(), fixture.ledger(), fixture.settings());
    let (trash, notifier) = (RecordingTrash::new(fixture.temp.path().join("trash")), RecordingNotifier::default());
    let wall = support::FixedClock {
        now: vpt_domain::time::UtcInstant { secs: now.duration_since(std::time::UNIX_EPOCH).expect("epoch").as_secs() as i64 },
        offset: vpt_domain::time::UtcOffset { secs: 0 },
    };
    let ingest = Ingest { recorder: &store, archive: &archive, ledger: &ledger, clock: &wall, trash: &trash, notifier: &notifier, settings: &settings };

    let report = ingest.run().expect("sweep");

    assert_eq!(report.deferred[0].reason, DeferralReason::NotAtRest);
}

#[test]
fn an_oversized_entry_is_deferred_before_any_content_is_read() {
    let fixture = Fixture::new();
    fixture.add_recording("big.m4a", &m4a(CAPTURED, 1, &[0u8; 4_096]));
    let (store, archive, ledger) = (fixture.store(), fixture.archive(), fixture.ledger());
    let mut settings = fixture.settings();
    settings.max_audio_bytes = 1_000;
    let (trash, notifier) = (RecordingTrash::new(fixture.temp.path().join("trash")), RecordingNotifier::default());
    let ingest = Ingest { recorder: &store, archive: &archive, ledger: &ledger, clock: &clock(), trash: &trash, notifier: &notifier, settings: &settings };

    let report = ingest.run().expect("sweep");

    assert_eq!(report.deferred[0].reason, DeferralReason::AudioTooLarge);
}

#[test]
fn the_deferred_event_is_raised_once_at_the_threshold() {
    let fixture = Fixture::new();
    let whole = m4a(CAPTURED, 1, b"payload");
    fixture.add_recording("a.m4a", &whole[..whole.len() - 12]);
    let (store, archive, ledger) = (fixture.store(), fixture.archive(), fixture.ledger());
    let mut settings = fixture.settings();
    settings.deferral_page_threshold = 2;
    let (trash, notifier) = (RecordingTrash::new(fixture.temp.path().join("trash")), RecordingNotifier::default());
    let ingest = Ingest { recorder: &store, archive: &archive, ledger: &ledger, clock: &clock(), trash: &trash, notifier: &notifier, settings: &settings };

    for _ in 0..3 {
        ingest.run().expect("sweep");
    }

    let events: Vec<_> = notifier.0.borrow().iter().map(|n| n.event).collect();
    assert_eq!(events, vec![EventKind::Deferred]);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-adapters --test ingest_gates`

Expected: the dataless test PANICS ("a dataless entry was opened"); the container test FAILS with
`Archive("staged container invalid ...")` as an error rather than a deferral; the rest and size tests
FAIL because the entries are ingested; the threshold test FAILS with no events.

- [ ] **Step 3: Write the minimal implementation**

Replace `process` in `crates/vpt-application/src/ingest/candidate.rs` and add `defer`:

```rust
use vpt_domain::container::inspect;
use vpt_domain::notification::{EventKind, Notification};
use vpt_domain::sweep::{CandidateFacts, DeferralReason, PreOpen, SeenFacts, SweepLimits, pre_open, rest_gate, size_gate};

impl Ingest<'_> {
    pub(super) fn process(
        &self,
        candidate: &Candidate,
        titles: Option<&dyn TitleSource>,
        report: &mut IngestReport,
    ) -> Result<(), IngestFailure> {
        let now = self.clock.now();
        let seen = self.ledger.seen(&candidate.path).map_err(IngestFailure::Ledger)?;
        let facts = CandidateFacts { size: candidate.size, mtime: candidate.mtime, dataless: candidate.dataless };
        let seen_facts = seen.as_ref().map(|row| SeenFacts {
            size: row.size,
            mtime: row.mtime,
            ingested: row.recording.is_some(),
            deferred_size: row.deferred_size,
        });
        match pre_open(&facts, seen_facts.as_ref()) {
            PreOpen::Dataless => return self.defer(candidate, seen, DeferralReason::Dataless, now, report),
            PreOpen::Unchanged => {
                report.skipped += 1;
                return Ok(());
            }
            PreOpen::Open => {}
        }
        let mut handle = match self.recorder.open(&candidate.path) {
            Ok(handle) => handle,
            Err(RecorderError::NotRegular(_)) => return self.defer(candidate, seen, DeferralReason::InvalidContainer, now, report),
            Err(error) => return Err(IngestFailure::Recorder(format!("{error:?}"))),
        };
        let metadata = handle.metadata().map_err(|error| IngestFailure::Recorder(format!("{error:?}")))?;
        let limits = SweepLimits { max_audio_bytes: self.settings.max_audio_bytes, quiet_period_secs: self.settings.quiet_period_secs };
        if let Err(reason) = size_gate(metadata.size, &limits) {
            return self.defer(candidate, seen, reason, now, report);
        }
        if inspect(&mut *handle).is_err() {
            return self.defer(candidate, seen, DeferralReason::InvalidContainer, now, report);
        }
        if let Err(reason) = rest_gate(metadata.mtime, metadata.size, now, seen_facts.as_ref(), &limits) {
            return self.defer(candidate, seen, reason, now, report);
        }
        self.stage_and_publish(candidate, &mut *handle, &metadata, seen, titles, now, report)
    }

    pub(super) fn defer(
        &self,
        candidate: &Candidate,
        seen: Option<SeenRow>,
        reason: DeferralReason,
        now: UtcInstant,
        report: &mut IngestReport,
    ) -> Result<(), IngestFailure> {
        let mut row = seen.unwrap_or_else(|| fresh_seen(candidate, now));
        row.size = candidate.size;
        row.mtime = candidate.mtime;
        row.dataless = candidate.dataless;
        row.last_seen = now;
        row.deferral_count += 1;
        row.deferral_reason = Some(reason);
        row.deferred_size = Some(candidate.size);
        self.ledger.record_seen(&row).map_err(IngestFailure::Ledger)?;
        if row.deferral_count == self.settings.deferral_page_threshold {
            self.notifier.deliver(&Notification::attention(
                EventKind::Deferred,
                None,
                format!("deferred {} times: {}", row.deferral_count, reason.as_str()),
                vec![("source".into(), candidate.path.clone())],
                now,
            ));
        }
        report.deferred.push(super::Deferred { path: candidate.path.clone(), reason });
        Ok(())
    }
}
```

with `use crate::ports::recorder::RecorderError;` added to the imports. In `publish.rs`, the two staged
gates become deferrals instead of failures. Replace the `after != *metadata` block and the container
line:

```rust
        if after != *metadata {
            self.discard(&staged.path, report);
            return self.defer(candidate, seen, DeferralReason::ChangedDuringRead, now, report);
        }
        let mut reader = self.archive.open(&staged.path).map_err(archive_failure)?;
        let container = match inspect(&mut *reader) {
            Ok(container) => container,
            Err(_) => {
                drop(reader);
                self.discard(&staged.path, report);
                return self.defer(candidate, seen, DeferralReason::InvalidContainer, now, report);
            }
        };
        drop(reader);
```

with `use vpt_domain::sweep::DeferralReason;` imported.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p vpt-adapters --test ingest_gates --test ingest_sweep`

Expected: all 8 PASS.

- [ ] **Step 5: Commit**

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(ingest): the sweep gates defer, retry and page at the threshold"
```

______________________________________________________________________

### Task 21: Duplicates, recovery and the archive collision

**Files:**

- Modify: `crates/vpt-application/src/ingest/publish.rs`
- Test: `crates/vpt-adapters/tests/ingest_duplicates.rs`

**Interfaces:**

- Consumes: `Archive::{digest, sync_existing}`, `RecordingLedger::{by_digest, set_source_path}`.

- Produces: `Ingest::known_bytes` and `Ingest::resolve_existing` (private to the module).

- [ ] **Step 1: Write the failing tests**

```rust
mod support;

use support::{CAPTURED, Fixture, RecordingNotifier, RecordingTrash, clock};
use vpt_application::ingest::{Ingest, IngestFailure};
use vpt_application::ports::archive::Archive;
use vpt_application::ports::ledger::RecordingLedger;
use vpt_domain::fixtures::m4a;
use vpt_domain::notification::EventKind;

fn parts(fixture: &Fixture) -> (vpt_adapters::voice_memos::store::VoiceMemosStore, vpt_adapters::archive::ClonefileArchive, vpt_adapters::ledger::sqlite::SqliteLedger, RecordingTrash, RecordingNotifier, vpt_application::settings::SourceSettings) {
    (
        fixture.store(),
        fixture.archive(),
        fixture.ledger(),
        RecordingTrash::new(fixture.temp.path().join("trash")),
        RecordingNotifier::default(),
        fixture.settings(),
    )
}

#[test]
fn the_same_bytes_at_a_second_path_are_one_recording_whose_source_path_moves() {
    let fixture = Fixture::new();
    let bytes = m4a(CAPTURED, 1, b"same");
    fixture.add_recording("first.m4a", &bytes);
    let (store, archive, ledger, trash, notifier, settings) = parts(&fixture);
    let ingest = Ingest { recorder: &store, archive: &archive, ledger: &ledger, clock: &clock(), trash: &trash, notifier: &notifier, settings: &settings };
    ingest.run().expect("first");
    std::fs::rename(fixture.recordings.join("first.m4a"), fixture.recordings.join("renamed.m4a")).expect("rename");

    let report = ingest.run().expect("second");

    assert_eq!(report.already_ingested.len(), 1);
    assert!(report.ingested.is_empty());
    let record = ledger.recordings().expect("list").remove(0);
    assert_eq!(record.source_path, Some(fixture.recordings.join("renamed.m4a")));
    assert_eq!(archive.archived().expect("archived").len(), 1);
    assert!(archive.staged_leftovers().expect("leftovers").is_empty());
    assert_eq!(trash.moved.borrow().len(), 1, "the staged duplicate went to the Trash");
}

#[test]
fn an_archive_without_a_ledger_row_is_recovered_on_publish_and_reported_as_already_ingested() {
    let fixture = Fixture::new();
    let bytes = m4a(CAPTURED, 1, b"orphan");
    fixture.add_recording("a.m4a", &bytes);
    let (store, archive, ledger, trash, notifier, settings) = parts(&fixture);
    let ingest = Ingest { recorder: &store, archive: &archive, ledger: &ledger, clock: &clock(), trash: &trash, notifier: &notifier, settings: &settings };
    let first = ingest.run().expect("first");
    let id = first.ingested[0].id.clone();
    drop(ledger);
    std::fs::remove_file(fixture.state.join("vpt.db")).expect("lose the ledger in the test only");
    let _ = std::fs::remove_file(fixture.state.join("vpt.db-wal"));
    let _ = std::fs::remove_file(fixture.state.join("vpt.db-shm"));
    let ledger = fixture.ledger();
    let ingest = Ingest { recorder: &store, archive: &archive, ledger: &ledger, clock: &clock(), trash: &trash, notifier: &notifier, settings: &settings };

    let report = ingest.run().expect("second");

    assert!(report.already_ingested.contains(&id) || report.recovered.contains(&id), "{report:?}");
    assert!(ledger.by_id(&id).expect("read").is_some());
    assert_eq!(archive.archived().expect("archived").len(), 1);
}

#[test]
fn a_target_with_different_bytes_is_an_archive_collision_and_nothing_is_replaced() {
    let fixture = Fixture::new();
    let bytes = m4a(CAPTURED, 1, b"real");
    fixture.add_recording("a.m4a", &bytes);
    let (store, archive, ledger, trash, notifier, settings) = parts(&fixture);
    let ingest = Ingest { recorder: &store, archive: &archive, ledger: &ledger, clock: &clock(), trash: &trash, notifier: &notifier, settings: &settings };
    let first = ingest.run().expect("first");
    let record = first.ingested[0].clone();
    std::fs::write(&record.audio_path, b"tampered").expect("tamper the archive in the test only");
    drop(ledger);
    std::fs::remove_file(fixture.state.join("vpt.db")).expect("lose the ledger in the test only");
    let _ = std::fs::remove_file(fixture.state.join("vpt.db-wal"));
    let _ = std::fs::remove_file(fixture.state.join("vpt.db-shm"));
    let ledger = fixture.ledger();
    let ingest = Ingest { recorder: &store, archive: &archive, ledger: &ledger, clock: &clock(), trash: &trash, notifier: &notifier, settings: &settings };

    let error = ingest.run().unwrap_err();

    assert!(matches!(error.failure, IngestFailure::ArchiveCollision { .. }), "{error:?}");
    assert_eq!(std::fs::read(&record.audio_path).expect("read"), b"tampered");
    assert!(archive.staged_leftovers().expect("leftovers").is_empty());
    assert_eq!(notifier.0.borrow().last().map(|n| n.event), Some(EventKind::IngestFailed));
}
```

The recovery test deletes a database file, which the spec forbids vpt itself from doing; the test harness
may, because it is simulating a lost ledger inside a temporary directory the test owns.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-adapters --test ingest_duplicates`

Expected: the rename test FAILS (the sweep reports `Archive("... exists")` because the same bytes derive
the same identity); the recovery test FAILS the same way; the collision test FAILS because the failure is
`Archive`, not `ArchiveCollision`.

- [ ] **Step 3: Write the minimal implementation**

In `crates/vpt-application/src/ingest/publish.rs`, after deriving `id` and before publishing:

```rust
        if let Some(existing) = self.ledger.by_digest(&staged.digest).map_err(ledger_failure)? {
            return self.known_bytes(candidate, existing, &staged, seen, now, report);
        }
```

Replace the `Published::Exists` arm:

```rust
            Published::Exists(audio_path) => {
                self.resolve_existing(candidate, id, container, offset, &staged, audio_path, seen, titles, now, report)
            }
```

and add the two methods to the `impl Ingest<'_>` block:

```rust
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
            self.ledger.set_source_path(&existing.id, &candidate.path).map_err(ledger_failure)?;
            report.log.push(format!("{}: known bytes at a new path, source path updated", existing.id));
        }
        self.ledger.record_seen(&ingested_seen(candidate, seen, &existing.id, now)).map_err(ledger_failure)?;
        self.discard(&staged.path, report);
        report.already_ingested.push(existing.id);
        Ok(())
    }

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
        titles: Option<&dyn TitleSource>,
        now: UtcInstant,
        report: &mut IngestReport,
    ) -> Result<(), IngestFailure> {
        let existing = self.archive.digest(&audio_path).map_err(archive_failure)?;
        let mut reader = self.archive.open(&audio_path).map_err(archive_failure)?;
        let valid = inspect(&mut *reader).is_ok();
        drop(reader);
        if existing != staged.digest || !valid {
            self.discard(&staged.path, report);
            return Err(IngestFailure::ArchiveCollision { target: audio_path, staged: staged.digest, existing });
        }
        self.archive.sync_existing(&audio_path).map_err(archive_failure)?;
        if self.ledger.by_id(&id).map_err(ledger_failure)?.is_none() {
            let (title, title_source) = lookup_title(titles, &candidate.file_name);
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
            self.ledger.commit_ingest(&record, &ingested_seen(candidate, seen, &id, now)).map_err(ledger_failure)?;
            report.recovered.push(id.clone());
        } else {
            self.ledger.record_seen(&ingested_seen(candidate, seen, &id, now)).map_err(ledger_failure)?;
        }
        self.discard(&staged.path, report);
        report.already_ingested.push(id);
        Ok(())
    }
```

with these imports added: `use crate::ports::archive::Staged;`, `use std::path::PathBuf;`,
`use vpt_domain::container::Container;`, `use vpt_domain::time::UtcOffset;`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p vpt-adapters --test ingest_duplicates`

Expected: 3 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(ingest): duplicates recover their rows and a collision refuses"
```

______________________________________________________________________

### Task 22: Vanished sources and orphaned archives

**Files:**

- Modify: `crates/vpt-application/src/ingest/mod.rs`
- Test: `crates/vpt-adapters/tests/ingest_recovery.rs`

**Interfaces:**

- Consumes: `RecordingLedger::{seen_all, record_seen, commit_recovered}`,
  `Archive::{archived, open, digest}`.

- Produces: `Ingest::recover_orphans` and `Ingest::mark_gone` (private).

- [ ] **Step 1: Write the failing tests**

```rust
mod support;

use support::{CAPTURED, Fixture, RecordingNotifier, RecordingTrash, clock};
use vpt_application::ingest::Ingest;
use vpt_application::ports::ledger::RecordingLedger;
use vpt_domain::fixtures::m4a;
use vpt_domain::identity::RecordingId;
use vpt_domain::time::UtcInstant;

#[test]
fn a_recording_whose_source_disappeared_is_marked_gone_and_its_clone_stays() {
    let fixture = Fixture::new();
    let source = fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"a"));
    let (store, archive, ledger, settings) = (fixture.store(), fixture.archive(), fixture.ledger(), fixture.settings());
    let (trash, notifier) = (RecordingTrash::new(fixture.temp.path().join("trash")), RecordingNotifier::default());
    let ingest = Ingest { recorder: &store, archive: &archive, ledger: &ledger, clock: &clock(), trash: &trash, notifier: &notifier, settings: &settings };
    let first = ingest.run().expect("first");
    std::fs::remove_file(&source).expect("the operator deleted the memo; test setup only");

    ingest.run().expect("second");

    let row = ledger.seen(&source).expect("seen").expect("row");
    assert_eq!(row.source_gone_at, Some(UtcInstant { secs: CAPTURED + 3_600 }));
    assert!(first.ingested[0].audio_path.exists());
    assert!(notifier.0.borrow().is_empty());
}

#[test]
fn an_archive_file_with_no_ledger_row_is_recovered_at_the_start_of_the_sweep_even_without_its_source() {
    let fixture = Fixture::new();
    let bytes = m4a(CAPTURED, 7, b"orphan");
    let (store, archive, settings) = (fixture.store(), fixture.archive(), fixture.settings());
    let digest = {
        let temp = fixture.temp.path().join("scratch.m4a");
        std::fs::write(&temp, &bytes).expect("scratch");
        vpt_adapters::stores::digest_file(&temp).expect("digest")
    };
    let id = RecordingId::derive(UtcInstant { secs: CAPTURED }, clock().offset, &digest);
    std::fs::write(fixture.audio.join(format!("{id}.m4a")), &bytes).expect("orphan archive");
    let ledger = fixture.ledger();
    let (trash, notifier) = (RecordingTrash::new(fixture.temp.path().join("trash")), RecordingNotifier::default());
    let ingest = Ingest { recorder: &store, archive: &archive, ledger: &ledger, clock: &clock(), trash: &trash, notifier: &notifier, settings: &settings };

    let report = ingest.run().expect("sweep");

    assert_eq!(report.recovered, vec![id.clone()]);
    let record = ledger.by_id(&id).expect("read").expect("recovered");
    assert_eq!(record.source_path, None);
    assert_eq!(record.duration_secs, 7);
    assert_eq!(record.digest, digest);
}

#[test]
fn an_archive_file_whose_name_does_not_match_its_bytes_is_logged_and_left_alone() {
    let fixture = Fixture::new();
    std::fs::write(fixture.audio.join("2026-08-24T144736-000000000000.m4a"), m4a(CAPTURED, 1, b"x")).expect("mismatch");
    let (store, archive, ledger, settings) = (fixture.store(), fixture.archive(), fixture.ledger(), fixture.settings());
    let (trash, notifier) = (RecordingTrash::new(fixture.temp.path().join("trash")), RecordingNotifier::default());
    let ingest = Ingest { recorder: &store, archive: &archive, ledger: &ledger, clock: &clock(), trash: &trash, notifier: &notifier, settings: &settings };

    let report = ingest.run().expect("sweep");

    assert!(report.recovered.is_empty());
    assert!(report.log.iter().any(|line| line.contains("does not match")), "{:?}", report.log);
    assert!(ledger.recordings().expect("list").is_empty());
    assert_eq!(archive.archived().expect("archived").len(), 1);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-adapters --test ingest_recovery`

Expected: the gone test FAILS on `source_gone_at == None`; the two orphan tests FAIL on an empty
`recovered` and an empty log.

- [ ] **Step 3: Write the minimal implementation**

In `crates/vpt-application/src/ingest/mod.rs`, `sweep` becomes:

```rust
    fn sweep(&self, report: &mut IngestReport) -> Result<(), IngestFailure> {
        self.recover_orphans(report)?;
        let candidates = self.recorder.candidates().map_err(|error| IngestFailure::StoreUnreadable(format!("{error:?}")))?;
        let titles = if self.settings.read_titles { Some(self.recorder.titles()) } else { None };
        for candidate in &candidates {
            self.process(candidate, titles.as_deref(), report)?;
        }
        self.mark_gone(&candidates, report)
    }

    fn recover_orphans(&self, report: &mut IngestReport) -> Result<(), IngestFailure> {
        for path in self.archive.archived().map_err(|error| IngestFailure::Archive(format!("{error:?}")))? {
            let name = path.file_stem().map(|stem| stem.to_string_lossy().into_owned()).unwrap_or_default();
            let Ok(named) = RecordingId::parse(&name) else {
                report.log.push(format!("{}: not an archive name, left alone", path.display()));
                continue;
            };
            if self.ledger.by_id(&named).map_err(IngestFailure::Ledger)?.is_some() {
                continue;
            }
            let mut reader = self.archive.open(&path).map_err(|error| IngestFailure::Archive(format!("{error:?}")))?;
            let container = match inspect(&mut *reader) {
                Ok(container) => container,
                Err(error) => {
                    report.log.push(format!("{}: container invalid ({error:?}), left alone", path.display()));
                    continue;
                }
            };
            drop(reader);
            let digest = self.archive.digest(&path).map_err(|error| IngestFailure::Archive(format!("{error:?}")))?;
            let offset = self.clock.offset_at(container.creation_time);
            let derived = RecordingId::derive(container.creation_time, offset, &digest);
            if derived != named {
                report.log.push(format!("{}: name does not match its bytes ({derived}), left alone", path.display()));
                continue;
            }
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
            self.ledger.commit_recovered(&record).map_err(IngestFailure::Ledger)?;
            report.log.push(format!("{named}: archive recovered into the ledger"));
            report.recovered.push(named);
        }
        Ok(())
    }

    fn mark_gone(&self, candidates: &[Candidate], report: &mut IngestReport) -> Result<(), IngestFailure> {
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
```

with imports `use crate::ports::ledger::StageStates;`, `use crate::ports::recorder::Candidate;`,
`use vpt_domain::container::inspect;`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p vpt-adapters --test 'ingest_*'`

Expected: all PASS.

- [ ] **Step 5: Commit**

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(ingest): mark vanished sources and recover orphaned archives"
```

______________________________________________________________________

### Task 23: Dry run, `--once`, and the aborts

**Files:**

- Modify: `crates/vpt-application/src/ingest/mod.rs`, `crates/vpt-application/src/ingest/candidate.rs`,
  `crates/vpt-application/src/ingest/publish.rs`
- Test: `crates/vpt-adapters/tests/ingest_modes.rs`

**Interfaces:**

- Produces: `vpt_application::ingest::Mode::{Sweep, DryRun, Once(PathBuf)}`;
  `Ingest::run(&self, mode: Mode) -> Result<IngestReport, IngestError>` (this replaces the no-argument
  signature; update the four earlier test files to pass `Mode::Sweep`).

- [ ] **Step 1: Write the failing tests**

```rust
mod support;

use support::{CAPTURED, Fixture, RecordingNotifier, RecordingTrash, clock};
use vpt_application::ingest::{Ingest, IngestFailure, Mode};
use vpt_application::ports::ledger::{RecordingLedger, TitleOrigin};
use vpt_domain::fixtures::m4a;
use vpt_domain::notification::EventKind;

#[test]
fn a_dry_run_runs_every_gate_and_writes_nothing_durable() {
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"a"));
    let whole = m4a(CAPTURED, 1, b"payload");
    fixture.add_recording("broken.m4a", &whole[..whole.len() - 12]);
    let (store, archive, ledger, settings) = (fixture.store(), fixture.archive(), fixture.ledger(), fixture.settings());
    let (trash, notifier) = (RecordingTrash::new(fixture.temp.path().join("trash")), RecordingNotifier::default());
    let ingest = Ingest { recorder: &store, archive: &archive, ledger: &ledger, clock: &clock(), trash: &trash, notifier: &notifier, settings: &settings };

    let report = ingest.run(Mode::DryRun).expect("dry run");

    assert_eq!(report.would_ingest.len(), 1);
    assert_eq!(report.would_ingest[0].title_source, TitleOrigin::Unavailable);
    assert_eq!(report.deferred.len(), 1);
    assert!(archive.archived().expect("archived").is_empty());
    assert!(ledger.seen_all().expect("seen").is_empty(), "a dry run records no deferral");
    assert!(notifier.0.borrow().is_empty());
    assert!(!fixture.state.join("title-copy").exists());
}

#[test]
fn once_ingests_exactly_the_named_file_with_the_gates() {
    let fixture = Fixture::new();
    let a = fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"a"));
    fixture.add_recording("b.m4a", &m4a(CAPTURED + 1, 1, b"b"));
    let (store, archive, ledger, settings) = (fixture.store(), fixture.archive(), fixture.ledger(), fixture.settings());
    let (trash, notifier) = (RecordingTrash::new(fixture.temp.path().join("trash")), RecordingNotifier::default());
    let ingest = Ingest { recorder: &store, archive: &archive, ledger: &ledger, clock: &clock(), trash: &trash, notifier: &notifier, settings: &settings };

    let report = ingest.run(Mode::Once(a)).expect("once");

    assert_eq!(report.ingested.len(), 1);
    assert_eq!(ledger.recordings().expect("list").len(), 1);
}

#[test]
fn an_unreadable_recordings_directory_aborts_with_one_event() {
    let fixture = Fixture::new();
    std::fs::remove_dir_all(&fixture.recordings).expect("test setup only");
    let (store, archive, ledger, settings) = (fixture.store(), fixture.archive(), fixture.ledger(), fixture.settings());
    let (trash, notifier) = (RecordingTrash::new(fixture.temp.path().join("trash")), RecordingNotifier::default());
    let ingest = Ingest { recorder: &store, archive: &archive, ledger: &ledger, clock: &clock(), trash: &trash, notifier: &notifier, settings: &settings };

    let error = ingest.run(Mode::Sweep).unwrap_err();

    assert!(matches!(error.failure, IngestFailure::StoreUnreadable(_)));
    assert_eq!(notifier.0.borrow().iter().map(|n| n.event).collect::<Vec<_>>(), vec![EventKind::IngestFailed]);
}

#[test]
fn an_empty_store_that_held_recordings_before_is_the_silent_failure_and_aborts() {
    let fixture = Fixture::new();
    let a = fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"a"));
    let (store, archive, ledger, settings) = (fixture.store(), fixture.archive(), fixture.ledger(), fixture.settings());
    let (trash, notifier) = (RecordingTrash::new(fixture.temp.path().join("trash")), RecordingNotifier::default());
    let ingest = Ingest { recorder: &store, archive: &archive, ledger: &ledger, clock: &clock(), trash: &trash, notifier: &notifier, settings: &settings };
    ingest.run(Mode::Sweep).expect("first");
    std::fs::remove_file(&a).expect("test setup only");

    let error = ingest.run(Mode::Sweep).unwrap_err();

    assert_eq!(error.failure, IngestFailure::EmptyStore);
    assert!(error.completed.is_empty());
}

#[test]
fn a_fresh_empty_store_is_not_a_failure() {
    let fixture = Fixture::new();
    let (store, archive, ledger, settings) = (fixture.store(), fixture.archive(), fixture.ledger(), fixture.settings());
    let (trash, notifier) = (RecordingTrash::new(fixture.temp.path().join("trash")), RecordingNotifier::default());
    let ingest = Ingest { recorder: &store, archive: &archive, ledger: &ledger, clock: &clock(), trash: &trash, notifier: &notifier, settings: &settings };
    assert!(ingest.run(Mode::Sweep).is_ok());
}

#[test]
fn a_failure_after_an_ingestion_lists_the_completed_identities() {
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"a"));
    let (store, archive, ledger, settings) = (fixture.store(), fixture.archive(), fixture.ledger(), fixture.settings());
    let (trash, notifier) = (RecordingTrash::new(fixture.temp.path().join("trash")), RecordingNotifier::default());
    let ingest = Ingest { recorder: &store, archive: &archive, ledger: &ledger, clock: &clock(), trash: &trash, notifier: &notifier, settings: &settings };
    let first = ingest.run(Mode::Sweep).expect("first");
    let bytes = m4a(CAPTURED + 5, 1, b"second");
    fixture.add_recording("b.m4a", &bytes);
    let digest = vpt_adapters::stores::digest_file(&fixture.recordings.join("b.m4a")).expect("digest");
    let id = vpt_domain::identity::RecordingId::derive(vpt_domain::time::UtcInstant { secs: CAPTURED + 5 }, clock().offset, &digest);
    std::fs::write(fixture.audio.join(format!("{id}.m4a")), b"tampered").expect("collision setup");
    std::fs::remove_file(fixture.recordings.join("a.m4a")).expect("test setup only");
    fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"a"));

    let error = ingest.run(Mode::Sweep).unwrap_err();

    assert!(matches!(error.failure, IngestFailure::ArchiveCollision { .. }), "{error:?}");
    assert!(error.completed.is_empty() || error.completed == vec![first.ingested[0].id.clone()]);
}
```

The last test's `completed` assertion admits both orderings because the sweep visits candidates by name
and `a.m4a` is already ingested; its point is that `completed` is populated from the run, not from the
ledger. Tighten it when the executor confirms the visit order: with names sorted, `a.m4a` is skipped as
unchanged and `completed` is empty.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-adapters --test ingest_modes`

Expected: compile error, `Mode` not found.

- [ ] **Step 3: Write the minimal implementation**

In `ingest/mod.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Sweep,
    DryRun,
    Once(PathBuf),
}
```

`run` takes `mode: Mode` and passes `&mode` to `sweep`; `sweep` becomes:

```rust
    fn sweep(&self, mode: &Mode, report: &mut IngestReport) -> Result<(), IngestFailure> {
        if *mode != Mode::DryRun {
            self.recover_orphans(report)?;
        }
        let candidates = match mode {
            Mode::Once(path) => vec![self.recorder.candidate(path).map_err(|error| IngestFailure::Recorder(format!("{error:?}")))?],
            _ => self.recorder.candidates().map_err(|error| IngestFailure::StoreUnreadable(format!("{error:?}")))?,
        };
        if candidates.is_empty() && *mode == Mode::Sweep && !self.ledger.seen_all().map_err(IngestFailure::Ledger)?.is_empty() {
            return Err(IngestFailure::EmptyStore);
        }
        let titles = if *mode != Mode::DryRun && self.settings.read_titles { Some(self.recorder.titles()) } else { None };
        for candidate in &candidates {
            self.process(candidate, titles.as_deref(), mode, report)?;
        }
        if *mode == Mode::Sweep {
            self.mark_gone(&candidates, report)?;
        }
        Ok(())
    }
```

`process` gains `mode: &Mode` and, after the rest gate and before `stage_and_publish`:

```rust
        if *mode == Mode::DryRun {
            let known = seen
                .as_ref()
                .and_then(|row| row.recording.clone())
                .and_then(|id| self.ledger.by_id(&id).ok().flatten());
            report.would_ingest.push(super::WouldIngest {
                path: candidate.path.clone(),
                title: known.as_ref().and_then(|record| record.title.clone()),
                title_source: known.map_or(TitleOrigin::Unavailable, |record| record.title_source),
            });
            return Ok(());
        }
```

and `defer` gains `mode: &Mode` too: under `DryRun` it appends to `report.deferred` and returns without
`record_seen` and without the event. Every call site passes `mode` through (`process` calls `defer` five
times; `stage_and_publish` calls it twice and therefore gains `mode: &Mode` as well). In `mod.rs` the
`use crate::ports::ledger::TitleOrigin;` import serves `candidate.rs` through `super::`.

Update the four earlier test files to call `ingest.run(Mode::Sweep)`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p vpt-adapters`

Expected: all PASS, including every earlier ingest test.

- [ ] **Step 5: Commit**

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(ingest): dry run, --once and the abort conditions"
```

______________________________________________________________________

### Task 24: Bounded process execution and the fake engine

Every command vpt spawns runs from its argv directly, in its own process group, with stdin, stdout and
stderr handled concurrently under one deadline; at the deadline or on interruption the group is
terminated, force-killed after one second and reaped. The fake engine is the child every later test
spawns.

**Files:**

- Create: `crates/vpt-adapters/src/spawn.rs`
- Modify: `crates/vpt-adapters/src/lib.rs`
- Create: `crates/vpt/src/bin/vpt-fake-engine.rs` (replacing the stand-in)

**Interfaces:**

- Consumes: nothing.

- Produces: `vpt_adapters::spawn::{run(argv: &[String], stdin: &[u8], deadline: Duration,`
  `output_limit: usize) -> Result<Outcome, SpawnError>,`
  `run_with_env(the same plus env: &[(&str, &str)]), Outcome { pub status: Status,`
  `pub stdout: Vec<u8>, pub stderr: Vec<u8>, pub group: libc::pid_t },`
  `Status::{Exited(i32), Signaled(i32), DeadlineExceeded, Interrupted},`
  `SpawnError::{NotFound(String), Io(String)}, install_interrupt_handlers(),`
  `note_interrupt(), interrupted() -> bool, OUTPUT_LIMIT: usize = 65_536,` `GRACE: Duration = 1 s}`.

- The fake engine `vpt-fake-engine`: `--version` prints
  `{"schema":"vpt.helper/1","version":"<VPT_FAKE_VERSION or 1.0.0>"}`; `notify --title <t> --body <b>`
  appends `notify\t<t>\t<b>\n` to `$VPT_FAKE_LOG` when set and prints `{"posted":true}`; `trash <path>`
  renames the path into `$VPT_FAKE_TRASH` and prints `{"trashed":"<path>"}`; `command-sink [args...]`
  reads stdin to the end, appends `command-sink\t<args joined by space>\t<stdin>\n` to `$VPT_FAKE_LOG`
  and exits with `$VPT_FAKE_EXIT` (default 0); with `VPT_FAKE_HANG=1` every subcommand sleeps for 60
  seconds before answering.

- [ ] **Step 1: Write the failing tests**

`crates/vpt-adapters/src/spawn.rs`, test section (children are `/bin/sh` and `/bin/cat`, which every
macOS runner has; vpt itself never spawns a shell):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn argv(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| (*w).to_owned()).collect()
    }

    fn clear_interrupt_for_tests() {
        INTERRUPTED.store(false, Ordering::SeqCst);
    }

    #[test]
    fn the_exit_code_and_both_streams_are_returned() {
        let outcome = run(&argv(&["/bin/sh", "-c", "echo out; echo err 1>&2; exit 3"]), b"", Duration::from_secs(5), OUTPUT_LIMIT).expect("ran");
        assert_eq!(outcome.status, Status::Exited(3));
        assert_eq!(outcome.stdout, b"out\n");
        assert_eq!(outcome.stderr, b"err\n");
    }

    #[test]
    fn stdin_is_written_and_a_missing_executable_is_not_found() {
        let outcome = run(&argv(&["/bin/cat"]), b"hello", Duration::from_secs(5), OUTPUT_LIMIT).expect("ran");
        assert_eq!(outcome.stdout, b"hello");
        assert!(matches!(run(&argv(&["/nonexistent/vpt-missing"]), b"", Duration::from_secs(1), OUTPUT_LIMIT), Err(SpawnError::NotFound(_))));
    }

    #[test]
    fn output_is_bounded_while_the_child_is_still_drained_to_completion() {
        let outcome = run(&argv(&["/bin/sh", "-c", "yes | head -c 200000"]), b"", Duration::from_secs(5), 1_000).expect("ran");
        assert_eq!(outcome.stdout.len(), 1_000);
        assert_eq!(outcome.status, Status::Exited(0));
    }

    #[test]
    fn the_deadline_terminates_the_whole_process_group_and_reaps_it() {
        let started = Instant::now();
        let outcome = run(&argv(&["/bin/sh", "-c", "sleep 30 & sleep 30"]), b"", Duration::from_millis(100), OUTPUT_LIMIT).expect("ran");
        assert_eq!(outcome.status, Status::DeadlineExceeded);
        assert!(started.elapsed() < Duration::from_millis(900), "{:?}", started.elapsed());
        // SAFETY: signal 0 probes for the group's existence and delivers nothing.
        let probe = unsafe { libc::killpg(outcome.group, 0) };
        assert_eq!(probe, -1, "the process group still exists");
    }

    #[test]
    fn an_interrupt_terminates_the_child_and_reports_interrupted() {
        note_interrupt();
        let outcome = run(&argv(&["/bin/sh", "-c", "sleep 30"]), b"", Duration::from_secs(5), OUTPUT_LIMIT).expect("ran");
        clear_interrupt_for_tests();
        assert_eq!(outcome.status, Status::Interrupted);
    }
}
```

`Outcome` therefore also carries `pub group: libc::pid_t`, the process group id, so a test can prove the
group is gone. `clear_interrupt_for_tests` lives inside the test module and nowhere else.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-adapters spawn`

Expected: compile error, `run` and `Status` not found.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-adapters/src/spawn.rs`:

```rust
//! Bounded process execution: argv, own process group, one deadline for
//! writing, draining and waiting, group termination, reaping.

use std::io::{Read, Write};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{Command, Stdio};
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

extern "C" fn on_signal(_signal: libc::c_int) {
    INTERRUPTED.store(true, Ordering::SeqCst);
}

/// Called once at startup by the command crate.
pub fn install_interrupt_handlers() {
    for signal in [libc::SIGINT, libc::SIGTERM, libc::SIGHUP] {
        // SAFETY: the handler only stores into an atomic, which is async-signal-safe.
        unsafe {
            libc::signal(signal, on_signal as extern "C" fn(libc::c_int) as libc::sighandler_t);
        }
    }
}

pub fn note_interrupt() {
    INTERRUPTED.store(true, Ordering::SeqCst);
}

pub fn interrupted() -> bool {
    INTERRUPTED.load(Ordering::SeqCst)
}

pub fn run(argv: &[String], stdin: &[u8], deadline: Duration, output_limit: usize) -> Result<Outcome, SpawnError> {
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
    let (program, arguments) = argv.split_first().ok_or_else(|| SpawnError::Io("empty argv".into()))?;
    let mut child = Command::new(program)
        .args(arguments)
        .envs(env.iter().copied())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => SpawnError::NotFound(program.clone()),
            _ => SpawnError::Io(error.to_string()),
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
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|error| SpawnError::Io(error.to_string()))? {
            break match (status.code(), status.signal()) {
                (Some(code), _) => Status::Exited(code),
                (None, Some(signal)) => Status::Signaled(signal),
                (None, None) => Status::Exited(-1),
            };
        }
        if interrupted() {
            terminate(group, &mut child);
            break Status::Interrupted;
        }
        if started.elapsed() >= deadline {
            terminate(group, &mut child);
            break Status::DeadlineExceeded;
        }
        std::thread::sleep(TICK);
    };
    let _ = writer.join();
    let stdout = out.join().unwrap_or_default();
    let stderr = err.join().unwrap_or_default();
    Ok(Outcome { status, stdout, stderr, group })
}

fn drain(pipe: Option<impl Read>, limit: usize) -> Vec<u8> {
    let Some(mut pipe) = pipe else { return Vec::new() };
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

/// TERM to the group, one second of grace, KILL, then reap the child.
fn terminate(group: libc::pid_t, child: &mut std::process::Child) {
    // SAFETY: `group` is the child's own process group, created by process_group(0).
    unsafe {
        libc::killpg(group, libc::SIGTERM);
    }
    let until = Instant::now() + GRACE;
    while Instant::now() < until {
        if matches!(child.try_wait(), Ok(Some(_))) {
            break;
        }
        std::thread::sleep(TICK);
    }
    // SAFETY: as above; a group that already exited makes killpg fail harmlessly.
    unsafe {
        libc::killpg(group, libc::SIGKILL);
    }
    let _ = child.wait();
}
```

The grandchild `sleep 30 &` in the deadline test exits with the group kill, which is what the
`killpg(group, 0)` probe proves. Add `pub mod spawn;` to `lib.rs`.

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
    if let Ok(path) = std::env::var("VPT_FAKE_LOG") {
        if let Ok(mut file) = std::fs::OpenOptions::new().append(true).create(true).open(path) {
            let _ = writeln!(file, "{line}");
        }
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

Run: `cargo test -p vpt-adapters spawn && cargo build -p vpt --features dev-tools`

Expected: 5 tests PASS; the fake engine builds.

- [ ] **Step 5: Commit**

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(adapters): bounded process execution and the fake engine"
```

______________________________________________________________________

### Task 25: The helper client and the bounded document reader

**Files:**

- Create: `crates/vpt-protocol/src/limits.rs`, `crates/vpt-protocol/src/helper.rs`
- Modify: `crates/vpt-protocol/src/lib.rs`
- Create: `crates/vpt-adapters/src/helper.rs`
- Modify: `crates/vpt-adapters/src/lib.rs`, `crates/vpt/src/lib.rs`, `crates/vpt/src/commands/version.rs`
- Test: `crates/vpt/tests/helper_client.rs`, and `crates/vpt/tests/version.rs` gains one test

**Interfaces:**

- Consumes: `spawn::{run, Status, SpawnError}`, `Trash`, `TrashError`.

- Produces:

  - `vpt_protocol::limits::{Limits { pub bytes: usize, pub depth: usize,`
    `pub text_chars: usize, pub array_len: usize }, Limits::incoming() -> Limits,`
    `LimitViolation::{Bytes(usize), Depth(usize), Text(usize), Array(usize),`
    `Syntax(String)}, read_bounded(bytes: &[u8], limits: &Limits) ->`
    `Result<serde_json::Value, LimitViolation>}`.
  - `vpt_protocol::helper::{HELPER_SCHEMA: &str = "vpt.helper/1",`
    `HelperVersion { pub schema: String, pub version: String },`
    `HelperVersion::major(&self) -> Option<u32>, Posted { pub posted: bool },`
    `Trashed { pub trashed: String }}` (serde `Deserialize`).
  - `vpt_adapters::helper::{HelperClient::new(path: PathBuf) -> HelperClient,`
    `CALL_DEADLINE: Duration = 5 s, BUILT_AGAINST_MAJOR: u32 = 1, HelperError::{Absent,`
    `MajorMismatch { found: u32 }, Failed(String), Unknown(String)}}` with
    `fn version(&self) -> Result<String, HelperError>`,
    `fn notify(&self, title: &str, body: &str) -> Result<(), HelperError>`, the `version_with_env`,
    `notify_with_env` and `trash_with_env` variants taking `env: &[(&str, &str)]`, and
    `impl Trash for HelperClient`.
  - Test support: `Sandbox::install_fake_helper(&self)` links `bin/vpt-macos` to the fake engine and
    creates `trash/` in the sandbox; `Sandbox::fake_log(&self) -> PathBuf`;
    `Sandbox::fake_engine() -> &'static str`.

- [ ] **Step 1: Write the failing tests**

`crates/vpt-protocol/src/limits.rs`, test section:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_document_within_every_limit_parses() {
        let value = read_bounded(br#"{"schema":"vpt.helper/1","version":"1.0.0"}"#, &Limits::incoming()).expect("parses");
        assert_eq!(value["version"], "1.0.0");
    }

    #[test]
    fn one_byte_over_the_byte_limit_is_refused_before_parsing() {
        let limits = Limits { bytes: 10, ..Limits::incoming() };
        assert_eq!(read_bounded(b"{\"a\":\"bcd\"}", &limits), Err(LimitViolation::Bytes(11)));
    }

    #[test]
    fn nesting_past_the_depth_limit_is_refused_without_allocating_the_tree() {
        let deep = format!("{}1{}", "[".repeat(9), "]".repeat(9));
        assert_eq!(read_bounded(deep.as_bytes(), &Limits::incoming()), Err(LimitViolation::Depth(9)));
        let ok = format!("{}1{}", "[".repeat(8), "]".repeat(8));
        assert!(read_bounded(ok.as_bytes(), &Limits::incoming()).is_ok());
    }

    #[test]
    fn brackets_inside_strings_do_not_count_as_nesting() {
        assert!(read_bounded(br#"{"text":"[[[[[[[[[[[["}"#, &Limits::incoming()).is_ok());
    }

    #[test]
    fn a_text_value_past_the_character_limit_is_refused() {
        let limits = Limits { text_chars: 3, ..Limits::incoming() };
        assert_eq!(read_bounded(br#"{"t":"abcd"}"#, &limits), Err(LimitViolation::Text(4)));
    }

    #[test]
    fn an_array_past_the_entry_limit_is_refused() {
        let limits = Limits { array_len: 2, ..Limits::incoming() };
        assert_eq!(read_bounded(b"[1,2,3]", &limits), Err(LimitViolation::Array(3)));
    }

    #[test]
    fn malformed_json_is_a_syntax_violation() {
        assert!(matches!(read_bounded(b"{", &Limits::incoming()), Err(LimitViolation::Syntax(_))));
    }
}
```

`crates/vpt/tests/helper_client.rs`:

```rust
mod support;

use std::path::PathBuf;
use std::time::Instant;
use support::Sandbox;
use vpt_adapters::helper::{BUILT_AGAINST_MAJOR, HelperClient, HelperError};
use vpt_application::ports::trash::{Trash, TrashError};

#[test]
fn the_helper_version_is_read_from_the_fake() {
    let sandbox = Sandbox::new("helper-version");
    sandbox.install_fake_helper();
    let client = HelperClient::new(sandbox.path().join("bin/vpt-macos"));
    assert_eq!(client.version().expect("version"), "1.0.0");
    assert_eq!(BUILT_AGAINST_MAJOR, 1);
}

#[test]
fn a_helper_of_another_major_version_is_refused_naming_it() {
    let sandbox = Sandbox::new("helper-major");
    sandbox.install_fake_helper();
    let client = HelperClient::new(sandbox.path().join("bin/vpt-macos"));
    let outcome = client.version_with_env(&[("VPT_FAKE_VERSION", "2.3.0")]);
    assert_eq!(outcome, Err(HelperError::MajorMismatch { found: 2 }));
}

#[test]
fn an_absent_helper_is_absent_not_a_failure() {
    let client = HelperClient::new(PathBuf::from("/nonexistent/vpt-macos"));
    assert_eq!(client.version(), Err(HelperError::Absent));
    assert_eq!(client.trash(std::path::Path::new("/tmp/x")), Err(TrashError::HelperAbsent));
}

#[test]
fn trash_moves_the_file_through_the_helper_and_returns_the_reply_path() {
    let sandbox = Sandbox::new("helper-trash");
    sandbox.install_fake_helper();
    let victim = sandbox.path().join("victim.txt");
    std::fs::write(&victim, b"bye").expect("victim");
    let client = HelperClient::new(sandbox.path().join("bin/vpt-macos"));
    let trashed = client.trash_with_env(&victim, &[("VPT_FAKE_TRASH", sandbox.path().join("trash").to_str().expect("utf8"))]).expect("trashed");
    assert_eq!(trashed, victim);
    assert!(!victim.exists());
    assert!(sandbox.path().join("trash/victim.txt").exists());
}

#[test]
fn a_hung_helper_is_an_unknown_outcome_within_the_call_deadline() {
    let sandbox = Sandbox::new("helper-hang");
    sandbox.install_fake_helper();
    let client = HelperClient::new(sandbox.path().join("bin/vpt-macos")).with_deadline(std::time::Duration::from_millis(200));
    let started = Instant::now();
    let outcome = client.trash_with_env(std::path::Path::new("/tmp/x"), &[("VPT_FAKE_HANG", "1")]);
    assert!(matches!(outcome, Err(TrashError::Unknown(_))), "{outcome:?}");
    assert!(started.elapsed() < std::time::Duration::from_secs(2));
}
```

The `_with_env` variants exist so a test can vary the fake's behavior per call; production calls the
plain forms, which pass no extra environment. Append to `crates/vpt/tests/version.rs`:

```rust
#[test]
fn version_reports_the_helper_when_one_answers_on_path() {
    let sandbox = Sandbox::new("version-helper");
    sandbox.install_fake_helper();

    let output = run(sandbox.vpt().args(["--version", "--json"]));

    let document: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("json");
    assert_eq!(document["helper_version"], "1.0.0");
}
```

and to `support/mod.rs`:

```rust
pub const FAKE_ENGINE: &str = env!("CARGO_BIN_EXE_vpt-fake-engine");

impl Sandbox {
    pub fn install_fake_helper(&self) {
        std::os::unix::fs::symlink(FAKE_ENGINE, self.root.join("bin/vpt-macos")).expect("fake helper on PATH");
        std::fs::create_dir_all(self.root.join("trash")).expect("fake trash");
    }

    pub fn fake_log(&self) -> PathBuf {
        self.root.join("fake.log")
    }
}
```

and `vpt()` additionally sets `VPT_FAKE_TRASH` to `<root>/trash` and `VPT_FAKE_LOG` to `fake_log()`.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-protocol limits &&`
`cargo test -p vpt --features dev-tools --test helper_client --test version`

Expected: compile errors naming `read_bounded`, `HelperClient`; after stubs, the version test FAILS with
`helper_version` null.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-protocol/src/limits.rs`:

```rust
//! Byte, depth, text-length and array-length limits, enforced while a
//! document is read and before it is fully allocated.

use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub bytes: usize,
    pub depth: usize,
    pub text_chars: usize,
    pub array_len: usize,
}

impl Limits {
    /// Every incoming document other than `vpt.engine/1` and `vpt.proposal/1`.
    pub fn incoming() -> Limits {
        Limits { bytes: 65_536, depth: 8, text_chars: 65_536, array_len: 256 }
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
    check_depth(bytes, limits.depth)?;
    let value: Value = serde_json::from_slice(bytes).map_err(|error| LimitViolation::Syntax(error.to_string()))?;
    check_sizes(&value, limits)?;
    Ok(value)
}

/// Count nesting over the raw bytes, ignoring brackets inside strings.
fn check_depth(bytes: &[u8], limit: usize) -> Result<(), LimitViolation> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for byte in bytes {
        if in_string {
            match (escaped, byte) {
                (true, _) => escaped = false,
                (false, b'\\') => escaped = true,
                (false, b'"') => in_string = false,
                _ => {}
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'{' | b'[' => {
                depth += 1;
                if depth > limit {
                    return Err(LimitViolation::Depth(depth));
                }
            }
            b'}' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    Ok(())
}

fn check_sizes(value: &Value, limits: &Limits) -> Result<(), LimitViolation> {
    match value {
        Value::String(text) => {
            let chars = text.chars().count();
            if chars > limits.text_chars { Err(LimitViolation::Text(chars)) } else { Ok(()) }
        }
        Value::Array(items) => {
            if items.len() > limits.array_len {
                return Err(LimitViolation::Array(items.len()));
            }
            items.iter().try_for_each(|item| check_sizes(item, limits))
        }
        Value::Object(fields) => fields.values().try_for_each(|item| check_sizes(item, limits)),
        _ => Ok(()),
    }
}
```

`crates/vpt-protocol/src/helper.rs`:

```rust
//! What the macOS helper prints: its version document and the two replies.

use serde::Deserialize;

pub const HELPER_SCHEMA: &str = "vpt.helper/1";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct HelperVersion {
    pub schema: String,
    pub version: String,
}

impl HelperVersion {
    pub fn major(&self) -> Option<u32> {
        self.version.split('.').next()?.parse().ok()
    }
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

`lib.rs` of the protocol crate lists `error`, `helper`, `limits`, `result`.

`crates/vpt-adapters/src/helper.rs`:

```rust
//! The macOS helper as a client: `--version`, `notify`, `trash`, each a
//! bounded spawn reading one bounded document.

use crate::spawn::{self, SpawnError, Status};
use std::path::{Path, PathBuf};
use std::time::Duration;
use vpt_application::ports::trash::{Trash, TrashError};
use vpt_protocol::helper::{HELPER_SCHEMA, HelperVersion, Posted, Trashed};
use vpt_protocol::limits::{Limits, read_bounded};

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
}

impl HelperClient {
    pub fn new(path: PathBuf) -> HelperClient {
        HelperClient { path, deadline: CALL_DEADLINE }
    }

    pub fn with_deadline(mut self, deadline: Duration) -> HelperClient {
        self.deadline = deadline;
        self
    }

    pub fn version(&self) -> Result<String, HelperError> {
        self.version_with_env(&[])
    }

    pub fn version_with_env(&self, env: &[(&str, &str)]) -> Result<String, HelperError> {
        let value = self.call(&["--version".to_owned()], env)?;
        let document: HelperVersion = serde_json::from_value(value).map_err(|error| HelperError::Unknown(error.to_string()))?;
        if document.schema != HELPER_SCHEMA {
            return Err(HelperError::Unknown(format!("unexpected schema {}", document.schema)));
        }
        match document.major() {
            Some(major) if major == BUILT_AGAINST_MAJOR => Ok(document.version),
            Some(found) => Err(HelperError::MajorMismatch { found }),
            None => Err(HelperError::Unknown("unparseable version".into())),
        }
    }

    pub fn notify(&self, title: &str, body: &str) -> Result<(), HelperError> {
        self.notify_with_env(title, body, &[])
    }

    pub fn notify_with_env(&self, title: &str, body: &str, env: &[(&str, &str)]) -> Result<(), HelperError> {
        let argv = ["notify", "--title", title, "--body", body].map(str::to_owned);
        let value = self.call(&argv, env)?;
        let reply: Posted = serde_json::from_value(value).map_err(|error| HelperError::Unknown(error.to_string()))?;
        if reply.posted { Ok(()) } else { Err(HelperError::Failed("the helper did not post".into())) }
    }

    pub fn trash_with_env(&self, path: &Path, env: &[(&str, &str)]) -> Result<PathBuf, TrashError> {
        let argv = ["trash".to_owned(), path.to_string_lossy().into_owned()];
        let value = self.call(&argv, env).map_err(|error| match error {
            HelperError::Absent => TrashError::HelperAbsent,
            HelperError::Failed(detail) => TrashError::Failed(detail),
            other => TrashError::Unknown(format!("{other:?}")),
        })?;
        let reply: Trashed = serde_json::from_value(value).map_err(|error| TrashError::Unknown(error.to_string()))?;
        Ok(PathBuf::from(reply.trashed))
    }

    fn call(&self, argv: &[String], env: &[(&str, &str)]) -> Result<serde_json::Value, HelperError> {
        let mut full = vec![self.path.to_string_lossy().into_owned()];
        full.extend(argv.iter().cloned());
        let outcome = spawn::run_with_env(&full, b"", self.deadline, spawn::OUTPUT_LIMIT, env).map_err(|error| match error {
            SpawnError::NotFound(_) => HelperError::Absent,
            SpawnError::Io(detail) => HelperError::Failed(detail),
        })?;
        match outcome.status {
            Status::Exited(0) => read_bounded(&outcome.stdout, &Limits::incoming()).map_err(|violation| HelperError::Unknown(format!("{violation:?}"))),
            Status::Exited(code) => Err(HelperError::Failed(format!("the helper exited {code}"))),
            Status::Signaled(signal) => Err(HelperError::Failed(format!("the helper died on signal {signal}"))),
            Status::DeadlineExceeded => Err(HelperError::Unknown("the helper exceeded its deadline".into())),
            Status::Interrupted => Err(HelperError::Unknown("interrupted".into())),
        }
    }
}

impl Trash for HelperClient {
    fn trash(&self, path: &Path) -> Result<PathBuf, TrashError> {
        self.trash_with_env(path, &[])
    }
}
```

Add `pub mod helper;` to the adapters `lib.rs`.

`vpt --version` composes the client from the default helper path. In `crates/vpt/src/lib.rs` the
`Verb::Version` arm becomes:

```rust
        Verb::Version => {
            let helper = HelperClient::new(PathBuf::from("vpt-macos"));
            let version = helper.version().ok();
            Outcome::Success {
                document: commands::version::document(version.as_deref()),
                human: commands::version::human(version.as_deref()),
            }
        }
```

with `use std::path::PathBuf;` and `use vpt_adapters::helper::HelperClient;`. The default `"vpt-macos"`
is the schema's `helper.path` default; Task 27's `Runtime` reads the configured value for every other
verb. `run()` also calls `vpt_adapters::spawn::install_interrupt_handlers()` first.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --workspace --features dev-tools`

Expected: all PASS.

- [ ] **Step 5: Commit**

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(helper): the helper client, its version check and the bounded reader"
```

______________________________________________________________________

### Task 26: Notifications: `vpt.event/1` and the three modes

**Files:**

- Create: `crates/vpt-protocol/src/event.rs`
- Modify: `crates/vpt-protocol/src/lib.rs`
- Create: `crates/vpt-adapters/src/notify/mod.rs`
- Modify: `crates/vpt-adapters/src/lib.rs`
- Test: `crates/vpt/tests/notify.rs`

**Interfaces:**

- Consumes: `HelperClient::notify`, `spawn::run`, `Notification`, `Notifier`, `DeliveryOutcome`.

- Produces:

  - `vpt_protocol::event::EventDocument { pub schema: String, pub event: String,`
    `pub state: String, pub recording: Option<String>, pub detail: String,`
    `pub counts: serde_json::Map<String, serde_json::Value>,`
    `pub paths: serde_json::Map<String, serde_json::Value>, pub occurred_at: String }` (serde
    `Serialize`).
  - `vpt_adapters::notify::{document(notification: &Notification) -> EventDocument,`
    `tokens(notification: &Notification, argv: &[String]) -> Vec<String>,`
    `DesktopNotifier::new(helper: HelperClient) -> DesktopNotifier,`
    `CommandNotifier::new(argv: Vec<String>, fallback: DesktopNotifier) -> CommandNotifier,`
    `OffNotifier, NOTIFY_DEADLINE: Duration = 5 s}`; all three implement `Notifier`.

- [ ] **Step 1: Write the failing tests**

`crates/vpt-adapters/src/notify/mod.rs`, test section:

```rust
#[cfg(test)]
mod tests {
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
            vec![("note".into(), PathBuf::from("/h/transcripts/a.md"))],
            UtcInstant { secs: 1_789_599_200 },
        )
    }

    #[test]
    fn the_event_document_carries_the_spec_fields() {
        let mut notification = review_needed();
        notification.counts = vec![("numeric".into(), 3), ("diagnostics".into(), 0)];
        let json = serde_json::to_value(document(&notification)).expect("json");
        assert_eq!(json["schema"], "vpt.event/1");
        assert_eq!(json["event"], "review_needed");
        assert_eq!(json["state"], "needs_attention");
        assert_eq!(json["recording"], "2026-08-24T144736-4f3ab19c02de");
        assert_eq!(json["counts"]["numeric"], 3);
        assert_eq!(json["paths"]["note"], "/h/transcripts/a.md");
        assert_eq!(json["occurred_at"], "2026-09-16T10:53:20Z");
    }

    #[test]
    fn tokens_are_substituted_inside_arguments_and_absent_values_are_empty() {
        let argv = ["pns", "--event", "{event}", "--id", "{id}", "--n", "{count}", "--path", "{path}", "--state", "{state}"].map(str::to_owned);
        let mut notification = review_needed();
        notification.counts = vec![("numeric".into(), 3), ("other".into(), 2)];
        assert_eq!(
            tokens(&notification, &argv),
            ["pns", "--event", "review_needed", "--id", "2026-08-24T144736-4f3ab19c02de", "--n", "5", "--path", "/h/transcripts/a.md", "--state", "needs_attention"].map(str::to_owned)
        );
        let bare = Notification::failed(EventKind::IngestFailed, "boom".into(), UtcInstant { secs: 0 });
        assert_eq!(tokens(&bare, &["{id}".to_owned(), "{path}".to_owned(), "{detail}".to_owned()]), ["", "", "boom"].map(str::to_owned));
    }

    #[test]
    fn off_raises_nothing_and_reports_suppressed() {
        assert_eq!(OffNotifier.deliver(&review_needed()), DeliveryOutcome::Suppressed);
    }
}
```

`crates/vpt/tests/notify.rs`:

```rust
mod support;

use std::path::{Path, PathBuf};
use support::{FAKE_ENGINE, Sandbox};
use vpt_adapters::helper::HelperClient;
use vpt_adapters::notify::{CommandNotifier, DesktopNotifier};
use vpt_application::ports::notifier::{DeliveryOutcome, Notifier};
use vpt_domain::notification::{EventKind, Notification};
use vpt_domain::time::UtcInstant;

fn event() -> Notification {
    Notification::failed(EventKind::IngestFailed, "the recordings directory is unreadable".into(), UtcInstant { secs: 7 })
}

fn log_lines(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path).unwrap_or_default().lines().map(str::to_owned).collect()
}

#[test]
fn desktop_mode_posts_through_the_helper_with_the_event_as_title() {
    let sandbox = Sandbox::new("notify-desktop");
    sandbox.install_fake_helper();
    let notifier = DesktopNotifier::new(HelperClient::new(sandbox.path().join("bin/vpt-macos"))).with_env(fake_env(&sandbox));

    assert_eq!(notifier.deliver(&event()), DeliveryOutcome::Delivered);

    assert_eq!(log_lines(&sandbox.fake_log()), vec!["notify\tvpt: ingest_failed\tthe recordings directory is unreadable"]);
}

#[test]
fn desktop_mode_with_no_helper_is_suppressed_not_failed() {
    let notifier = DesktopNotifier::new(HelperClient::new(PathBuf::from("/nonexistent/vpt-macos")));
    assert_eq!(notifier.deliver(&event()), DeliveryOutcome::Suppressed);
}

#[test]
fn command_mode_substitutes_tokens_and_writes_the_document_on_stdin() {
    let sandbox = Sandbox::new("notify-command");
    sandbox.install_fake_helper();
    let argv = [FAKE_ENGINE, "command-sink", "--event", "{event}", "--state", "{state}"].map(str::to_owned).to_vec();
    let notifier = CommandNotifier::new(argv, DesktopNotifier::new(HelperClient::new(sandbox.path().join("bin/vpt-macos"))))
        .with_env(fake_env(&sandbox));

    assert_eq!(notifier.deliver(&event()), DeliveryOutcome::Delivered);

    let lines = log_lines(&sandbox.fake_log());
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].starts_with("command-sink\t--event ingest_failed --state failed\t"), "{}", lines[0]);
    assert!(lines[0].contains(r#"\"schema\":\"vpt.event/1\""#) || lines[0].contains(r#""schema":"vpt.event/1""#), "{}", lines[0]);
}

#[test]
fn a_non_zero_command_falls_back_to_the_desktop_notice_once_and_reports_failed() {
    let sandbox = Sandbox::new("notify-fallback");
    sandbox.install_fake_helper();
    let argv = [FAKE_ENGINE, "command-sink"].map(str::to_owned).to_vec();
    let mut env = fake_env(&sandbox);
    env.push(("VPT_FAKE_EXIT".into(), "7".into()));
    let notifier = CommandNotifier::new(argv, DesktopNotifier::new(HelperClient::new(sandbox.path().join("bin/vpt-macos")))).with_env(env);

    let outcome = notifier.deliver(&event());

    assert!(matches!(outcome, DeliveryOutcome::Failed(ref detail) if detail.contains('7')), "{outcome:?}");
    let lines = log_lines(&sandbox.fake_log());
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(lines[1].starts_with("notify\tvpt: ingest_failed"), "{}", lines[1]);
}

/// The fake reads VPT_FAKE_LOG from its own environment; these in-process tests
/// hand it through the notifier's env list rather than the test process env.
fn fake_env(sandbox: &Sandbox) -> Vec<(String, String)> {
    vec![("VPT_FAKE_LOG".into(), sandbox.fake_log().to_string_lossy().into_owned())]
}
```

`with_env` on either notifier sets what the spawned child sees; `CommandNotifier::with_env` hands the
same list to its desktop fallback, so one call configures both paths.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-adapters notify && cargo test -p vpt --features dev-tools --test notify`
Expected: compile errors naming `document`, `tokens`, `OffNotifier`, `DesktopNotifier`,
`CommandNotifier`.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-protocol/src/event.rs`:

```rust
//! `vpt.event/1`: identities, counts, classes and paths, never recording text.

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
//! The three `[notify]` modes over the producer API: desktop through the
//! helper, a configured command with tokens and JSON on stdin, or off.

use crate::helper::{HelperClient, HelperError};
use crate::spawn::{self, Status};
use serde_json::{Map, Value};
use std::time::Duration;
use vpt_application::ports::notifier::{DeliveryOutcome, Notifier};
use vpt_domain::notification::Notification;
use vpt_protocol::event::{EVENT_SCHEMA, EventDocument};

pub const NOTIFY_DEADLINE: Duration = Duration::from_secs(5);

pub fn document(notification: &Notification) -> EventDocument {
    let mut counts = Map::new();
    for (name, count) in &notification.counts {
        counts.insert(name.clone(), Value::from(*count));
    }
    let mut paths = Map::new();
    for (name, path) in &notification.paths {
        paths.insert(name.clone(), Value::String(path.to_string_lossy().into_owned()));
    }
    EventDocument {
        schema: EVENT_SCHEMA.into(),
        event: notification.event.as_str().into(),
        state: notification.state.as_str().into(),
        recording: notification.recording.as_ref().map(|id| id.as_str().to_owned()),
        detail: notification.detail.clone(),
        counts,
        paths,
        occurred_at: notification.occurred_at.rfc3339(),
    }
}

pub fn tokens(notification: &Notification, argv: &[String]) -> Vec<String> {
    let count: u64 = notification.counts.iter().map(|(_, n)| n).sum();
    let path = notification.paths.first().map(|(_, p)| p.to_string_lossy().into_owned()).unwrap_or_default();
    let id = notification.recording.as_ref().map(|id| id.as_str().to_owned()).unwrap_or_default();
    argv.iter()
        .map(|word| {
            word.replace("{event}", notification.event.as_str())
                .replace("{state}", notification.state.as_str())
                .replace("{id}", &id)
                .replace("{detail}", &notification.detail)
                .replace("{count}", &count.to_string())
                .replace("{path}", &path)
        })
        .collect()
}

pub struct DesktopNotifier {
    helper: HelperClient,
    env: Vec<(String, String)>,
}

impl DesktopNotifier {
    pub fn new(helper: HelperClient) -> DesktopNotifier {
        DesktopNotifier { helper, env: vec![] }
    }

    pub fn with_env(mut self, env: Vec<(String, String)>) -> DesktopNotifier {
        self.env = env;
        self
    }
}

fn borrowed(env: &[(String, String)]) -> Vec<(&str, &str)> {
    env.iter().map(|(key, value)| (key.as_str(), value.as_str())).collect()
}

impl Notifier for DesktopNotifier {
    fn deliver(&self, notification: &Notification) -> DeliveryOutcome {
        let title = format!("vpt: {}", notification.event.as_str());
        match self.helper.notify_with_env(&title, &notification.detail, &borrowed(&self.env)) {
            Ok(()) => DeliveryOutcome::Delivered,
            Err(HelperError::Absent) => DeliveryOutcome::Suppressed,
            Err(error) => DeliveryOutcome::Failed(format!("{error:?}")),
        }
    }
}

pub struct CommandNotifier {
    argv: Vec<String>,
    fallback: DesktopNotifier,
    env: Vec<(String, String)>,
}

impl CommandNotifier {
    pub fn new(argv: Vec<String>, fallback: DesktopNotifier) -> CommandNotifier {
        CommandNotifier { argv, fallback, env: vec![] }
    }

    pub fn with_env(mut self, env: Vec<(String, String)>) -> CommandNotifier {
        self.fallback.env = env.clone();
        self.env = env;
        self
    }
}

impl Notifier for CommandNotifier {
    fn deliver(&self, notification: &Notification) -> DeliveryOutcome {
        let argv = tokens(notification, &self.argv);
        let body = serde_json::to_vec(&document(notification)).unwrap_or_default();
        let detail = match spawn::run_with_env(&argv, &body, NOTIFY_DEADLINE, spawn::OUTPUT_LIMIT, &borrowed(&self.env)) {
            Ok(outcome) => match outcome.status {
                Status::Exited(0) => return DeliveryOutcome::Delivered,
                Status::Exited(code) => format!("notify command exited {code}"),
                Status::Signaled(signal) => format!("notify command died on signal {signal}"),
                Status::DeadlineExceeded => "notify command exceeded its deadline".into(),
                Status::Interrupted => "interrupted".into(),
            },
            Err(error) => format!("notify command could not start: {error:?}"),
        };
        let _ = self.fallback.deliver(notification);
        DeliveryOutcome::Failed(detail)
    }
}

pub struct OffNotifier;

impl Notifier for OffNotifier {
    fn deliver(&self, _notification: &Notification) -> DeliveryOutcome {
        DeliveryOutcome::Suppressed
    }
}
```

Add `pub mod notify;` to the adapters `lib.rs` and `pub mod event;` to the protocol `lib.rs`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --workspace --features dev-tools`

Expected: all PASS.

- [ ] **Step 5: Commit**

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(notify): vpt.event/1 and the desktop, command and off modes"
```

______________________________________________________________________

### Task 27: `vpt ingest` at the command line: composition, the lock, repair before work

**Files:**

- Create: `crates/vpt-adapters/src/clock.rs`
- Modify: `crates/vpt-adapters/src/lib.rs`
- Modify: `crates/vpt/src/compose.rs`
- Create: `crates/vpt/src/commands/ingest.rs`, `crates/vpt/src/documents/mod.rs`,
  `crates/vpt/src/documents/record.rs`
- Modify: `crates/vpt/src/lib.rs`, `crates/vpt/src/commands/mod.rs`
- Test: `crates/vpt/tests/ingest.rs`; `crates/vpt/tests/support/mod.rs` gains `write_config` and
  `add_recording`

**Interfaces:**

- Consumes: everything built so far.

- Produces:

  - `vpt_adapters::clock::SystemClock` (implements `Clock`; `offset_at` through `libc::localtime_r`).
  - `vpt::compose::{Runtime, WRITE_LOCK_WAIT: Duration = 5 s}` with
    `Runtime::load(environment: &Environment, config: Option<&Path>, creation: Creation) ->`
    `Result<Runtime, ErrorDocument>`; fields
    `pub settings: Settings, pub roots: Roots, pub ledger: SqliteLedger,`
    `pub recorder: VoiceMemosStore, pub archive: ClonefileArchive, pub clock: SystemClock,`
    `pub helper: HelperClient, pub notifier: Box<dyn Notifier>`;
    `Runtime::mutating(&self) -> Result<WriteLock, ErrorDocument>` (takes the lock, then repairs
    publications and reconciles nothing else yet); `Runtime::trash(&self) -> &dyn Trash`.
  - `vpt::documents::record::record_json(record: &RecordingRecord) -> serde_json::Value`.
  - `vpt::commands::ingest::run(runtime: &Runtime, dry_run: bool, once: Option<PathBuf>) -> Outcome`.
  - Test support: `Sandbox::write_config(&self, extra: &str)` writes a config pointing every path into
    the sandbox (home `home/.vpt`, state `state/vpt`, recordings `recordings`, helper `bin/vpt-macos`,
    notify `off`) followed by `extra`;
    `Sandbox::add_recording(&self, name: &str, bytes: &[u8]) -> PathBuf` (mtime 60 s in the past);
    `Sandbox::ledger(&self) -> rusqlite::Connection` opened on `state/vpt/vpt.db`.

- [ ] **Step 1: Write the failing tests**

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
    let digest = vpt_adapters::stores::digest_file(&target).expect("digest").hex();
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
    assert!(!sandbox.path().join("state/vpt/title-copy").exists());
    assert!(std::fs::read_dir(sandbox.path().join("home/.vpt/audio")).map(|mut d| d.next().is_none()).unwrap_or(true));
}
```

Support additions in `crates/vpt/tests/support/mod.rs`:

```rust
impl Sandbox {
    pub fn write_config(&self, extra: &str) {
        let config = format!(
            "config_version = 1\n[home]\npath = \"{home}\"\nstate_dir = \"{state}\"\n[helper]\npath = \"{helper}\"\n\
             [source]\nrecordings_dir = \"{recordings}\"\n[notify]\nmode = \"off\"\n{extra}",
            home = self.root.join("home/.vpt").display(),
            state = self.root.join("state/vpt").display(),
            helper = self.root.join("bin/vpt-macos").display(),
            recordings = self.root.join("recordings").display(),
        );
        std::fs::create_dir_all(self.config_path().parent().expect("dir")).expect("config dir");
        std::fs::write(self.config_path(), config).expect("config");
        std::fs::create_dir_all(self.root.join("home/.vpt")).expect("home");
    }

    pub fn add_recording(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.root.join("recordings").join(name);
        std::fs::write(&path, bytes).expect("recording");
        let file = std::fs::File::options().write(true).open(&path).expect("open");
        file.set_times(std::fs::FileTimes::new().set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(60))).expect("mtime");
        path
    }

    pub fn ledger(&self) -> rusqlite::Connection {
        rusqlite::Connection::open(self.root.join("state/vpt/vpt.db")).expect("ledger")
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt --features dev-tools --test ingest`

Expected: every test FAILS with exit 2 and "verb not implemented yet".

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-adapters/src/clock.rs`:

```rust
//! The system clock, and the local offset at an instant through `localtime_r`.

use vpt_application::ports::clock::Clock;
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
//! The composition root: the environment, the settings and the adapters every
//! command shares. Nothing here decides policy.

use std::path::{Path, PathBuf};
use std::time::Duration;
use vpt_adapters::archive::ClonefileArchive;
use vpt_adapters::clock::SystemClock;
use vpt_adapters::config::load::{ConfigError, load_file};
use vpt_adapters::config::paths::{config_path, default_state_dir};
use vpt_adapters::config::roots::{Creation, RootError, Roots, resolve};
use vpt_adapters::config::settings::from_table;
use vpt_adapters::helper::HelperClient;
use vpt_adapters::ledger::sqlite::SqliteLedger;
use vpt_adapters::lock::{LockError, WriteLock};
use vpt_adapters::notify::{CommandNotifier, DesktopNotifier, OffNotifier};
use vpt_adapters::stores::FilesystemStores;
use vpt_adapters::voice_memos::store::VoiceMemosStore;
use vpt_application::ports::artifacts::NoRenderers;
use vpt_application::ports::notifier::Notifier;
use vpt_application::ports::trash::Trash;
use vpt_application::publication::{RepairError, repair_publications};
use vpt_application::settings::{NotifyMode, Settings};
use vpt_domain::notification::{EventKind, Notification};
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub const WRITE_LOCK_WAIT: Duration = Duration::from_secs(5);

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
        config_path(&|name| self.var(name))
    }

    pub fn home_dir(&self) -> PathBuf {
        PathBuf::from(self.var("HOME").unwrap_or_default())
    }

    pub fn state_dir_default(&self) -> String {
        default_state_dir(&|name| self.var(name))
    }
}

pub struct Runtime {
    pub settings: Settings,
    pub roots: Roots,
    pub ledger: SqliteLedger,
    pub recorder: VoiceMemosStore,
    pub archive: ClonefileArchive,
    pub clock: SystemClock,
    pub helper: HelperClient,
    pub notifier: Box<dyn Notifier>,
}

pub fn config_error(error: &ConfigError, path: &Path) -> ErrorDocument {
    let message = match error {
        ConfigError::Missing(_) => format!("no configuration at {}; run `vpt setup`", path.display()),
        ConfigError::Unreadable { detail, .. } => format!("configuration unreadable: {detail}"),
        ConfigError::Unparseable(detail) => format!("configuration does not parse: {detail}"),
        ConfigError::MissingVersion => "config_version is missing".into(),
        ConfigError::UnsupportedVersion(found) => format!("config_version {found} is not supported (this build reads 1)"),
        ConfigError::UnknownKey(key) => format!("unknown key {key}"),
        ConfigError::WrongType { key, expected } => format!("{key} must be a {expected}"),
        ConfigError::OutOfRange { key, rule } => format!("{key} must be {rule}"),
    };
    ErrorDocument::new(ErrorKind::Config, message)
}

fn root_error(error: &RootError) -> ErrorDocument {
    let message = match error {
        RootError::NotAbsolute { key } => format!("{key} must be absolute after expansion"),
        RootError::ParentMissing { key } => format!("{key}: the parent directory does not exist"),
        RootError::Overlap { first, second } => format!("{first} overlaps {second}"),
        RootError::Io { key, detail } => format!("{key}: {detail}"),
    };
    ErrorDocument::new(ErrorKind::Config, message)
}

pub fn settings_from(environment: &Environment, config: Option<&Path>) -> Result<Settings, ErrorDocument> {
    let path = config.map(Path::to_path_buf).unwrap_or_else(|| environment.config_path());
    let table = load_file(&path).map_err(|error| config_error(&error, &path))?;
    from_table(&table, &environment.home_dir()).map_err(|error| config_error(&error, &path))
}

pub fn notifier_for(settings: &Settings) -> Box<dyn Notifier> {
    let desktop = DesktopNotifier::new(HelperClient::new(settings.helper_path.clone()));
    match &settings.notify.mode {
        NotifyMode::Desktop => Box::new(desktop),
        NotifyMode::Command(argv) => Box::new(CommandNotifier::new(argv.clone(), desktop)),
        NotifyMode::Off => Box::new(OffNotifier),
    }
}

impl Runtime {
    pub fn load(environment: &Environment, config: Option<&Path>, creation: Creation) -> Result<Runtime, ErrorDocument> {
        let settings = settings_from(environment, config)?;
        let notifier = notifier_for(&settings);
        let roots = match resolve(&settings, creation) {
            Ok(roots) => roots,
            Err(error) => {
                let document = root_error(&error);
                notifier.deliver(&Notification::failed(EventKind::ConfigRefused, document.message.clone(), SystemClock.now()));
                return Err(document);
            }
        };
        let ledger = SqliteLedger::open(&roots.state_dir).map_err(|error| ErrorDocument::new(ErrorKind::Ledger, format!("{error:?}")))?;
        Ok(Runtime {
            recorder: VoiceMemosStore::new(roots.recordings_dir.clone(), roots.state_dir.clone(), settings.source.read_titles),
            archive: ClonefileArchive::new(roots.stores.audio.clone()),
            helper: HelperClient::new(settings.helper_path.clone()),
            clock: SystemClock,
            ledger,
            roots,
            settings,
            notifier,
        })
    }

    /// The lock every mutating command holds, taken before the repair of
    /// unfinished publications that precedes new work.
    pub fn mutating(&self) -> Result<WriteLock, ErrorDocument> {
        let lock = WriteLock::acquire(&self.roots.state_dir, WRITE_LOCK_WAIT).map_err(|error| match error {
            LockError::Busy => ErrorDocument::new(ErrorKind::Ledger, "another vpt command holds the write lock"),
            LockError::Io(detail) => ErrorDocument::new(ErrorKind::Ledger, detail),
        })?;
        repair_publications(&self.ledger, &FilesystemStores, &NoRenderers).map_err(|error| match error {
            RepairError::TargetModified(path) => ErrorDocument::new(
                ErrorKind::Refused,
                format!("{} is not what the ledger last published", path.display()),
            )
            .rule("target_modified"),
            RepairError::Sync { path, detail } => ErrorDocument::new(ErrorKind::Store, format!("{}: {detail}", path.display())),
            RepairError::Render(path) => ErrorDocument::new(ErrorKind::Refused, format!("{}: no renderer for this artifact", path.display())).rule("target_modified"),
            RepairError::Ledger(error) => ErrorDocument::new(ErrorKind::Ledger, format!("{error:?}")),
            RepairError::Files(error) => ErrorDocument::new(ErrorKind::Store, error.0),
        })?;
        Ok(lock)
    }

    pub fn trash(&self) -> &dyn Trash {
        &self.helper
    }
}
```

`SystemClock.now()` needs `use vpt_application::ports::clock::Clock;` in scope.

`crates/vpt/src/documents/mod.rs`: `pub mod record;`. `crates/vpt/src/documents/record.rs`:

```rust
//! A recording's record as `vpt show` and `vpt list` print it.

use serde_json::{Value, json};
use vpt_application::ports::ledger::RecordingRecord;

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
use crate::documents::record::record_json;
use serde_json::json;
use std::path::PathBuf;
use vpt_application::ingest::{Ingest, IngestError, IngestFailure, IngestReport, Mode};
use vpt_protocol::error::{ErrorDocument, ErrorKind};
use vpt_protocol::result::document;

pub fn run(runtime: &Runtime, dry_run: bool, once: Option<PathBuf>) -> Outcome {
    let lock = if dry_run { None } else { match runtime.mutating() { Ok(lock) => Some(lock), Err(error) => return Outcome::Failure(error) } };
    let ingest = Ingest {
        recorder: &runtime.recorder,
        archive: &runtime.archive,
        ledger: &runtime.ledger,
        clock: &runtime.clock,
        trash: runtime.trash(),
        notifier: runtime.notifier.as_ref(),
        settings: &runtime.settings.source,
    };
    let mode = match (dry_run, once) {
        (true, _) => Mode::DryRun,
        (false, Some(path)) => Mode::Once(path),
        (false, None) => Mode::Sweep,
    };
    let outcome = match ingest.run(mode) {
        Ok(report) => Outcome::Success { human: human(&report), document: document("ingest", body(&report)) },
        Err(error) => Outcome::Failure(failure(error)),
    };
    drop(lock);
    outcome
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
        IngestFailure::Ledger(_) => ErrorDocument::new(ErrorKind::Ledger, message),
        _ => ErrorDocument::new(ErrorKind::Store, message),
    };
    document.completed(completed)
}
```

In `crates/vpt/src/lib.rs`, `dispatch` gains:

```rust
        Verb::Ingest { dry_run, once } => {
            let creation = if *dry_run { Creation::None } else { Creation::CreateLeaves };
            match Runtime::load(&environment, invocation.config.as_deref(), creation) {
                Ok(runtime) => commands::ingest::run(&runtime, *dry_run, once.clone()),
                Err(error) => Outcome::Failure(error),
            }
        }
```

with `let environment = Environment::from_process();` at the top of `dispatch`, `pub mod documents;`, and
the imports `use compose::{Environment, Runtime};` and `use vpt_adapters::config::roots::Creation;`.
`commands/mod.rs` lists `ingest`, `setup`, `version`. Add `pub mod clock;` to the adapters `lib.rs`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --workspace --features dev-tools`

Expected: all PASS. Also run `cargo clippy --workspace --all-targets --features dev-tools -- -D warnings`
and expect no warnings; `main.rs` stays at three lines and `lib.rs` under 150.

- [ ] **Step 5: Commit**

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

- Create: `crates/vpt-application/src/ports/stores.rs`, `crates/vpt-application/src/inventory.rs`
- Modify: `crates/vpt-application/src/ports/mod.rs`, `crates/vpt-application/src/lib.rs`
- Modify: `crates/vpt-adapters/src/stores.rs`
- Create: `crates/vpt/src/commands/show.rs`, `crates/vpt/src/commands/list.rs`,
  `crates/vpt/src/commands/storage.rs`
- Modify: `crates/vpt/src/commands/mod.rs`, `crates/vpt/src/lib.rs`
- Test: `crates/vpt/tests/show_list_storage.rs`

**Interfaces:**

- Consumes: `RecordingLedger::{by_id, recordings}`, `record_json`, `Runtime`, `StoreKey::all`.

- Produces:

  - `vpt_application::ports::stores::{StoreEntry { pub path: PathBuf, pub size: u64,`
    `pub mtime: FileTime }, Stores}` with
    `fn entries(&self, root: &Path) -> Result<Vec<StoreEntry>, ArtifactError>` (regular files at depth
    one, symbolic links skipped, an absent root an empty list).
  - `vpt_application::inventory::{StoreInventory { pub files: u64, pub bytes: u64,`
    `pub oldest: Option<FileTime>, pub newest: Option<FileTime> },`
    `inventory(entries: &[StoreEntry]) -> StoreInventory}`.
  - `impl Stores for FilesystemStores`.
  - `vpt::commands::{show::run(runtime: &Runtime, id: &str) -> Outcome,`
    `list::run(runtime: &Runtime, stage: Option<&str>) -> Outcome,`
    `storage::run(runtime: &Runtime) -> Outcome}`.

- [ ] **Step 1: Write the failing tests**

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
        assert_eq!(inventory(&[]), StoreInventory { files: 0, bytes: 0, oldest: None, newest: None });
    }

    #[test]
    fn files_and_bytes_are_summed_and_the_extremes_found() {
        let report = inventory(&[entry(10, 300), entry(5, 100), entry(1, 200)]);
        assert_eq!(report.files, 3);
        assert_eq!(report.bytes, 16);
        assert_eq!(report.oldest, Some(FileTime { secs: 100, nanos: 0 }));
        assert_eq!(report.newest, Some(FileTime { secs: 300, nanos: 0 }));
    }
}
```

`crates/vpt-adapters/src/stores.rs`, appended tests:

```rust
    #[test]
    fn entries_lists_regular_files_at_depth_one_and_nothing_else() {
        let dir = tempfile::tempdir().expect("dir");
        std::fs::write(dir.path().join("a.m4a"), b"aaa").expect("a");
        std::fs::create_dir(dir.path().join("sub")).expect("sub");
        std::fs::write(dir.path().join("sub/b.m4a"), b"b").expect("b");
        std::os::unix::fs::symlink(dir.path().join("a.m4a"), dir.path().join("link.m4a")).expect("link");
        let mut names: Vec<String> = FilesystemStores.entries(dir.path()).expect("entries").into_iter().map(|e| e.path.file_name().unwrap().to_string_lossy().into_owned()).collect();
        names.sort();
        assert_eq!(names, ["a.m4a"]);
        assert!(FilesystemStores.entries(&dir.path().join("absent")).expect("absent").is_empty());
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
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-application inventory && cargo test -p vpt-adapters stores &&`
`cargo test -p vpt --features dev-tools --test show_list_storage`

Expected: compile errors for `inventory` and `entries`; the three command tests FAIL with exit 2 "verb
not implemented yet".

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-application/src/ports/stores.rs`:

```rust
//! What a store holds, as the filesystem reports it.

use super::artifacts::ArtifactError;
use std::path::{Path, PathBuf};
use vpt_domain::time::FileTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreEntry {
    pub path: PathBuf,
    pub size: u64,
    pub mtime: FileTime,
}

pub trait Stores {
    fn entries(&self, root: &Path) -> Result<Vec<StoreEntry>, ArtifactError>;
}
```

`crates/vpt-application/src/inventory.rs`:

```rust
//! `vpt storage`: counts, bytes and the mtime extremes of one store.

use crate::ports::stores::StoreEntry;
use vpt_domain::time::FileTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoreInventory {
    pub files: u64,
    pub bytes: u64,
    pub oldest: Option<FileTime>,
    pub newest: Option<FileTime>,
}

pub fn inventory(entries: &[StoreEntry]) -> StoreInventory {
    let key = |time: &FileTime| (time.secs, time.nanos);
    StoreInventory {
        files: entries.len() as u64,
        bytes: entries.iter().map(|entry| entry.size).sum(),
        oldest: entries.iter().map(|entry| entry.mtime).min_by_key(key),
        newest: entries.iter().map(|entry| entry.mtime).max_by_key(key),
    }
}
```

`ports/mod.rs` adds `pub mod stores;`, the application `lib.rs` adds `pub mod inventory;`. In
`crates/vpt-adapters/src/stores.rs`:

```rust
impl Stores for FilesystemStores {
    fn entries(&self, root: &Path) -> Result<Vec<StoreEntry>, ArtifactError> {
        let directory = match std::fs::read_dir(root) {
            Ok(directory) => directory,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(ArtifactError(error.to_string())),
        };
        let mut entries = Vec::new();
        for entry in directory {
            let entry = entry.map_err(|error| ArtifactError(error.to_string()))?;
            let metadata = entry.metadata().map_err(|error| ArtifactError(error.to_string()))?;
            if !metadata.is_file() {
                continue;
            }
            entries.push(StoreEntry {
                path: entry.path(),
                size: metadata.len(),
                mtime: FileTime { secs: metadata.mtime(), nanos: metadata.mtime_nsec() as u32 },
            });
        }
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(entries)
    }
}
```

`DirEntry::metadata` does not follow symbolic links, which is what skips `link.m4a`. The imports are
`std::os::unix::fs::MetadataExt`, `vpt_application::ports::stores::{StoreEntry, Stores}` and
`vpt_domain::time::FileTime`.

`crates/vpt/src/commands/show.rs`:

```rust
//! `vpt show <id>`.

use crate::cli::output::Outcome;
use crate::compose::Runtime;
use crate::documents::record::record_json;
use vpt_application::ports::ledger::RecordingLedger;
use vpt_domain::identity::RecordingId;
use vpt_protocol::error::{ErrorDocument, ErrorKind};
use vpt_protocol::result::document;

pub fn run(runtime: &Runtime, id: &str) -> Outcome {
    let Ok(identity) = RecordingId::parse(id) else {
        return Outcome::Failure(ErrorDocument::new(ErrorKind::Usage, format!("{id} is not a recording id")));
    };
    match runtime.ledger.by_id(&identity) {
        Ok(Some(record)) => {
            let json = record_json(&record);
            Outcome::Success { human: format!("{}\n", serde_json::to_string_pretty(&json).unwrap_or_default()), document: document("show", json) }
        }
        Ok(None) => Outcome::Failure(ErrorDocument::new(ErrorKind::Usage, format!("no recording {id}")).ids(vec![id.to_owned()])),
        Err(error) => Outcome::Failure(ErrorDocument::new(ErrorKind::Ledger, format!("{error:?}"))),
    }
}
```

`crates/vpt/src/commands/list.rs`:

```rust
//! `vpt list [--stage <stage>]`.

use crate::cli::output::Outcome;
use crate::compose::Runtime;
use crate::documents::record::record_json;
use serde_json::json;
use vpt_application::ports::ledger::{RecordingLedger, RecordingRecord, StageState};
use vpt_protocol::error::{ErrorDocument, ErrorKind};
use vpt_protocol::result::document;

pub fn run(runtime: &Runtime, stage: Option<&str>) -> Outcome {
    let outstanding: fn(&RecordingRecord) -> StageState = match stage {
        None => |_| StageState::Pending,
        Some("transcribe") => |record| record.stages.transcribe,
        Some("note") => |record| record.stages.note,
        Some("synthesis") => |record| record.stages.synthesis,
        Some(other) => return Outcome::Failure(ErrorDocument::new(ErrorKind::Usage, format!("{other} is not a stage"))),
    };
    let records = match runtime.ledger.recordings() {
        Ok(records) => records,
        Err(error) => return Outcome::Failure(ErrorDocument::new(ErrorKind::Ledger, format!("{error:?}"))),
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
use crate::compose::Runtime;
use serde_json::{Map, Value, json};
use vpt_adapters::stores::FilesystemStores;
use vpt_application::inventory::inventory;
use vpt_application::ports::stores::Stores;
use vpt_domain::layout::StoreKey;
use vpt_domain::time::{FileTime, UtcInstant};
use vpt_protocol::error::{ErrorDocument, ErrorKind};
use vpt_protocol::result::document;

fn stamp(time: Option<FileTime>) -> Value {
    time.map_or(Value::Null, |t| Value::String(UtcInstant { secs: t.secs }.rfc3339()))
}

pub fn run(runtime: &Runtime) -> Outcome {
    let mut stores = Map::new();
    let mut human = String::new();
    for key in StoreKey::all() {
        let root = runtime.roots.stores.get(key);
        let entries = match FilesystemStores.entries(root) {
            Ok(entries) => entries,
            Err(error) => return Outcome::Failure(ErrorDocument::new(ErrorKind::Store, format!("{}: {}", root.display(), error.0))),
        };
        let report = inventory(&entries);
        human.push_str(&format!("{:<15} {:>6} files {:>12} bytes  {}\n", key.key_name(), report.files, report.bytes, root.display()));
        stores.insert(
            key.key_name().to_owned(),
            json!({"files": report.files, "bytes": report.bytes, "oldest": stamp(report.oldest), "newest": stamp(report.newest), "path": root}),
        );
    }
    Outcome::Success { human, document: document("storage", json!({"stores": stores})) }
}
```

The dispatch arms in `crates/vpt/src/lib.rs`:

```rust
        Verb::Show { id } => with_runtime(&environment, invocation, Creation::None, |runtime| commands::show::run(runtime, id)),
        Verb::List { stage } => {
            with_runtime(&environment, invocation, Creation::None, |runtime| commands::list::run(runtime, stage.as_deref()))
        }
        Verb::Storage => with_runtime(&environment, invocation, Creation::None, |runtime| commands::storage::run(runtime)),
```

with the helper, which the `Ingest` arm from Task 27 now also uses:

```rust
fn with_runtime(environment: &Environment, invocation: &Invocation, creation: Creation, command: impl FnOnce(&Runtime) -> Outcome) -> Outcome {
    match Runtime::load(environment, invocation.config.as_deref(), creation) {
        Ok(runtime) => command(&runtime),
        Err(error) => Outcome::Failure(error),
    }
}
```

The `Ingest` arm of Task 27 becomes:

```rust
        Verb::Ingest { dry_run, once } => {
            let creation = if *dry_run { Creation::None } else { Creation::CreateLeaves };
            with_runtime(&environment, invocation, creation, |runtime| commands::ingest::run(runtime, *dry_run, once.clone()))
        }
```

`commands/mod.rs` lists `ingest`, `list`, `setup`, `show`, `storage`, `version`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --workspace --features dev-tools`

Expected: all PASS.

- [ ] **Step 5: Commit**

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

- Consumes: `Settings::{home, symlink_target}`, `settings_from`.

- Produces: `vpt_adapters::symlink::{LinkState::{Absent, LinkTo(PathBuf), Other(String)},`
  `inspect(link: &Path) -> LinkState, deploy(link: &Path, target: &Path) -> Result<(),`
  `SymlinkError>, verify(link: &Path, target: &Path) -> Result<(), SymlinkError>,`
  `SymlinkError::{Occupied(String), TargetParentMissing, Missing, WrongTarget(PathBuf),` `Io(String)}}`
  and `vpt::commands::symlink::{deploy(environment, config) -> Outcome, verify(environment,`
  `config) -> Outcome}`.

- [ ] **Step 1: Write the failing tests**

`crates/vpt-adapters/src/symlink.rs`, test section:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
        let dir = tempfile::tempdir().expect("dir");
        std::fs::create_dir(dir.path().join("notes")).expect("parent");
        let link = dir.path().join(".vpt");
        let target = dir.path().join("notes/vpt");
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
        sandbox.path().join("recordings").display()
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

Run: `cargo test -p vpt-adapters symlink && cargo test -p vpt --features dev-tools --test symlink`
Expected: compile errors naming `deploy` and `verify`; the command tests FAIL with exit 2.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-adapters/src/symlink.rs`:

```rust
//! The managed link at the default home: created, verified, never removed.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkState {
    Absent,
    LinkTo(PathBuf),
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymlinkError {
    Occupied(String),
    TargetParentMissing,
    Missing,
    WrongTarget(PathBuf),
    Io(String),
}

pub fn inspect(link: &Path) -> LinkState {
    match std::fs::symlink_metadata(link) {
        Err(_) => LinkState::Absent,
        Ok(metadata) if metadata.file_type().is_symlink() => {
            std::fs::read_link(link).map_or(LinkState::Other("an unreadable link".into()), LinkState::LinkTo)
        }
        Ok(metadata) if metadata.is_dir() => LinkState::Other("a directory".into()),
        Ok(_) => LinkState::Other("a file".into()),
    }
}

pub fn deploy(link: &Path, target: &Path) -> Result<(), SymlinkError> {
    match inspect(link) {
        LinkState::LinkTo(existing) if existing == target => return Ok(()),
        LinkState::LinkTo(existing) => return Err(SymlinkError::Occupied(format!("a link to {}", existing.display()))),
        LinkState::Other(what) => return Err(SymlinkError::Occupied(what)),
        LinkState::Absent => {}
    }
    let parent = target.parent().ok_or(SymlinkError::TargetParentMissing)?;
    if !parent.is_dir() {
        return Err(SymlinkError::TargetParentMissing);
    }
    if !target.is_dir() {
        std::fs::create_dir(target).map_err(|error| SymlinkError::Io(error.to_string()))?;
    }
    std::os::unix::fs::symlink(target, link).map_err(|error| SymlinkError::Io(error.to_string()))
}

pub fn verify(link: &Path, target: &Path) -> Result<(), SymlinkError> {
    match inspect(link) {
        LinkState::LinkTo(existing) if existing == target => Ok(()),
        LinkState::LinkTo(existing) => Err(SymlinkError::WrongTarget(existing)),
        LinkState::Absent => Err(SymlinkError::Missing),
        LinkState::Other(what) => Err(SymlinkError::Occupied(what)),
    }
}
```

Add `pub mod symlink;` to the adapters `lib.rs`. `crates/vpt/src/commands/symlink.rs`:

```rust
//! `vpt symlink deploy` and `vpt symlink verify`.

use crate::cli::output::Outcome;
use crate::compose::{Environment, settings_from};
use serde_json::json;
use std::path::{Path, PathBuf};
use vpt_adapters::symlink::{SymlinkError, deploy as deploy_link, verify as verify_link};
use vpt_protocol::error::{ErrorDocument, ErrorKind};
use vpt_protocol::result::document;

fn link_and_target(environment: &Environment, config: Option<&Path>) -> Result<(PathBuf, PathBuf), ErrorDocument> {
    let settings = settings_from(environment, config)?;
    let target = settings.symlink_target.ok_or_else(|| ErrorDocument::new(ErrorKind::Usage, "home.symlink_target is not set"))?;
    Ok((settings.home, target))
}

fn describe(error: &SymlinkError, link: &Path, target: &Path) -> String {
    match error {
        SymlinkError::Occupied(what) => format!("{} exists and is {what}", link.display()),
        SymlinkError::TargetParentMissing => format!("the parent of {} does not exist", target.display()),
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
        Err(error) => Outcome::Failure(error),
    }
}

pub fn verify(environment: &Environment, config: Option<&Path>) -> Outcome {
    match link_and_target(environment, config) {
        Ok((link, target)) => outcome("symlink verify", "symlink_verify", verify_link(&link, &target), &link, &target),
        Err(error) => Outcome::Failure(error),
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

Run: `cargo test --workspace --features dev-tools`

Expected: all PASS.

- [ ] **Step 5: Commit**

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
- Create: `crates/vpt/src/doctor/mod.rs`, `crates/vpt/src/doctor/checks.rs`
- Modify: `crates/vpt/src/lib.rs`
- Test: `crates/vpt/tests/doctor.rs`

**Interfaces:**

- Consumes: `settings_from`, `resolve`, `HelperClient::version`,
  `VoiceMemosStore::{candidates, subdirectory_counts}`, `symlink::{inspect, verify}`,
  `ClonefileArchive::staged_leftovers`, `RecordingLedger::seen_all`, `Check`, `ErrorDocument::checks`.

- Produces:

  - `vpt_adapters::git_tree::enclosing_git_tree(path: &Path) -> Option<PathBuf>` (the nearest ancestor,
    the path itself included, holding a `.git` entry of any type).
  - `Outcome::FailedReport { error: ErrorDocument, human: String }`: without `--json` the report goes to
    stderr ahead of the `vpt: <message>` line; with `--json` only the error document is printed.
  - `vpt::doctor::run(environment: &Environment, config: Option<&Path>) -> Outcome` and
    `vpt::doctor::checks::all(environment: &Environment, config: Option<&Path>) -> Vec<Check>`.
  - `INSTALL_HINT: &str` naming the two install commands of spec section 13.

- [ ] **Step 1: Write the failing tests**

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
        assert_eq!(emit(outcome, true), 3);
    }
```

`crates/vpt/tests/doctor.rs`:

```rust
mod support;

use support::{Sandbox, run, stderr, stdout};

fn checks(document: &serde_json::Value, key: &str) -> Vec<(String, bool, String)> {
    document[key]
        .as_array()
        .expect("checks array")
        .iter()
        .map(|c| (c["name"].as_str().unwrap().to_owned(), c["ok"].as_bool().unwrap(), c["detail"].as_str().unwrap().to_owned()))
        .collect()
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
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-adapters git_tree && cargo test -p vpt --features dev-tools output --test doctor`
Expected: compile errors naming `enclosing_git_tree` and `FailedReport`; the doctor tests FAIL with exit
2\.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-adapters/src/git_tree.rs`:

```rust
//! Whether a path sits inside a git working tree: a walk up for a `.git` entry.

use std::path::{Path, PathBuf};

pub fn enclosing_git_tree(path: &Path) -> Option<PathBuf> {
    path.ancestors().find(|ancestor| ancestor.join(".git").symlink_metadata().is_ok()).map(Path::to_path_buf)
}
```

Add `pub mod git_tree;` to the adapters `lib.rs`. In `crates/vpt/src/cli/output.rs` the enum and `emit`
become:

```rust
pub enum Outcome {
    Success { document: serde_json::Value, human: String },
    Failure(ErrorDocument),
    FailedReport { error: ErrorDocument, human: String },
}

pub fn emit(outcome: Outcome, json: bool) -> i32 {
    match outcome {
        Outcome::Success { document, human } => {
            let text = if json { format!("{document}\n") } else { human };
            let _ = std::io::stdout().write_all(text.as_bytes());
            0
        }
        Outcome::Failure(error) => failure(&error, json, ""),
        Outcome::FailedReport { error, human } => failure(&error, json, &human),
    }
}

fn failure(error: &ErrorDocument, json: bool, report: &str) -> i32 {
    let text = if json { format!("{}\n", error.to_json()) } else { format!("{report}vpt: {}\n", error.message) };
    let _ = std::io::stderr().write_all(text.as_bytes());
    error.exit_code()
}
```

`crates/vpt/src/doctor/mod.rs`:

```rust
//! `vpt doctor`: every check, always, then one verdict.

pub mod checks;

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
//! One function per check. Each returns a `Check` and none may panic.

use super::INSTALL_HINT;
use crate::compose::{Environment, settings_from};
use std::path::Path;
use vpt_adapters::archive::ClonefileArchive;
use vpt_adapters::config::roots::{Creation, Roots, resolve};
use vpt_adapters::git_tree::enclosing_git_tree;
use vpt_adapters::helper::{BUILT_AGAINST_MAJOR, HelperClient, HelperError};
use vpt_adapters::ledger::sqlite::SqliteLedger;
use vpt_adapters::symlink::{LinkState, inspect, verify};
use vpt_adapters::voice_memos::store::VoiceMemosStore;
use vpt_application::ports::archive::Archive;
use vpt_application::ports::ledger::RecordingLedger;
use vpt_application::ports::recorder::RecorderStore;
use vpt_application::settings::Settings;
use vpt_domain::layout::StoreKey;
use vpt_protocol::error::Check;

const NOT_RUN: &str = "not run: the configuration did not load";

fn check(name: impl Into<String>, ok: bool, detail: impl Into<String>) -> Check {
    Check { name: name.into(), ok, detail: detail.into() }
}

pub fn all(environment: &Environment, config: Option<&Path>) -> Vec<Check> {
    let settings = match settings_from(environment, config) {
        Ok(settings) => settings,
        Err(error) => return not_run(check("config", false, error.message)),
    };
    let mut checks = vec![check("config", true, format!("config_version {}", settings.config_version))];
    let roots = match resolve(&settings, Creation::None) {
        Ok(roots) => roots,
        Err(error) => {
            checks.push(check("stores", false, format!("{error:?}")));
            checks.push(helper(&settings));
            return checks;
        }
    };
    checks.push(check("stores", true, format!("home {}", roots.home.display())));
    checks.extend(StoreKey::all().into_iter().map(|key| git_tree(key, &roots)));
    checks.push(helper(&settings));
    let store = VoiceMemosStore::new(roots.recordings_dir.clone(), roots.state_dir.clone(), false);
    checks.push(recordings_dir(&store));
    checks.push(symlink(&settings));
    checks.extend(store.subdirectory_counts().into_iter().map(|(name, count)| {
        check(format!("subdirectory:{name}"), true, count.map_or("absent".to_owned(), |n| format!("{n} entries")))
    }));
    checks.push(cleanup_pending(&roots));
    checks.push(source_gone(&roots));
    checks
}

fn not_run(config: Check) -> Vec<Check> {
    let mut checks = vec![config];
    for name in ["stores", "helper", "recordings_dir", "symlink", "cleanup_pending", "source_gone"] {
        checks.push(check(name, false, NOT_RUN));
    }
    checks
}

fn git_tree(key: StoreKey, roots: &Roots) -> Check {
    let name = format!("git_tree:{}", key.key_name());
    match enclosing_git_tree(roots.stores.get(key)) {
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

fn helper(settings: &Settings) -> Check {
    match HelperClient::new(settings.helper_path.clone()).version() {
        Ok(version) => check("helper", true, format!("vpt-macos {version} at {}", settings.helper_path.display())),
        Err(HelperError::Absent) => check("helper", false, format!("{} not found; {INSTALL_HINT}", settings.helper_path.display())),
        Err(HelperError::MajorMismatch { found }) => check("helper", false, format!("major version {found}, this build needs {BUILT_AGAINST_MAJOR}")),
        Err(error) => check("helper", false, format!("{error:?}")),
    }
}

fn recordings_dir(store: &VoiceMemosStore) -> Check {
    match store.candidates() {
        Ok(candidates) => check("recordings_dir", true, format!("readable, {} candidates", candidates.len())),
        Err(error) => check("recordings_dir", false, format!("{error:?}")),
    }
}

fn symlink(settings: &Settings) -> Check {
    match &settings.symlink_target {
        Some(target) => match verify(&settings.home, target) {
            Ok(()) => check("symlink", true, format!("{} -> {}", settings.home.display(), target.display())),
            Err(error) => check("symlink", false, format!("{error:?}")),
        },
        None => match inspect(&settings.home) {
            LinkState::LinkTo(target) => check("symlink", true, format!("followed, not managed: {} -> {}", settings.home.display(), target.display())),
            _ => check("symlink", true, "not configured"),
        },
    }
}

fn cleanup_pending(roots: &Roots) -> Check {
    match ClonefileArchive::new(roots.stores.audio.clone()).staged_leftovers() {
        Ok(leftovers) if leftovers.is_empty() => check("cleanup_pending", true, "none"),
        Ok(leftovers) => {
            let names: Vec<String> = leftovers.iter().map(|p| p.display().to_string()).collect();
            check("cleanup_pending", false, format!("{} staged files await the Trash: {}", names.len(), names.join(", ")))
        }
        Err(error) => check("cleanup_pending", false, format!("{error:?}")),
    }
}

fn source_gone(roots: &Roots) -> Check {
    if !roots.state_dir.join("vpt.db").exists() {
        return check("source_gone", true, "no ledger yet");
    }
    match SqliteLedger::open(&roots.state_dir).and_then(|ledger| ledger.seen_all()) {
        Ok(rows) => {
            let gone = rows.iter().filter(|row| row.source_gone_at.is_some()).count();
            check("source_gone", true, format!("{gone} recordings whose source is gone"))
        }
        Err(error) => check("source_gone", false, format!("{error:?}")),
    }
}
```

The `stores` failure path returns early after the helper check because the remaining checks need resolved
roots; that early return keeps the list honest rather than reporting roots that do not exist. The doctor
opens the ledger only when the database already exists, so a machine that has never ingested gains no
state directory from a diagnosis.

Dispatch: `Verb::Doctor => doctor::run(&environment, invocation.config.as_deref()),` with
`pub mod doctor;` in `lib.rs`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --workspace --features dev-tools`

Expected: all PASS. `checks.rs` must stay under 200 implementation lines; it is about 120.

- [ ] **Step 5: Commit**

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
- Modify: `crates/vpt-adapters/src/ledger/sqlite/mod.rs`, `crates/vpt-adapters/src/ledger/memory.rs`,
  `crates/vpt-adapters/src/ledger/contract.rs`
- Test: the contract invocations in both ledgers' test modules gain the new suite

**Interfaces:**

- Consumes: `Hold`, `FileTime::age_secs`, `LedgerError`, the `retention_intents` table of Task 11.
- Produces:
  - `vpt_domain::retention::expired(mtime: FileTime, hold: Hold, now: UtcInstant) -> bool`.
  - `vpt_application::ports::retention::{ArtifactKind::Audio,`
    `NewIntent { pub kind: ArtifactKind, pub recording: RecordingId, pub path: PathBuf,`
    `pub expected: Sha256Digest, pub recorded_at: UtcInstant },`
    `RetentionIntent { pub id: i64, ...the same fields }, RetentionJournal}` with
    `fn record_intent(&self, intent: &NewIntent) -> Result<i64, LedgerError>`,
    `fn pending_intents(&self) -> Result<Vec<RetentionIntent>, LedgerError>`,
    `fn complete_intent(&self, id: i64, at: UtcInstant) -> Result<(), LedgerError>`.
  - `retention_journal_contract!` and the two implementations.

The `expected` column is text so a later stage can record a note's identity and stage there; stage 1
records the digest hex, the only ownership proof an audio clone has.

- [ ] **Step 1: Write the failing tests**

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
}
```

`crates/vpt-adapters/src/ledger/contract.rs`, appended:

```rust
pub mod retention_scenarios {
    use super::digest;
    use std::path::PathBuf;
    use vpt_application::ports::retention::{ArtifactKind, NewIntent, RetentionJournal};
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

    pub fn a_recorded_intent_is_pending_with_its_fields(journal: &dyn RetentionJournal) {
        let id = journal.record_intent(&intent(1)).expect("recorded");
        let pending = journal.pending_intents().expect("pending");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, id);
        assert_eq!(pending[0].path, PathBuf::from("/store/1.m4a"));
        assert_eq!(pending[0].expected, digest(1));
        assert_eq!(pending[0].kind, ArtifactKind::Audio);
    }

    pub fn completing_an_intent_removes_it_and_keeps_the_others_in_order(journal: &dyn RetentionJournal) {
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
            use crate::ledger::contract::retention_scenarios::*;

            #[test]
            fn a_recorded_intent_is_pending_with_its_fields_() {
                let (_guard, journal) = $make();
                a_recorded_intent_is_pending_with_its_fields(&*journal);
            }
            #[test]
            fn completing_an_intent_removes_it_and_keeps_the_others_in_order_() {
                let (_guard, journal) = $make();
                completing_an_intent_removes_it_and_keeps_the_others_in_order(&*journal);
            }
        }
    };
}
pub(crate) use retention_journal_contract;
```

and, in the `tests` modules of `sqlite/mod.rs` and `memory.rs`, beside the two earlier invocations:

```rust
    crate::ledger::contract::retention_journal_contract!(|| {
        let temp = tempfile::tempdir().expect("temp");
        let ledger = SqliteLedger::open(temp.path()).expect("opens");
        (temp, Box::new(ledger) as Box<dyn vpt_application::ports::retention::RetentionJournal>)
    });
```

```rust
    crate::ledger::contract::retention_journal_contract!(|| {
        ((), Box::new(MemoryLedger::new()) as Box<dyn vpt_application::ports::retention::RetentionJournal>)
    });
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-domain retention && cargo test -p vpt-adapters ledger`

Expected: compile errors naming `expired`, `NewIntent`, `RetentionJournal`.

- [ ] **Step 3: Write the minimal implementation**

Append to `crates/vpt-domain/src/retention.rs`:

```rust
use crate::time::{FileTime, UtcInstant};

/// Expired when the hold is set and the file's age has reached it.
pub fn expired(mtime: FileTime, hold: Hold, now: UtcInstant) -> bool {
    hold.seconds() != 0 && mtime.age_secs(now) >= hold.seconds() as i64
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

`ports/mod.rs` adds `pub mod retention;`. `crates/vpt-adapters/src/ledger/sqlite/retention.rs`:

```rust
//! `retention_intents` behind the journal port.

use super::{SqliteLedger, map};
use rusqlite::params;
use std::path::PathBuf;
use vpt_application::ports::ledger::LedgerError;
use vpt_application::ports::retention::{ArtifactKind, NewIntent, RetentionIntent, RetentionJournal};
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

`State` gains `intents: Vec<(RetentionIntent, Option<UtcInstant>)>`. With all three implementations in
it, `memory.rs` stays under 300 lines.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --workspace --features dev-tools`

Expected: all PASS, four new contract tests among them.

- [ ] **Step 5: Commit**

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(retention): the expiry rule and the intents journal under one contract"
```

______________________________________________________________________

### Task 32: `vpt retention run`

**Files:**

- Create: `crates/vpt-application/src/retention/mod.rs`,
  `crates/vpt-application/src/retention/reconcile.rs`
- Modify: `crates/vpt-application/src/lib.rs`
- Create: `crates/vpt-adapters/tests/retention_reconcile.rs`
- Modify: `crates/vpt/src/compose.rs`
- Create: `crates/vpt/src/commands/retention.rs`
- Modify: `crates/vpt/src/commands/mod.rs`, `crates/vpt/src/lib.rs`
- Test: `crates/vpt/tests/retention.rs`; `crates/vpt/tests/support/mod.rs` gains `set_mtime`

**Interfaces:**

- Consumes: `expired`, `RetentionJournal`, `RecordingLedger::{recordings, set_audio_trashed}`,
  `Stores::entries`, `ArtifactFiles::digest_of`, `Trash`, `Clock`, `Notifier`, `RetentionSettings`,
  `StorePaths`, `HelperClient::version`, `Runtime::mutating`.

- Produces:

  - `vpt_application::retention::{Retention<'a> { pub ledger: &'a dyn RecordingLedger,`
    `pub journal: &'a dyn RetentionJournal, pub stores: &'a dyn Stores,`
    `pub files: &'a dyn ArtifactFiles, pub trash: &'a dyn Trash, pub clock: &'a dyn Clock,`
    `pub notifier: &'a dyn Notifier, pub settings: &'a RetentionSettings,`
    `pub paths: &'a StorePaths }, Moved { pub store: StoreKey, pub path: PathBuf },`
    `Kept { pub path: PathBuf, pub reason: KeptReason }, KeptReason::{Untracked,`
    `DigestMismatch}, RetentionReport { pub moved: Vec<Moved>, pub kept: Vec<Kept> },`
    `RetentionError::{Disabled, TargetModified(PathBuf), Ledger(LedgerError), Files(String),`
    `Trash(TrashError)}, RetentionFailure { pub error: RetentionError,` `pub completed: Vec<PathBuf> }}`
    with `Retention::run(&self, dry_run: bool) -> Result<RetentionReport, RetentionFailure>`.
  - `vpt_application::retention::reconcile::reconcile_intents(journal, ledger, files, trash,`
    `clock) -> Result<Vec<PathBuf>, RetentionError>` (the paths moved again), called by
    `Runtime::mutating` after publication repair.
  - `vpt::commands::retention::run(runtime: &Runtime, dry_run: bool) -> Outcome`.
  - `Sandbox::set_mtime(&self, path: &Path, seconds_ago: u64)`.

- [ ] **Step 1: Write the failing tests**

`crates/vpt-adapters/tests/retention_reconcile.rs` (the intent matrix, over the in-memory ledger, a
temporary store, and the recording trash and fixed clock of `tests/support/mod.rs`):

```rust
mod support;

use std::path::PathBuf;
use support::{RecordingTrash, clock};
use vpt_adapters::ledger::memory::MemoryLedger;
use vpt_adapters::stores::{FilesystemStores, digest_file};
use vpt_application::ports::ledger::{RecordingLedger, RecordingRecord, StageStates, TitleOrigin};
use vpt_application::ports::retention::{ArtifactKind, NewIntent, RetentionJournal};
use vpt_application::retention::RetentionError;
use vpt_application::retention::reconcile::reconcile_intents;
use vpt_domain::digest::Sha256Digest;
use vpt_domain::identity::RecordingId;
use vpt_domain::time::{UtcInstant, UtcOffset};

struct World {
    dir: tempfile::TempDir,
    ledger: MemoryLedger,
    trash: RecordingTrash,
}

fn world() -> World {
    let dir = tempfile::tempdir().expect("dir");
    std::fs::create_dir(dir.path().join("trash")).expect("trash");
    World { trash: RecordingTrash::new(dir.path().join("trash")), ledger: MemoryLedger::new(), dir }
}

fn recording(audio_path: PathBuf) -> RecordingRecord {
    let captured_at = UtcInstant { secs: 1_787_690_856 };
    let captured_offset = UtcOffset { secs: -21_600 };
    let digest = Sha256Digest([7; 32]);
    RecordingRecord {
        id: RecordingId::derive(captured_at, captured_offset, &digest),
        source_path: None,
        digest,
        captured_at,
        captured_offset,
        duration_secs: 3,
        title: None,
        title_source: TitleOrigin::Unavailable,
        ingested_at: captured_at,
        audio_path,
        stages: StageStates::fresh(),
        audio_trashed_at: None,
    }
}

fn intent(world: &World, name: &str, expected: Sha256Digest) -> (PathBuf, RecordingId, i64) {
    let recording = recording(world.dir.path().join(name));
    world.ledger.commit_recovered(&recording).expect("recorded");
    let id = world
        .ledger
        .record_intent(&NewIntent { kind: ArtifactKind::Audio, recording: recording.id.clone(), path: recording.audio_path.clone(), expected, recorded_at: UtcInstant { secs: 1 } })
        .expect("intent");
    (recording.audio_path, recording.id, id)
}

#[test]
fn an_absent_path_completes_the_expiration_without_the_trash() {
    let world = world();
    let (path, recording, _) = intent(&world, "gone.m4a", Sha256Digest([1; 32]));

    let moved = reconcile_intents(&world.ledger, &world.ledger, &FilesystemStores, &world.trash, &clock()).expect("reconciled");

    assert!(moved.is_empty());
    assert!(world.ledger.pending_intents().expect("pending").is_empty());
    assert!(world.ledger.by_id(&recording).expect("read").expect("row").audio_trashed_at.is_some());
    assert!(!path.exists());
}

#[test]
fn an_unchanged_original_is_moved_again_and_completed() {
    let world = world();
    let path = world.dir.path().join("still.m4a");
    std::fs::write(&path, b"clone").expect("clone");
    intent(&world, "still.m4a", digest_file(&path).expect("digest"));

    let moved = reconcile_intents(&world.ledger, &world.ledger, &FilesystemStores, &world.trash, &clock()).expect("reconciled");

    assert_eq!(moved, vec![path.clone()]);
    assert!(!path.exists());
    assert!(world.dir.path().join("trash/still.m4a").exists());
    assert!(world.ledger.pending_intents().expect("pending").is_empty());
}

#[test]
fn replaced_content_is_a_refusal_naming_the_path_and_stays_pending() {
    let world = world();
    let path = world.dir.path().join("changed.m4a");
    std::fs::write(&path, b"someone else's bytes").expect("replacement");
    intent(&world, "changed.m4a", Sha256Digest([9; 32]));

    let outcome = reconcile_intents(&world.ledger, &world.ledger, &FilesystemStores, &world.trash, &clock());

    assert_eq!(outcome, Err(RetentionError::TargetModified(path.clone())));
    assert!(path.exists());
    assert_eq!(world.ledger.pending_intents().expect("pending").len(), 1);
}
```

`crates/vpt/tests/retention.rs`:

```rust
mod support;

use support::{Sandbox, run, stderr, stdout};
use vpt_domain::fixtures::m4a;

const CAPTURED: i64 = 1_787_690_856;
const ENABLED: &str = "[retention]\nenabled = true\ninclude_audio = true\n[retention.hold]\naudio = \"1d\"\n";

fn ingested(name: &str, extra: &str) -> (Sandbox, String, std::path::PathBuf) {
    let sandbox = Sandbox::new(name);
    sandbox.install_fake_helper();
    sandbox.write_config(extra);
    sandbox.add_recording("a.m4a", &m4a(CAPTURED, 3, b"audio"));
    let output = run(sandbox.vpt().args(["ingest", "--json"]));
    let document: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("json");
    let id = document["ingested"][0]["id"].as_str().expect("id").to_owned();
    let clone = sandbox.path().join(format!("home/.vpt/audio/{id}.m4a"));
    (sandbox, id, clone)
}

#[test]
fn an_expired_clone_moves_to_the_trash_through_the_helper_and_the_ledger_records_it() {
    let (sandbox, id, clone) = ingested("retention-move", ENABLED);
    sandbox.set_mtime(&clone, 2 * 86_400);

    let output = run(sandbox.vpt().args(["retention", "run", "--json"]));

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let document: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("json");
    assert_eq!(document["moved"][0]["store"], "audio");
    assert_eq!(document["moved"][0]["path"], clone.to_string_lossy());
    assert!(!clone.exists());
    assert!(sandbox.path().join(format!("trash/{id}.m4a")).exists());
    let shown = run(sandbox.vpt().args(["show", &id, "--json"]));
    let record: serde_json::Value = serde_json::from_str(&stdout(&shown)).expect("json");
    assert!(record["audio_trashed_at"].is_string());
    let intents: i64 = sandbox.ledger().query_row("SELECT COUNT(*) FROM retention_intents WHERE completed_at IS NOT NULL", [], |r| r.get(0)).expect("count");
    assert_eq!(intents, 1);
}

#[test]
fn an_untracked_file_in_a_store_survives_and_is_reported_kept() {
    let (sandbox, _id, _clone) = ingested("retention-untracked", ENABLED);
    let stray = sandbox.path().join("home/.vpt/audio/stray.m4a");
    std::fs::write(&stray, b"not vpt's").expect("stray");
    sandbox.set_mtime(&stray, 30 * 86_400);

    let output = run(sandbox.vpt().args(["retention", "run", "--json"]));

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let document: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("json");
    assert!(stray.exists());
    assert_eq!(document["kept"][0]["path"], stray.to_string_lossy());
    assert_eq!(document["kept"][0]["reason"], "untracked");
    assert_eq!(document["moved"], serde_json::json!([]));
}

#[test]
fn audio_is_excluded_unless_include_audio_is_set() {
    let (sandbox, _id, clone) = ingested("retention-exclude", "[retention]\nenabled = true\n[retention.hold]\naudio = \"1d\"\n");
    sandbox.set_mtime(&clone, 2 * 86_400);

    let output = run(sandbox.vpt().args(["retention", "run", "--json"]));

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(clone.exists());
    let document: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("json");
    assert_eq!(document["moved"], serde_json::json!([]));
}

#[test]
fn dry_run_lists_what_would_move_and_moves_nothing() {
    let (sandbox, _id, clone) = ingested("retention-dry", ENABLED);
    sandbox.set_mtime(&clone, 2 * 86_400);

    let output = run(sandbox.vpt().args(["retention", "run", "--dry-run", "--json"]));

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let document: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("json");
    assert_eq!(document["moved"][0]["path"], clone.to_string_lossy());
    assert!(clone.exists());
    let intents: i64 = sandbox.ledger().query_row("SELECT COUNT(*) FROM retention_intents", [], |r| r.get(0)).expect("count");
    assert_eq!(intents, 0);
    assert!(!sandbox.fake_log().exists());
}

#[test]
fn disabled_is_exit_2_and_an_absent_or_mismatched_helper_is_refused() {
    let (sandbox, _id, _clone) = ingested("retention-refusals", "");

    let disabled = run(sandbox.vpt().args(["retention", "run"]));
    assert_eq!(disabled.status.code(), Some(2));
    assert!(stderr(&disabled).contains("retention.enabled"), "{}", stderr(&disabled));

    let (sandbox, _id, _clone) = ingested("retention-no-helper", ENABLED);
    let mismatch = run(sandbox.vpt().args(["retention", "run", "--json"]).env("VPT_FAKE_VERSION", "2.0.0"));
    assert_eq!(mismatch.status.code(), Some(3));
    let error: serde_json::Value = serde_json::from_str(&stderr(&mismatch)).expect("json");
    assert_eq!(error["error"]["rule"], "helper_version");
    std::fs::remove_file(sandbox.path().join("bin/vpt-macos")).expect("unlink the fake's symlink");
    let absent = run(sandbox.vpt().args(["retention", "run", "--json"]));
    assert_eq!(absent.status.code(), Some(3));
    let error: serde_json::Value = serde_json::from_str(&stderr(&absent)).expect("json");
    assert_eq!(error["error"]["rule"], "no_trash");
}

#[test]
fn a_pending_intent_whose_target_changed_refuses_the_next_mutating_command() {
    let (sandbox, id, clone) = ingested("retention-modified", ENABLED);
    sandbox
        .ledger()
        .execute(
            "INSERT INTO retention_intents (artifact_kind, recording, path, expected, recorded_at) VALUES ('audio', ?1, ?2, ?3, 1)",
            [id.as_str(), clone.to_string_lossy().as_ref(), &"ab".repeat(32)],
        )
        .expect("seed");

    let output = run(sandbox.vpt().args(["ingest", "--json"]));

    assert_eq!(output.status.code(), Some(3));
    let error: serde_json::Value = serde_json::from_str(&stderr(&output)).expect("json");
    assert_eq!(error["error"]["rule"], "retention_target_modified");
    assert!(clone.exists());
}
```

`std::fs::remove_file` on the sandbox's own symlink is the test removing a link it created inside its
temporary directory, the one place this plan unlinks anything. Support addition:

```rust
impl Sandbox {
    pub fn set_mtime(&self, path: &Path, seconds_ago: u64) {
        let file = std::fs::File::options().write(true).open(path).expect("open for mtime");
        let when = std::time::SystemTime::now() - std::time::Duration::from_secs(seconds_ago);
        file.set_times(std::fs::FileTimes::new().set_modified(when)).expect("mtime");
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vpt-adapters --test retention_reconcile &&`
`cargo test -p vpt --features dev-tools --test retention`

Expected: compile errors naming `reconcile_intents` and `RetentionError`; the command tests FAIL with
exit 2.

- [ ] **Step 3: Write the minimal implementation**

`crates/vpt-application/src/retention/reconcile.rs`:

```rust
//! Pending intents, settled before any other mutation: absent completes,
//! unchanged moves again, replaced refuses.

use super::RetentionError;
use crate::ports::artifacts::ArtifactFiles;
use crate::ports::clock::Clock;
use crate::ports::ledger::RecordingLedger;
use crate::ports::retention::RetentionJournal;
use crate::ports::trash::Trash;
use std::path::PathBuf;

pub fn reconcile_intents(
    journal: &dyn RetentionJournal,
    ledger: &dyn RecordingLedger,
    files: &dyn ArtifactFiles,
    trash: &dyn Trash,
    clock: &dyn Clock,
) -> Result<Vec<PathBuf>, RetentionError> {
    let mut moved = Vec::new();
    for intent in journal.pending_intents().map_err(RetentionError::Ledger)? {
        match files.digest_of(&intent.path).map_err(|error| RetentionError::Files(error.0))? {
            None => {}
            Some(digest) if digest == intent.expected => {
                trash.trash(&intent.path).map_err(RetentionError::Trash)?;
                moved.push(intent.path.clone());
            }
            Some(_) => return Err(RetentionError::TargetModified(intent.path)),
        }
        let now = clock.now();
        ledger.set_audio_trashed(&intent.recording, now).map_err(RetentionError::Ledger)?;
        journal.complete_intent(intent.id, now).map_err(RetentionError::Ledger)?;
    }
    Ok(moved)
}
```

`crates/vpt-application/src/retention/mod.rs`:

```rust
//! `vpt retention run`: the expired artifacts the ledger owns, moved to the
//! Trash one intent at a time.

pub mod reconcile;

use crate::ports::artifacts::ArtifactFiles;
use crate::ports::clock::Clock;
use crate::ports::ledger::{LedgerError, RecordingLedger, RecordingRecord};
use crate::ports::notifier::Notifier;
use crate::ports::retention::{ArtifactKind, NewIntent, RetentionJournal};
use crate::ports::stores::{StoreEntry, Stores};
use crate::ports::trash::{Trash, TrashError};
use crate::settings::{RetentionSettings, StorePaths};
use std::path::PathBuf;
use vpt_domain::layout::StoreKey;
use vpt_domain::notification::{EventKind, Notification};
use vpt_domain::retention::expired;

pub struct Retention<'a> {
    pub ledger: &'a dyn RecordingLedger,
    pub journal: &'a dyn RetentionJournal,
    pub stores: &'a dyn Stores,
    pub files: &'a dyn ArtifactFiles,
    pub trash: &'a dyn Trash,
    pub clock: &'a dyn Clock,
    pub notifier: &'a dyn Notifier,
    pub settings: &'a RetentionSettings,
    pub paths: &'a StorePaths,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Moved {
    pub store: StoreKey,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeptReason {
    Untracked,
    DigestMismatch,
}

impl KeptReason {
    pub fn as_str(self) -> &'static str {
        match self {
            KeptReason::Untracked => "untracked",
            KeptReason::DigestMismatch => "digest_mismatch",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kept {
    pub path: PathBuf,
    pub reason: KeptReason,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RetentionReport {
    pub moved: Vec<Moved>,
    pub kept: Vec<Kept>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetentionError {
    Disabled,
    TargetModified(PathBuf),
    Ledger(LedgerError),
    Files(String),
    Trash(TrashError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionFailure {
    pub error: RetentionError,
    pub completed: Vec<PathBuf>,
}

impl Retention<'_> {
    pub fn run(&self, dry_run: bool) -> Result<RetentionReport, RetentionFailure> {
        if !self.settings.enabled {
            return Err(RetentionFailure { error: RetentionError::Disabled, completed: vec![] });
        }
        let mut report = RetentionReport::default();
        let owned = self.ledger.recordings().map_err(|error| RetentionFailure { error: RetentionError::Ledger(error), completed: vec![] })?;
        for key in StoreKey::all() {
            if key == StoreKey::Audio && !self.settings.include_audio {
                continue;
            }
            let hold = self.settings.hold(key);
            if hold.seconds() == 0 {
                continue;
            }
            let entries = self.stores.entries(self.paths.get(key)).map_err(|error| self.failure(RetentionError::Files(error.0), &report))?;
            for entry in entries {
                let Some(record) = owned.iter().find(|record| record.audio_path == entry.path && record.audio_trashed_at.is_none()) else {
                    report.kept.push(Kept { path: entry.path, reason: KeptReason::Untracked });
                    continue;
                };
                if !expired(entry.mtime, hold, self.clock.now()) {
                    continue;
                }
                self.expire(key, &entry, record, dry_run, &mut report)?;
            }
        }
        if !dry_run && !report.moved.is_empty() {
            self.notifier.deliver(&self.notification(&report));
        }
        Ok(report)
    }

    fn expire(&self, key: StoreKey, entry: &StoreEntry, record: &RecordingRecord, dry_run: bool, report: &mut RetentionReport) -> Result<(), RetentionFailure> {
        let fail = |error| self.failure(error, report);
        if self.files.digest_of(&entry.path).map_err(|error| fail(RetentionError::Files(error.0)))? != Some(record.digest) {
            report.kept.push(Kept { path: entry.path.clone(), reason: KeptReason::DigestMismatch });
            return Ok(());
        }
        if !dry_run {
            let now = self.clock.now();
            let intent = NewIntent { kind: ArtifactKind::Audio, recording: record.id.clone(), path: entry.path.clone(), expected: record.digest, recorded_at: now };
            let id = self.journal.record_intent(&intent).map_err(|error| fail(RetentionError::Ledger(error)))?;
            self.trash.trash(&entry.path).map_err(|error| fail(RetentionError::Trash(error)))?;
            self.ledger.set_audio_trashed(&record.id, now).map_err(|error| fail(RetentionError::Ledger(error)))?;
            self.journal.complete_intent(id, now).map_err(|error| fail(RetentionError::Ledger(error)))?;
        }
        report.moved.push(Moved { store: key, path: entry.path.clone() });
        Ok(())
    }

    fn failure(&self, error: RetentionError, report: &RetentionReport) -> RetentionFailure {
        RetentionFailure { error, completed: report.moved.iter().map(|moved| moved.path.clone()).collect() }
    }

    fn notification(&self, report: &RetentionReport) -> Notification {
        let mut counts: Vec<(String, u64)> = Vec::new();
        for moved in &report.moved {
            match counts.iter_mut().find(|(store, _)| store == moved.store.key_name()) {
                Some((_, count)) => *count += 1,
                None => counts.push((moved.store.key_name().to_owned(), 1)),
            }
        }
        Notification::done(EventKind::Retention, format!("{} artifacts moved to the Trash", report.moved.len()), counts, self.clock.now())
    }
}
```

The closure `fail` borrows `report` immutably while `expire` later pushes into it; write `fail` as a
small method call at each site instead, `self.failure(RetentionError::Files(error.0), report)`, so the
borrow checker is satisfied. The listing above shows the intent; the committed file uses the method form
at every `map_err`. Add `pub mod retention;` to the application `lib.rs`.

`Runtime::mutating` in `crates/vpt/src/compose.rs` gains, after `repair_publications`:

```rust
        reconcile_intents(&self.ledger, &self.ledger, &FilesystemStores, self.trash(), &self.clock).map_err(retention_error)?;
```

with

```rust
pub fn retention_error(error: RetentionError) -> ErrorDocument {
    match error {
        RetentionError::Disabled => ErrorDocument::new(ErrorKind::Usage, "retention.enabled is false"),
        RetentionError::TargetModified(path) => {
            ErrorDocument::new(ErrorKind::Refused, format!("{} changed since its retention intent was recorded", path.display())).rule("retention_target_modified")
        }
        RetentionError::Ledger(error) => ErrorDocument::new(ErrorKind::Ledger, format!("{error:?}")),
        RetentionError::Files(detail) => ErrorDocument::new(ErrorKind::Store, detail),
        RetentionError::Trash(TrashError::HelperAbsent) => ErrorDocument::new(ErrorKind::Refused, "the helper is absent, so nothing can reach the Trash").rule("no_trash"),
        RetentionError::Trash(error) => ErrorDocument::new(ErrorKind::Helper, format!("{error:?}")),
    }
}
```

`crates/vpt/src/commands/retention.rs`:

```rust
//! `vpt retention run [--dry-run]`.

use crate::cli::output::Outcome;
use crate::compose::{Runtime, retention_error};
use serde_json::json;
use vpt_adapters::helper::{BUILT_AGAINST_MAJOR, HelperError};
use vpt_adapters::stores::FilesystemStores;
use vpt_application::retention::{Retention, RetentionReport};
use vpt_protocol::error::{ErrorDocument, ErrorKind};
use vpt_protocol::result::document;

pub fn run(runtime: &Runtime, dry_run: bool) -> Outcome {
    if !runtime.settings.retention.enabled {
        return Outcome::Failure(ErrorDocument::new(ErrorKind::Usage, "retention.enabled is false"));
    }
    match runtime.helper.version() {
        Err(HelperError::Absent) => {
            return Outcome::Failure(ErrorDocument::new(ErrorKind::Refused, "the helper is absent, so nothing can reach the Trash").rule("no_trash"));
        }
        Err(HelperError::MajorMismatch { found }) => {
            let message = format!("the helper is major version {found}; this build needs {BUILT_AGAINST_MAJOR}");
            return Outcome::Failure(ErrorDocument::new(ErrorKind::Refused, message).rule("helper_version"));
        }
        _ => {}
    }
    let lock = if dry_run { None } else { match runtime.mutating() { Ok(lock) => Some(lock), Err(error) => return Outcome::Failure(error) } };
    let retention = Retention {
        ledger: &runtime.ledger,
        journal: &runtime.ledger,
        stores: &FilesystemStores,
        files: &FilesystemStores,
        trash: runtime.trash(),
        clock: &runtime.clock,
        notifier: runtime.notifier.as_ref(),
        settings: &runtime.settings.retention,
        paths: &runtime.roots.stores,
    };
    let outcome = match retention.run(dry_run) {
        Ok(report) => Outcome::Success { human: human(&report, dry_run), document: document("retention run", body(&report)) },
        Err(failure) => Outcome::Failure(retention_error(failure.error).completed(failure.completed.iter().map(|p| p.display().to_string()).collect())),
    };
    drop(lock);
    outcome
}

fn body(report: &RetentionReport) -> serde_json::Value {
    json!({
        "moved": report.moved.iter().map(|m| json!({"store": m.store.key_name(), "path": m.path})).collect::<Vec<_>>(),
        "kept": report.kept.iter().map(|k| json!({"path": k.path, "reason": k.reason.as_str()})).collect::<Vec<_>>(),
    })
}

fn human(report: &RetentionReport, dry_run: bool) -> String {
    let verb = if dry_run { "would move" } else { "moved" };
    let mut lines: Vec<String> = report.moved.iter().map(|m| format!("{verb} {} ({})", m.path.display(), m.store.key_name())).collect();
    lines.extend(report.kept.iter().map(|k| format!("kept {} ({})", k.path.display(), k.reason.as_str())));
    lines.push(format!("{verb} {}, kept {}", report.moved.len(), report.kept.len()));
    lines.join("\n") + "\n"
}
```

The helper is probed before the lock so an absent or mismatched helper is refused with nothing else
touched, dry run included. Dispatch:
`Verb::RetentionRun { dry_run } => with_runtime(&environment, invocation,`
`Creation::None, |runtime| commands::retention::run(runtime, *dry_run)),` and `commands/mod.rs` gains
`retention`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --workspace --features dev-tools`

Expected: all PASS, the ingest suites of Task 27 included, since `mutating` now reconciles intents too.

- [ ] **Step 5: Commit**

```bash
git add crates
SKIP_AI_COMMIT=1 git commit -m "feat(retention): vpt retention run with journaled moves to the Trash"
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
        .testTarget(name: "VptMacosTests", dependencies: ["VptMacos"]),
    ]
)
```

`helper/vpt-macos/Tests/VptMacosTests/ArgumentsTests.swift`:

```swift
import XCTest
@testable import VptMacos

final class ArgumentsTests: XCTestCase {
    func testVersionNotifyAndTrashParse() {
        XCTAssertEqual(parse(["--version"]), .success(.version))
        XCTAssertEqual(parse(["notify", "--title", "t", "--body", "b"]), .success(.notify(title: "t", body: "b")))
        XCTAssertEqual(parse(["notify", "--body", "b", "--title", "t"]), .success(.notify(title: "t", body: "b")))
        XCTAssertEqual(parse(["trash", "/tmp/x"]), .success(.trash(path: "/tmp/x")))
    }

    func testUnknownAndIncompleteArgumentsAreUsageErrors() {
        XCTAssertEqual(parse([]), .failure(UsageError(message: "unknown subcommand")))
        XCTAssertEqual(parse(["transcribe", "x"]), .failure(UsageError(message: "unknown subcommand")))
        XCTAssertEqual(parse(["--version", "extra"]), .failure(UsageError(message: "--version takes no arguments")))
        XCTAssertEqual(parse(["notify", "--title"]), .failure(UsageError(message: "--title needs a value")))
        XCTAssertEqual(parse(["notify", "--title", "t"]), .failure(UsageError(message: "notify needs --title and --body")))
        XCTAssertEqual(parse(["trash"]), .failure(UsageError(message: "trash takes exactly one path")))
    }
}
```

`helper/vpt-macos/Tests/VptMacosTests/RunTests.swift`:

```swift
import XCTest
@testable import VptMacos

final class RecordingPoster: NotificationPoster {
    var posted: [(title: String, body: String)] = []
    var failure: PostFailure?

    func post(title: String, body: String) throws {
        if let failure { throw failure }
        posted.append((title, body))
    }
}

final class TemporaryTrasher: Trasher {
    let directory: URL

    init(directory: URL) { self.directory = directory }

    func trash(_ url: URL) throws {
        try FileManager.default.moveItem(at: url, to: directory.appendingPathComponent(url.lastPathComponent))
    }
}

final class RunTests: XCTestCase {
    private func scratch() throws -> URL {
        let base = FileManager.default.temporaryDirectory.appendingPathComponent("vpt-macos-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: base.appendingPathComponent("trash"), withIntermediateDirectories: true)
        return base
    }

    func testVersionPrintsTheHelperDocument() throws {
        let outcome = run(.version, poster: RecordingPoster(), trasher: TemporaryTrasher(directory: try scratch()))
        XCTAssertEqual(outcome.exitCode, 0)
        XCTAssertEqual(outcome.stdout, "{\"schema\":\"vpt.helper/1\",\"version\":\"1.0.0\"}")
    }

    func testNotifyPostsThroughTheProtocolAndPrintsPosted() throws {
        let poster = RecordingPoster()
        let outcome = run(.notify(title: "vpt: deferred", body: "3 waiting"), poster: poster, trasher: TemporaryTrasher(directory: try scratch()))
        XCTAssertEqual(outcome.exitCode, 0)
        XCTAssertEqual(outcome.stdout, "{\"posted\":true}")
        XCTAssertEqual(poster.posted.count, 1)
        XCTAssertEqual(poster.posted.first?.title, "vpt: deferred")
        XCTAssertEqual(poster.posted.first?.body, "3 waiting")
    }

    func testAFailedPostIsExit1WithNothingOnStdout() throws {
        let poster = RecordingPoster()
        poster.failure = PostFailure(detail: "no session")
        let outcome = run(.notify(title: "t", body: "b"), poster: poster, trasher: TemporaryTrasher(directory: try scratch()))
        XCTAssertEqual(outcome.exitCode, 1)
        XCTAssertEqual(outcome.stdout, "")
        XCTAssertTrue(outcome.stderr.contains("no session"), outcome.stderr)
    }

    func testTrashMovesThroughTheAdapterAndPrintsTheRequestedPath() throws {
        let base = try scratch()
        let victim = base.appendingPathComponent("victim.txt")
        try Data("bye".utf8).write(to: victim)
        let outcome = run(.trash(path: victim.path), poster: RecordingPoster(), trasher: TemporaryTrasher(directory: base.appendingPathComponent("trash")))
        XCTAssertEqual(outcome.exitCode, 0)
        XCTAssertEqual(outcome.stdout, "{\"trashed\":\"\(victim.path)\"}")
        XCTAssertFalse(FileManager.default.fileExists(atPath: victim.path))
        XCTAssertTrue(FileManager.default.fileExists(atPath: base.appendingPathComponent("trash/victim.txt").path))
    }

    func testAFailedTrashIsExit1AndTheFileStays() throws {
        let base = try scratch()
        let victim = base.appendingPathComponent("victim.txt")
        try Data("bye".utf8).write(to: victim)
        let outcome = run(.trash(path: victim.path), poster: RecordingPoster(), trasher: TemporaryTrasher(directory: base.appendingPathComponent("missing")))
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

Expected: the build fails, `parse`, `run`, `NotificationPoster` and the rest undefined.

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

    public init(message: String) { self.message = message }
}

public let usage = "usage: vpt-macos --version | notify --title <t> --body <b> | trash <path>"

public func parse(_ arguments: [String]) -> Result<Command, UsageError> {
    switch arguments.first {
    case "--version":
        return arguments.count == 1 ? .success(.version) : .failure(UsageError(message: "--version takes no arguments"))
    case "notify":
        return parseNotify(Array(arguments.dropFirst()))
    case "trash":
        guard arguments.count == 2 else { return .failure(UsageError(message: "trash takes exactly one path")) }
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
        guard index + 1 < words.count else { return .failure(UsageError(message: "\(words[index]) needs a value")) }
        switch words[index] {
        case "--title": title = words[index + 1]
        case "--body": body = words[index + 1]
        default: return .failure(UsageError(message: "unknown option \(words[index])"))
        }
        index += 2
    }
    guard let title, let body else { return .failure(UsageError(message: "notify needs --title and --body")) }
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

    public init(posted: Bool) { self.posted = posted }
}

public struct TrashedDocument: Codable, Equatable {
    public let trashed: String

    public init(trashed: String) { self.trashed = trashed }
}

/// One line of JSON with sorted keys and no escaped slashes.
public func encode<Document: Encodable>(_ document: Document) -> String {
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
    guard let data = try? encoder.encode(document), let text = String(data: data, encoding: .utf8) else {
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

    public init(detail: String) { self.detail = detail }
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

    public init(detail: String) { self.detail = detail }
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
    if !outcome.stdout.isEmpty { print(outcome.stdout) }
    if !outcome.stderr.isEmpty { FileHandle.standardError.write(Data((outcome.stderr + "\n").utf8)) }
    exit(outcome.exitCode)
}
```

`justfile` gains three recipes and the Swift sizes in `file-size`, and `ship` grows:

```just
swift-build:
  cd helper/vpt-macos && swift build -c release

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

Run: `cd helper/vpt-macos && swift test && swift build -c release && .build/release/vpt-macos --version`
Expected: 8 tests pass; the release build prints `{"schema":"vpt.helper/1","version":"1.0.0"}`.

Run: `just file-size`

Expected: no `FAIL` line; every Swift source is under 200 lines.

- [ ] **Step 5: Commit**

```bash
git add helper justfile .github .gitignore
SKIP_AI_COMMIT=1 git commit -m "feat(helper): the vpt-macos package with version, notify and trash"
```

______________________________________________________________________

### Task 34: The final gates and the pull request record

Nothing new is built here. This task proves the stage as a whole and prepares what the pull request
carries: the gate output, the file-size table and the by-hand mutation table spec section 12 requires.

**Files:**

- Modify: none, unless a gate fails and the fix is committed under the task that owns the file.

- [ ] **Step 1: Run every gate in CI order**

Run: `just ship`

Expected: `fmt` prints nothing; `clippy` ends in `Finished` with no warnings; `test` reports every crate
`test result: ok.` and no test over one second in the `--report-time` sense (spot-check the slowest with
`cargo test --workspace --features dev-tools -- -Z unstable-options --report-time` on a nightly only if
one looks slow; the sandbox tests spawn one process each and finish in tens of milliseconds); `doc`
finishes with no warning; `gitleaks` reports no leaks; `file-size` prints no `FAIL`; `swift-build` and
`swift-test` pass.

- [ ] **Step 2: Record the file-size table**

Run: `just file-size 2>&1 | sort -k2 -n | tail -15`

Expected: the fifteen largest files, none over 500 total, and any `WARN` line listed in the pull request
with the split it would take. The files this plan expects nearest the warning line are
`crates/vpt-adapters/src/config/schema.rs` (the key table),
`crates/vpt-application/src/ingest/candidate.rs`, `crates/vpt/src/compose.rs`,
`crates/vpt/src/doctor/checks.rs` and the two ingest acceptance suites. A file that crosses 500 splits
along the seam its task named before the pull request opens.

- [ ] **Step 3: Verify the mutants by hand and record the table**

For each row, apply the mutation to a scratch copy of the working tree, run the named test, confirm it
fails, then run the unmutated control and confirm it passes. The table goes in the pull request body.

| Behavior                              | Mutation                                                       | Test that must go red                                                          |
| ------------------------------------- | -------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| the wholeness gate                    | `inspect` returns `Ok` when `moov` is missing                  | `a_file_without_moov_is_refused`                                               |
| the rest gate                         | `>=` becomes `>` on the quiet period                           | `the_rest_gate_needs_the_whole_quiet_period`                                   |
| the size gate                         | the limit comparison drops one byte                            | `the_size_gate_defers_one_byte_over_the_limit_and_accepts_the_limit`           |
| the source stays read-only            | `open` drops `O_NOFOLLOW`                                      | `open_refuses_a_symbolic_link_and_reads_a_regular_file_by_descriptor`          |
| the source is unchanged after a sweep | staging writes back one byte to the source handle              | the Task 19 sweep test asserting `entries()` before equals after               |
| dry run touches no title copy         | `Mode::DryRun` refreshes the title copy                        | `dry_run_creates_no_state_directory_and_no_title_copy`                         |
| exclusive publication                 | `exclusive` falls back to `rename` when the target exists      | `publish_never_replaces_an_existing_target_and_names_it`                       |
| repair after rename, before clear     | `repair_publications` skips the directory sync before clearing | `a_failed_directory_sync_leaves_the_entry_pending`                             |
| target modified refuses               | the digest comparison in repair always matches                 | `any_other_bytes_are_refused_as_target_modified_and_nothing_is_overwritten`    |
| a future schema is refused            | `migrate` accepts any `user_version`                           | `a_future_schema_version_is_refused_with_its_number`                           |
| the write lock                        | `acquire` returns before `flock` succeeds                      | `a_second_acquisition_waits_the_bounded_time_then_reports_busy`                |
| the deadline kills the group          | `terminate` signals the child pid instead of the group         | `the_deadline_terminates_the_whole_process_group_and_reaps_it`                 |
| the bounded reader                    | `check_depth` ignores `[`                                      | `nesting_past_the_depth_limit_is_refused_without_allocating_the_tree`          |
| helper major version                  | `version_with_env` accepts any major                           | `a_helper_of_another_major_version_is_refused_naming_it`                       |
| command notify falls back once        | the fallback delivery is removed                               | `a_non_zero_command_falls_back_to_the_desktop_notice_once_and_reports_failed`  |
| an untracked file survives retention  | `Retention::run` trashes entries with no ledger owner          | `an_untracked_file_in_a_store_survives_and_is_reported_kept`                   |
| retention target modified             | `reconcile_intents` moves a path whose digest differs          | `replaced_content_is_a_refusal_naming_the_path_and_stays_pending`              |
| audio excluded by default             | `include_audio` is ignored                                     | `audio_is_excluded_unless_include_audio_is_set`                                |
| doctor never refuses at startup       | `checks::all` returns early on a config error                  | `without_a_config_the_config_check_fails_and_the_rest_are_reported_as_not_run` |
| symlink verify writes nothing         | `verify` creates the target when missing                       | `verify_names_a_missing_link_and_a_wrong_target_and_writes_nothing`            |

- [ ] **Step 4: Confirm the README carries both install steps**

Run: `grep -c 'cargo install --git https://github.com/webdavis/vpt vpt' README.md &&`
`grep -c 'swift build -c release' README.md`

Expected: `1` and `1`.

- [ ] **Step 5: Open the pull request**

The branch carries one commit per task in order. The pull request body carries the gate summary, the
file-size table and the mutation table. No commit is made by this task.

______________________________________________________________________

## Self-review

### Spec coverage, sections 4, 5 and 9 to 13 as they apply to stage 1

| Spec requirement                                                                                                                                                                                       | Task                                                                                                                                                                                                             |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 4.1 one home, default `~/.vpt`, seven stores with per-store overrides, `~` and `<home>/` expansion                                                                                                     | 3, 5                                                                                                                                                                                                             |
| 4.1 a root must be absolute after expansion; an invalid root is exit 2 naming the keys                                                                                                                 | 5, 27                                                                                                                                                                                                            |
| 4.1 setup creates the default store leaves; a store elsewhere needs an existing parent, leaf created                                                                                                   | 5, 6                                                                                                                                                                                                             |
| 4.1 every root resolved once at startup before any work                                                                                                                                                | 5, 27                                                                                                                                                                                                            |
| 4.1 stores pairwise disjoint, the home may contain them, no overlap with the Voice Memos container                                                                                                     | 5                                                                                                                                                                                                                |
| 4.1 a release destination may not overlap a private store, the state directory or the config directory                                                                                                 | 5                                                                                                                                                                                                                |
| 4.1 `path_escape` below a resolved root                                                                                                                                                                | not reachable in stage 1: every path stage 1 opens or publishes below a root is `<root>/<id>.m4a` with a validated identity and an exclusive create; the check lands with the first artifact renderer in stage 3 |
| 4.1 the ledger lives in `~/.local/state/vpt/` by default, never in a store                                                                                                                             | 5, 11                                                                                                                                                                                                            |
| 4.1 config at `~/.config/vpt/config.toml`, mode 0600                                                                                                                                                   | 5, 6                                                                                                                                                                                                             |
| 4.2 `symlink_target` with a non-default home is a startup refusal                                                                                                                                      | 5                                                                                                                                                                                                                |
| 4.2 `vpt symlink deploy` and `verify`, the refusals, the leaf creation, verify writes nothing                                                                                                          | 29                                                                                                                                                                                                               |
| 4.2 doctor runs verify; an unmanaged link is followed and reported, never removed                                                                                                                      | 30                                                                                                                                                                                                               |
| 4.3 the identity: local capture timestamp without a colon plus twelve hex characters of the digest                                                                                                     | 7, 8                                                                                                                                                                                                             |
| 4.3 the archive name `<id>.m4a`                                                                                                                                                                        | 19                                                                                                                                                                                                               |
| 4.4 SQLite in the state directory, 0600 in 0700, WAL, busy timeout, `user_version`, a future version refused                                                                                           | 11                                                                                                                                                                                                               |
| 4.4 the eleven tables of version 1, columns for stages 2 to 4 anticipated                                                                                                                              | 11                                                                                                                                                                                                               |
| 4.4 the in-memory twin under the same contract suites                                                                                                                                                  | 12, 14, 31                                                                                                                                                                                                       |
| 4.4 the write lock every mutating command holds                                                                                                                                                        | 13, 27                                                                                                                                                                                                           |
| 4.4 the dirty-publication protocol: record, publish, sync the directory, clear; repair before new work                                                                                                 | 14, 27                                                                                                                                                                                                           |
| 4.4 recovery clears nothing before the directory is durable (F19)                                                                                                                                      | 14                                                                                                                                                                                                               |
| 4.4 `target_modified` refuses every mutating command                                                                                                                                                   | 14, 27                                                                                                                                                                                                           |
| 4.4 `vpt show <id> --json` and `vpt list --json`                                                                                                                                                       | 28                                                                                                                                                                                                               |
| 4.5 off by default; holds per store from a file's own mtime; `0` means never                                                                                                                           | 3, 4, 31                                                                                                                                                                                                         |
| 4.5 rule 1: only ledger-owned artifacts, identity verified by digest, untracked files kept and reported                                                                                                | 32                                                                                                                                                                                                               |
| 4.5 rule 2: journaled intents, reconciliation before new work, `retention_target_modified`                                                                                                             | 31, 32                                                                                                                                                                                                           |
| 4.5 rule 3: audio excluded unless `include_audio`; `audio_trashed_at` recorded                                                                                                                         | 32                                                                                                                                                                                                               |
| 4.5 rule 4: nothing unlinked; helper absent is `no_trash`                                                                                                                                              | 32                                                                                                                                                                                                               |
| 4.5 rule 5: the `retention` event with counts per store                                                                                                                                                | 32                                                                                                                                                                                                               |
| 5.1 depth-one `*.m4a` candidates, the Apple subdirectories never entered, counts for doctor                                                                                                            | 15, 30                                                                                                                                                                                                           |
| 5.1 `SF_DATALESS` through the port, never opened                                                                                                                                                       | 10, 15, 20                                                                                                                                                                                                       |
| 5.1 read-only descriptors, `O_NOFOLLOW`, no write to the container ever                                                                                                                                | 15, 19                                                                                                                                                                                                           |
| 5.1 the private title copy under the state directory, read-only open, schema change is unavailable                                                                                                     | 16                                                                                                                                                                                                               |
| 5.2 the seen triple skip, the size gate, the rest gate, the wholeness gate                                                                                                                             | 9, 10, 19, 20                                                                                                                                                                                                    |
| 5.2 deferral counts, the deferred page at the threshold, `--once`                                                                                                                                      | 20, 23                                                                                                                                                                                                           |
| 5.2 dry run: no title copy, no durable state                                                                                                                                                           | 23, 27                                                                                                                                                                                                           |
| 5.3 staging by clone, digest, exclusive publication, file and directory sync before commit                                                                                                             | 17, 18, 19                                                                                                                                                                                                       |
| 5.3 duplicates: same digest is skipped or recovered; a different file at the target is `archive_collision`                                                                                             | 21                                                                                                                                                                                                               |
| 5.3 a failed staging goes to the Trash; helper absent leaves it and doctor reports `cleanup_pending`                                                                                                   | 19, 23, 30                                                                                                                                                                                                       |
| 5.4 a deleted source is `source_gone_at`, reported by doctor; a moved source is recovered by digest                                                                                                    | 21, 22, 30                                                                                                                                                                                                       |
| 5.4 an orphaned archive is recovered into the ledger                                                                                                                                                   | 22                                                                                                                                                                                                               |
| 5.5 an unreadable or emptied store is exit 1 with `completed`; no space; the `ingest_failed` event                                                                                                     | 20, 23, 27                                                                                                                                                                                                       |
| 5.6 what stage 1 does not do                                                                                                                                                                           | nothing to build                                                                                                                                                                                                 |
| 9 `--json` withheld until the final status; error on stderr with `completed`; `vpt: <message>` otherwise                                                                                               | 2, 27                                                                                                                                                                                                            |
| 9 `--config` and `VPT_CONFIG`; unknown argument is usage exit 2                                                                                                                                        | 1, 2, 5                                                                                                                                                                                                          |
| 9 `vpt setup [--force]`                                                                                                                                                                                | 6                                                                                                                                                                                                                |
| 9 `vpt doctor` and its two output shapes                                                                                                                                                               | 30                                                                                                                                                                                                               |
| 9 `vpt ingest [--dry-run] [--once <path>]` and its shape                                                                                                                                               | 23, 27                                                                                                                                                                                                           |
| 9 `vpt show`, `vpt list [--stage]`, `vpt storage`                                                                                                                                                      | 28                                                                                                                                                                                                               |
| 9 `vpt retention run [--dry-run]`                                                                                                                                                                      | 32                                                                                                                                                                                                               |
| 9 `vpt symlink deploy` and `verify`                                                                                                                                                                    | 29                                                                                                                                                                                                               |
| 9 `vpt --version` with `helper_version`                                                                                                                                                                | 1, 25                                                                                                                                                                                                            |
| 9 the exit code mapping and the error document fields                                                                                                                                                  | 2                                                                                                                                                                                                                |
| 9 `--dry-run` opens existing state read-only, no migration, no write, no notification, no Trash                                                                                                        | 23, 27, 32                                                                                                                                                                                                       |
| 10 one key table with defaults, comments, secrets marked; `config_version = 1`; unknown keys refused                                                                                                   | 3                                                                                                                                                                                                                |
| 10 value rules by kind, the duration syntax, dynamic engine tables                                                                                                                                     | 3, 4                                                                                                                                                                                                             |
| 10 the setup template with the main engine chosen and every other key at its default                                                                                                                   | 3, 6                                                                                                                                                                                                             |
| 10.2 a home inside a vault: a symlinked home, stores overlapping the home                                                                                                                              | 5                                                                                                                                                                                                                |
| 11 the eight kinds and the rules stage 1 keeps: `archive_collision`, `target_modified`, `no_trash`, `retention_target_modified`, `doctor_checks`, `helper_version`, `symlink_deploy`, `symlink_verify` | 2, 21, 27, 29, 30, 32                                                                                                                                                                                            |
| 11 helper major mismatch is a refusal for the verbs that need the helper; doctor reports it                                                                                                            | 25, 30, 32                                                                                                                                                                                                       |
| 11 doctor never refuses at startup                                                                                                                                                                     | 30                                                                                                                                                                                                               |
| 11 nothing deletes; the staged file with no helper stays 0600 and is `cleanup_pending`                                                                                                                 | 19, 30                                                                                                                                                                                                           |
| 12 test-first, one second per test, unit tests beside the code                                                                                                                                         | every task                                                                                                                                                                                                       |
| 12 the fake engine behind `dev-tools`: `--version`, `notify`, `trash`, a hang, a command sink                                                                                                          | 24                                                                                                                                                                                                               |
| 12 the fake recorder store: assembled MPEG-4 bytes, the Apple subdirectories, the title database fixture                                                                                               | 9, 15, 16, 19                                                                                                                                                                                                    |
| 12 the fixed clock, the in-memory ledger under the contract, the temporary HOME with no real reads                                                                                                     | 1, 12, 19                                                                                                                                                                                                        |
| 12 the four stage 1 behaviors written first: source unchanged after a sweep, dry run leaves the title copy, repair after rename before clear, an untracked file survives retention                     | 19, 23, 14, 32                                                                                                                                                                                                   |
| 12 the Swift suite reaches no real destination; `just smoke` is operator-run                                                                                                                           | 33                                                                                                                                                                                                               |
| 12 CI on macOS with the seven gates and `just ship`                                                                                                                                                    | 1, 33                                                                                                                                                                                                            |
| 13 the repository layout, `rust-toolchain.toml` pinning stable, `Cargo.lock` committed                                                                                                                 | 1                                                                                                                                                                                                                |
| 13 `cargo install` installs `vpt` alone; the fake engine needs `dev-tools`                                                                                                                             | 1, 24                                                                                                                                                                                                            |
| 13 the helper built with `swift build -c release`; the README states both steps; doctor names them                                                                                                     | 1, 30, 33                                                                                                                                                                                                        |
| 13 the dotfiles builder and LaunchAgent                                                                                                                                                                | the dotfiles repository's, out of scope by the spec's own words                                                                                                                                                  |
| 13 `recordings_dir readable` reported by doctor                                                                                                                                                        | 30                                                                                                                                                                                                               |
| 14 the Stage 1 entry, every item                                                                                                                                                                       | 1 to 33                                                                                                                                                                                                          |

### Placeholder scan

Every step carries its code, its command and its expected output. A search of this document for `TBD`,
`TODO`, `placeholder`, `similar to`, `fill in`, `add appropriate` and `implement the rest` finds only
this sentence. The two stubs an earlier draft carried, a leaf-table probe in Task 3 and an
intended-digest helper in Task 14, are gone: each task's Step 3 is the code that stays.

### Type consistency

Names used across tasks resolve to one definition each:

- `RecordingRecord` (Task 12) carries `source_path: Option<PathBuf>` and `title_source: TitleOrigin`;
  Tasks 19, 22, 27 and 28 read those fields with those types. `TitleOrigin` is the ledger's enum and
  `TitleSource` the recorder port's trait; they never share a name.
- `RecordingLedger` (Task 12) has `seen`, `seen_all`, `record_seen`, `by_digest`, `by_id`, `recordings`,
  `commit_ingest`, `commit_recovered`, `set_source_path`, `set_audio_trashed`; Tasks 19 to 22, 30 and 32
  call only those.
- `Outcome` (Task 2) gains `FailedReport` in Task 30; `emit` handles all three variants.
- `Runtime` (Task 27) exposes `settings`, `roots`, `ledger`, `recorder`, `archive`, `clock`, `helper`,
  `notifier`, `mutating`, `trash`; Tasks 28, 30 and 32 use those names. `with_runtime` (Task 28) takes
  the `Creation` argument every dispatch arm passes.
- `HelperClient` (Task 25) has `version`, `version_with_env`, `notify`, `notify_with_env`,
  `trash_with_env`, `with_deadline`; Tasks 26, 27, 30 and 32 call those.
- `Sandbox` (Task 1) gains `install_fake_helper` and `fake_log` in Task 25, `write_config`,
  `add_recording` and `ledger` in Task 27, `set_mtime` in Task 32; `FAKE_ENGINE` is the constant Task 25
  defines.
- `spawn::run_with_env` (Task 25) is the one body; `run` calls it with no environment.
- `Check` (Task 2) is `{ name, ok, detail }`; Task 30 builds every check through it.
- `StoreKey::{all, key_name, default_leaf, from_key_name, is_private}` (Task 5) are the only store-key
  methods later tasks call.
