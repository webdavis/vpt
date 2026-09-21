# vpt design

Status: design, approved in shape by the operator on 2026-09-20. This is the one specification for vpt,
the Voice Processing Tool. It folds the seven design documents of 2026-09-14, the fourteen architecture
decisions of 2026-09-15, the ten earlier transcript decisions and the sixteen rulings of 2026-09-20 into
a single document that stands on its own. Where two of those sources disagreed, the ruling won and only
the winning answer appears here.

The tool is built in Rust, stage by stage, test-first, to the clean-code standard. The implementation
plan argues from this document; every requirement below is written so that a failing test can be named
for it.

## 1. Purpose and scope

vpt collects the voice memos a person records on an iPhone and that iCloud syncs to their Mac, keeps a
byte-identical copy of each original, transcribes it with one or two engines, writes a transcript note in
Markdown, and optionally hands the transcript to an agent command for a synthesis note. Around that
pipeline it offers per-occasion briefs assembled from those notes, redacted copies for sharing, a handoff
document for an external notebook, and a small amount of housekeeping.

It is for one person on one Mac: someone who records everyday personal voice memos, keeps notes in
Markdown (in an Obsidian vault or in a plain directory), and wants transcripts that say plainly where the
engines were unsure rather than transcripts that read as confident and are wrong.

vpt is not:

- a cloud service, a daemon, or a sync mechanism. Every command runs to completion and exits. A scheduler
  outside vpt (launchd on macOS) runs it on an interval.
- a sharing transport. vpt writes files. Mail, messaging, upload and the clipboard are absent, and the
  operator sends a file with their own tools.
- a backup. vpt's copy of a recording is an archive copy on the same disk; a machine backup is its own
  project.
- a second capture path. vpt reads Apple's Voice Memos store read-only and never writes into it, never
  configures Voice Memos, and never touches the dictation path (FluidVoice stays exactly as it is).
- a language model. Where a model is useful (synthesis, tag proposals) vpt runs a command the operator
  configured, disabled by default, and names no vendor.

vpt is a product other people install with `cargo install --git https://github.com/webdavis/vpt vpt`.
Nothing in it assumes the author's machine, vault, dotfiles repository, notification engine or homelab.
Everything machine-specific is configuration.

## 2. Rulings in force

Every operator decision the design rests on, one line each. A row records that a ruling exists and what
it says; it never records the option that lost.

### 2.1 The ten transcript decisions, 2026-06-01 to 2026-08-30

| #   | Date       | Ruling                                                                                    |
| --- | ---------- | ----------------------------------------------------------------------------------------- |
| T1  | 2026-06-01 | whisply is a transcription engine already in daily use for captions.                      |
| T2  | 2026-07-21 | Apple's Voice Memos store is the capture source; the original request was to watch it.    |
| T3  | 2026-07-21 | Third-party recorder applications are rejected as a capture route.                        |
| T4  | 2026-07-21 | Just Press Record is rejected as a capture route.                                         |
| T5  | 2026-07-21 | The purpose of extracting memos is a transcription pipeline.                              |
| T6  | 2026-07-21 | Four stages (extract, transcribe, vault note, agent synthesis), one spec, a staged build. |
| T7  | 2026-07-21 | Auto-create over review gates wherever a gate is not separately ruled.                    |
| T8  | 2026-07-21 | `minutes` adds a layer of complexity (the doubt that decision D1 confirmed).              |
| T9  | 2026-07-28 | FluidVoice is a must and stays in daily use; vpt never touches the dictation path.        |
| T10 | 2026-08-30 | whisply is installed and configured on the machine on sight.                              |

### 2.2 The fourteen architecture decisions, 2026-09-15

| #   | Ruling                                                                                                                                        |
| --- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| D1  | vpt does not use or depend on `minutes` in any form.                                                                                          |
| D2  | Whether audio leaves the machine is the user's choice; local is the default.                                                                  |
| D3  | Two engine slots, both named in config; whether the second runs on failure only or always is config.                                          |
| D4  | Reconciliation picks a winner, flags uncertainty inline plus a `vpt review <id>` queue; no summary block.                                     |
| D5  | Built: a winner rule, both-engines-fail handling, engine per language. Not built: a disagreement threshold, a cloud spend ceiling.            |
| D6  | `[notify]` has exactly three modes, `desktop`, `command`, `off`; command mode substitutes argv tokens and writes JSON on stdin, both, always. |
| D7  | vpt carries no copy of any notification engine's wire contract and no producer-wire crate.                                                    |
| D8  | Every subcommand ships `--json` on its own output, in vpt's own shape.                                                                        |
| D9  | The configured command's own exit code decides whether vpt falls back to its desktop notice.                                                  |
| D10 | Engines are named in config; two first-class adapters (Apple Speech, whisply) plus a generic command adapter with no uncertainty flagging.    |
| D11 | Two ideas adopted from FluidVoice with credit: local by default with cloud opt-in, and post-processing as its own layer.                      |
| D12 | whisply, openai-whisper and the ElevenLabs CLI stay declared on the machine; the `minutes` cask has no remaining use.                         |
| D13 | Posture converges on the same three-mode `[notify]` shape (a ledger task in the dotfiles repository, not vpt's).                              |
| D14 | The tool is named `vpt`, Voice Processing Tool; config at `~/.config/vpt/config.toml`.                                                        |

### 2.3 The sixteen rulings of 2026-09-20

| #   | Ruling                                                                                                                                                                                                                                                                                                                                                    |
| --- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| G1  | Code home: its own repository `webdavis/vpt`, built from a local clone by the dotfiles repository (the scalebar shape).                                                                                                                                                                                                                                   |
| G2  | Auto-create is a config switch: fully automatic (tags and links written, never asked), or hold a tag new to the vault.                                                                                                                                                                                                                                    |
| G3  | vpt spawns a configured agent command for model calls, disabled by default, no vendor named.                                                                                                                                                                                                                                                              |
| G4  | Home is `~/.vpt/` by default; config and credentials in `~/.config/vpt/config.toml`; one home, per-store overrides, an optional managed symlink deployed and verified by vpt.                                                                                                                                                                             |
| G5  | Ship now; vpt's copy is not a backup and backup is its own project.                                                                                                                                                                                                                                                                                       |
| G6  | Three uncertainty signals: disagreement, low confidence, and agreed-unverified risk-class spans suppressed by `known-terms.txt`.                                                                                                                                                                                                                          |
| G7  | A same-family engine pair is refused at startup, with an `allow_same_family` escape.                                                                                                                                                                                                                                                                      |
| G8  | The winner is a config-named main engine; the other slot is the checker.                                                                                                                                                                                                                                                                                  |
| G9  | Post-processing is a readability pass, off by default, that never touches a flagged span.                                                                                                                                                                                                                                                                 |
| G10 | Briefs on request (`vpt brief`) and a calendar trigger built now, opt-in through config.                                                                                                                                                                                                                                                                  |
| G11 | Calendar and task context come from `dam` or a native Rust Google Calendar reader, chosen by `type`; never gog.                                                                                                                                                                                                                                           |
| G12 | Retention is opt-in, off by default, one hold time per store; expired files move to the system Trash; vpt reports what moved.                                                                                                                                                                                                                             |
| G13 | Open Notebook receives released redacted copies by default; a config switch allows originals.                                                                                                                                                                                                                                                             |
| G14 | Apple's Voice Memos store is read read-only; the database is confined to the title lookup and degrades to untitled.                                                                                                                                                                                                                                       |
| G15 | Sharing is redaction at share time: no approval step, no expiry clock, no cloud service; automatic per recording or on demand; a shared folder in config with a per-run destination flag; standardized names; refuse a byte-identical duplicate; timestamp a name collision.                                                                              |
| G16 | Defaults taken without objection: `~/.cargo/bin` install; docs, tests and CI in the vpt repository; the Swift helper built inside that repository; the four stages ship in the July order; a brief reads only the calendars and projects config names; flag ranking and summary layout are spec details; the 28 low config defaults go in a strike table. |

### 2.4 Three rulings that override the seven 2026-09-14 documents

| #   | Date       | Ruling                                                                                                                            |
| --- | ---------- | --------------------------------------------------------------------------------------------------------------------------------- |
| O1  | 2026-09-19 | Task and event context comes from `dam`, never `td`.                                                                              |
| O2  | 2026-09-20 | Five crates in one workspace, to the clean-code standard, test-first without exception.                                           |
| O3  | 2026-09-15 | Notification is the producer API (argv tokens plus JSON on stdin, exit code load-bearing), never a named engine's submit command. |

Standing repository rules that bind vpt as well: Rust files target 200 implementation and 300 total
lines, and never exceed 500 total with tests included; no workspace depends on another; a tool never
hardcodes its own path; vpt ships no shell script; `trash` semantics wherever vpt removes a file; no
removal mechanism beyond the opt-in retention of G12; the operator runs applies.

## 3. Architecture

### 3.1 The five crates

One Cargo workspace, `Cargo.lock` committed, built `--locked`:

```
crates/vpt-domain        pure policy, no I/O
crates/vpt-application   use cases and the ports they own
crates/vpt-protocol      the versioned documents vpt reads and writes across a process boundary
crates/vpt-adapters      everything concrete: filesystem, SQLite, processes, HTTP, TOML
crates/vpt               the command crate: argument decoding, exit codes, composition
```

Dependency direction, enforced by each crate's `[dependencies]`:

```
vpt-domain <- vpt-application <- vpt-adapters <- vpt
vpt-protocol is used by vpt-adapters and vpt and depends on nothing in the workspace.
```

The command crate is named `vpt`, not `vpt-cli`, so that `cargo install --git <url> vpt` names the
package a person would guess. Its binary is `vpt`. `main.rs` is under 150 lines and holds no policy.

`vpt-domain` excludes filesystem access, SQLite, TOML, JSON, HTTP, environment variables, process
spawning, macOS APIs, vendor APIs and terminal output. It holds: the recording identity, the MPEG-4
wholeness gate over bounded box headers, the normalizer, the aligner, the flag classifier and ranking,
the readability pass, the slug sanitizer, the note renderer and the managed-block rewriter over strings,
the tag gate, the relation rules, the brief selector and pack, the redaction pass and residue scan, the
retention decision, and every value type. Each is a total function of its arguments. The crate has no
infrastructure dependency; a small crate for Unicode normalization form KC and full case folding is a
permitted domain primitive, and one normalization policy serves alignment, terms, tags and slugs, its
Unicode version pinned by the lockfile.

`vpt-application` holds one concrete use case per verb (`Setup`, `Run`, `Ingest`, `Transcribe`,
`WriteNote`, `Path`, `Synthesize`, `Review`, `Confirm`, `Brief`, `Redact`, `Handoff`, `Storage`,
`Retention`, `Doctor`, `Symlink`) and the ports below. A use case is a concrete type; no use case gets a
trait.

`vpt-protocol` holds the documents that cross a process boundary, each carrying a `schema` string with a
major version: `vpt.engine/1` (an engine's transcript), `vpt.event/1` (a notification), `vpt.proposal/1`
(an agent's synthesis), `vpt.context/1` (calendar and task context), `vpt.brief/1`, `vpt.handoff/1`, and
the `--json` output and error documents. It is plain serde over JSON. An unknown major version is refused
with the version named; an unknown additive field is accepted and named in diagnostics; byte, depth,
array-length and text-length limits are enforced while a document is read, before it is fully allocated:
16,777,216 bytes for `vpt.engine/1`, 1,048,576 bytes for `vpt.proposal/1` and 65,536 bytes for every
other incoming document; depth 8; 65,536 characters per text value; engine `segments` and `words` and
proposal `summary` and `actions` allow at most 65,536 entries, every other array 256.

Application ports exchange domain values (`Transcript`, `Notification`, `Proposal`, `ContextSnapshot`,
`Record`), never a protocol document: an adapter validates an incoming wire document and converts it into
the domain value, and serializes an outgoing value into its protocol document, so `vpt-application` does
not depend on `vpt-protocol`.

`vpt-adapters` is organized by capability, one module each, never one broad module.

### 3.2 Ports, and the adapter behind each

| Port (in vpt-application) | What a use case asks of it                                                                | Adapters (in vpt-adapters)                                                                         |
| ------------------------- | ----------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| `RecorderStore`           | list candidate recordings with size, mtime, flags; read bytes; read a title by path       | `voice_memos` (Apple's group container plus a private copy of its database); `fixture_dir` (tests) |
| `Archive`                 | clone bytes to the audio store, never replacing an existing target                        | `clonefile` (the raw syscall, byte-copy fallback on `EXDEV`)                                       |
| `Engine`                  | transcribe an audio path into a `Transcript`; report its model family and locality        | `apple` (the Swift helper), `whisply`, `command`                                                   |
| `Clock`                   | now, in UTC and in the local zone                                                         | system clock; fixed clock in tests                                                                 |
| `Ledger`                  | the repositories of section 4.4                                                           | `sqlite` (one database, write-ahead log, busy timeout); in-memory (tests)                          |
| `Stores`                  | resolve a store path; write a file atomically; read; list; walk up for a git working tree | `filesystem`                                                                                       |
| `Trash`                   | move a path to the system Trash                                                           | `macos_helper` (`vpt-macos trash`)                                                                 |
| `Notifier`                | deliver a `Notification`                                                                  | `desktop` (`vpt-macos notify`), `command` (argv tokens plus stdin), `off`                          |
| `ContextSource`           | a `ContextSnapshot` (occasions and tasks) for a window and a selection                    | `dam` (spawns `dam ls --json --no-pull`), `google` (native HTTPS, read-only scope), `none`         |
| `AgentCommand`            | run the configured synthesis command over a transcript and read a `Proposal` back         | `command`                                                                                          |
| `KnownTerms`, `KnownTags` | read and append the two confirmation lists                                                | `filesystem`                                                                                       |

Every port is a trait because each is an external capability with a fake in tests. Nothing else in the
workspace is a trait. `dyn Trait` appears only at the composition root in `vpt`.

### 3.3 How the four stages map onto the crates

| Stage                  | Domain                                           | Application use case                       | Adapters used                                             |
| ---------------------- | ------------------------------------------------ | ------------------------------------------ | --------------------------------------------------------- |
| 1 Extract              | identity, wholeness gate, sweep decision         | `Ingest`                                   | `RecorderStore`, `Archive`, `Ledger`, `Notifier`          |
| 2 Transcribe           | normalize, align, classify, rank, readability    | `Transcribe`, `Review`, `Confirm`          | `Engine` x2, `Ledger`, `Stores`, `KnownTerms`, `Notifier` |
| 3 Vault note           | render, managed block, tag gate, relations, slug | `WriteNote`, `Path`                        | `Stores`, `Ledger`, `KnownTags`                           |
| 4 Synthesis and extras | verify-note grounding, brief selector, redaction | `Synthesize`, `Brief`, `Redact`, `Handoff` | `AgentCommand`, `ContextSource`, `Stores`, `Ledger`       |

`vpt run` composes the use cases in stage order. Each stage of a recording is `pending`, `succeeded`,
`failed`, `disabled` or `expired`; success is recorded only after the stage's artifacts commit. Stage
eligibility is evaluated from the current configuration on every run: a stage that is off in config is
marked `disabled` and skipped, and an incomplete stage marked `disabled` on an earlier run becomes
`pending` when its configuration enables it, so a recording processed before synthesis was configured is
synthesized once it is. Disabling a stage discards nothing: an earlier `succeeded` or `expired` state
stays. Every derived stage records the digest of the exact input it consumed: synthesis records the
digest of the content-region bytes it wrote to the command's stdin, and a verification finding records
the claim, the transcript and the lexical-flag state it was computed against. Before selecting work on
every run, vpt renders the current synthesis input (a review correction and the readability pass both
change it without touching the transcript of record) and compares its digest with the recorded one; a
mismatch makes enabled synthesis `pending` again unless retention expired its output. A verification
finding and the inherited uncertainty persisted with a proposal are current only while the transcript and
the lexical-flag state they recorded are the current ones; a change to either invalidates the prior
result and the next run recomputes it. A run selects every recording with an enabled stage that is
`pending` or `failed` and resumes from its earliest such stage, so a transient failure is retried on the
next run; an `expired` stage (retention moved its artifact) is never retried by `run`, and a released
copy is immutable whatever changes upstream.

### 3.4 The macOS helper

Apple's speech framework, the desktop notification and the system Trash are reachable only through macOS
frameworks, so they live in one Swift executable, `vpt-macos`, built from `helper/vpt-macos/` in this
repository (a Swift package, `swift build -c release`). It has three subcommands, each a pure function of
its arguments with JSON on stdout:

```
vpt-macos transcribe <audio-path> --locale <tag>     prints one vpt.engine/1 document
vpt-macos notify --title <t> --body <b>              posts a notification, prints {"posted": true}
vpt-macos trash <path>                               moves to the Trash, prints {"trashed": "<path>"}
```

`transcribe` uses `SpeechAnalyzer` and `SpeechTranscriber` with per-token confidence and time range (the
`ConfidenceAttribute` and `TimeRangeAttribute` the framework exposes on macOS 26 and above), so the Apple
adapter is a first-class engine with uncertainty flagging. `notify` posts through the
`display notification` AppleScript command executed in-process with `NSAppleScript`, which needs no
signed bundle. `trash` calls `FileManager.trashItem(at:resultingItemURL:)`, which is the system Trash and
never `unlink`.

vpt finds the helper by the `[helper] path` config value, default `vpt-macos`, resolved on `PATH`. When
the helper is absent: the Apple engine is unavailable (a startup refusal if it is configured), `desktop`
notify degrades to `off` with one log line, and retention refuses to run (it never unlinks, so with no
Trash there is no removal). `vpt doctor` reports the helper's presence and version. The helper prints
`{"schema": "vpt.helper/1", "version": "<semver>"}` for `--version`, and vpt refuses a helper whose major
version differs from the one it was built against.

### 3.5 Spawned commands

Every command vpt spawns (an engine, the helper, the agent command, `dam`, the notify command) is
executed from its argv directly, never through a shell, in its own process group. Writing its stdin,
draining its stdout and stderr concurrently, and waiting share one deadline: the configured
`timeout_secs` for an engine or the agent command, 30 seconds for a whole context collection, five
seconds for a notification and for every helper call other than `transcribe`, `--version` included. At
the deadline, and when vpt itself is interrupted (`SIGINT`, `SIGTERM` or `SIGHUP`), vpt terminates the
group, force-kills survivors after one second and reaps the child before it exits, so no engine
descendant outlives the command that spawned it. Child output is bounded by the protocol limits and is
never quoted raw in a log line or an error document.

## 4. The home and the stores

### 4.1 One home, per-store overrides

The home is one directory, default `~/.vpt`, under which every store lands by default:

| Store            | Default path under the home | Holds                                                                                          | Sensitivity                               |
| ---------------- | --------------------------- | ---------------------------------------------------------------------------------------------- | ----------------------------------------- |
| `audio`          | `audio/`                    | the archive clone of each recording, `<id>.m4a`, mode 0600                                     | the recording itself                      |
| `transcripts`    | `transcripts/`              | the transcript note per recording                                                              | searchable text of the recording          |
| `analysis`       | `analysis/`                 | the synthesis note per recording                                                               | derived text                              |
| `briefs`         | `briefs/`                   | one brief per occasion                                                                         | names the people in a room                |
| `engine_outputs` | `engine-outputs/`           | each engine's raw `vpt.engine/1` document, `<id>.<engine>.json`, mode 0600                     | full transcripts with timings             |
| `drafts`         | `drafts/`                   | each redacted copy's private report, `<id>.<stage>.<creation-sequence>.report.json`, mode 0600 | the map from every mask to its real value |
| `released`       | `released/`                 | the shared folder: redacted copies ready to send                                               | intended to leave the machine             |

Each store has its own key under `[stores]` and may point anywhere. A store path is expanded from `~`
(and from a leading `<home>/`, which `vpt setup` emits for a derived default) and must be absolute after
expansion. `vpt setup` creates the default store leaves beneath the home; a store pointed elsewhere needs
an existing parent, and vpt creates its leaf and nothing above it. Every configured root (the home
through its permitted symlink, each store, the state directory, the config directory) is resolved once at
startup, before any work. Stores are pairwise disjoint, and the home may contain its stores, which is
what the defaults do; no writable store, staging path, state directory or release destination may overlap
the Voice Memos container; a release destination may not overlap a private store, the state directory or
the config directory. An invalid root is exit 2 naming the keys. Below a resolved root, a path that
traverses a symbolic link in any component, or that escapes the root, is refused when it is opened or
published, exit 3 `path_escape`.

vpt's own state (the ledger, section 4.4) does not live in a store. It lives in `~/.local/state/vpt/` by
default (`[home] state_dir`), so that a home inside a git-tracked vault never commits a database or its
write-ahead log.

Configuration and credentials live in `~/.config/vpt/config.toml`, mode 0600, and the two confirmation
lists beside it: `known-terms.txt` and `known-tags.txt`. A secret is a value in that file; how the
operator renders it there (a password manager, a template) is outside vpt.

### 4.2 The managed symlink

`[home] symlink_target` is empty by default. When set, `~/.vpt` (the default home path) is a symbolic
link to that directory and `[home] path` must stay at its default, a startup refusal (2) otherwise, so
the link and the home never name different places. vpt manages the link:

- `vpt symlink deploy` creates the link. It refuses when the default path exists and is not a link to the
  target (naming what is there), refuses when the target's parent does not exist, and creates the target
  directory when only the leaf is missing.
- `vpt symlink verify` exits 0 when the link exists and resolves to the target, exit 3 otherwise, naming
  the discrepancy. It writes nothing.
- `vpt doctor` runs `verify` as one of its checks.

When `symlink_target` is empty and the default path is a symlink, vpt follows it and reports the fact in
`doctor`; it never removes a link it did not create.

### 4.3 Recording identity and naming

A recording's identity is `<local-capture-timestamp>-<hash12>`, for example
`2026-08-24T144736-4f3ab19c02de`:

- the capture timestamp is the MPEG-4 `mvhd` creation time read out of the audio file itself, converted
  to the machine's local zone, formatted `YYYY-MM-DDThhmmss` (no colons, because Obsidian forbids a colon
  in a filename);
- `hash12` is the first twelve hexadecimal characters of the SHA-256 (secure hash algorithm 256-bit)
  digest of the whole file.

Both halves come from the file. Identity survives a schema change in Apple's database, a rename in Voice
Memos and a store migration. The same bytes at two source paths derive one identity. A recording edited
or trimmed in Voice Memos has different bytes and becomes a new recording beside the original; vpt never
follows an edit.

The ledger records the capture instant in UTC as well, with its offset, so the local form is derivable
and the UTC form is unambiguous across a daylight-saving transition. Lookup is by content digest first: a
sweep that derives a digest already in the ledger reuses that recording's identity and pinned paths
whatever the machine's zone is now, so a change of time zone never mints a second identity for known
bytes. The digest is unique in the ledger.

Notes are named `{date}-{slug}-{hash8}.md`, for example `2026-08-24-invoice-call-4f3ab19c.md`: the
capture date, a slug from the Voice Memos title, and the first eight hexadecimal characters of the
identity's hash. The slug comes from the title and from nothing else; a recording with no title gets
`untitled`; the name is pinned at first write and never changes when a title arrives or an engine
changes. Slug sanitization is a trust boundary (section 7.4).

### 4.4 The ledger

One SQLite database, `~/.local/state/vpt/vpt.db`, mode 0600 in a 0700 directory, write-ahead log (WAL)
mode, a 5 s busy timeout on every connection, versioned migrations applied at open inside a transaction.
It is the authority for everything vpt knows; a note is a rendering of it and the index over the output
tree is a cache rebuilt per run. SQLite rather than a file per record because two writers exist (a
scheduled `vpt run` and a manual command in a terminal) and because the clean-code standard vpt is built
to prefers a transactional store for multi-record state.

The repositories below, one SQLite type implementing all of them, plus an in-memory implementation that
runs the same contract tests:

| Repository             | Rows                                                                                                                                                                                                                                                                                                                                                      |
| ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `seen`                 | per source path: filename, size, mtime, flags, first seen, last seen, deferral count and reason, `source_gone_at`                                                                                                                                                                                                                                         |
| `recordings`           | per identity: source path, digest (unique), captured_at (UTC with offset), duration, title, title_source, ingested_at, stage states, note paths, audio path, engines, language, `open_flags`, `diagnostics`                                                                                                                                               |
| `transcripts`          | per recording: the accepted transcript of record, segments and word timings, with the engine that produced it                                                                                                                                                                                                                                             |
| `proposals`            | per recording: the accepted `Proposal` (summary, actions, tags, relations) with the digest of the content-region bytes the command received, and per claim its source ranges and inherited uncertainty with the lexical-flag state it was computed against                                                                                                |
| `flags`                | per flag: recording, identifier, shape (lexical, diagnostic or verification), class, occurrence ranges (null for a verification finding without a source), the artifact, claim index and claim digest of a verification finding, record text, alternative text, confidence, state, resolution text, resolved_at                                           |
| `occasions`            | per occasion: identity, provider key (unique), source (`manual`, `dam`, `google`), at, duration, title, participants, tags, the assembled pack with its source identities and digests, brief path, briefed_at                                                                                                                                             |
| `tags` and `relations` | per recording: tag, state (`confirmed`, `suggested`, `rejected`), provenance; relation kind, target, state, provenance, rule                                                                                                                                                                                                                              |
| `releases`             | per released copy: recording, source stage, absolute destination, content digest, source-artifact digest, creation sequence, report path, and a provenance snapshot (capture instant, engines, open lexical flags, the complete `open_flags` count with its verification findings, diagnostics, reviewed state) taken from the source artifact at release |

The ledger holds the render inputs: the accepted transcript, the accepted proposal and each assembled
brief pack commit before the artifact they render is published, and they survive the retention of raw
engine outputs, so a note can always be regenerated from the ledger. A note is a rendering of that state
and `vpt note write` reads nothing else. `vpt show <id> --json` prints a recording's full record;
`vpt list --json` prints the recordings table.

Every mutating command takes one advisory lock on `<state_dir>/write.lock` (`flock`, close on exec),
waiting at most five seconds and exiting 1 when it cannot; a read-only verb takes none. A mutation
commits its authoritative rows together with a dirty entry per artifact it must publish. The entry
records the target path, the digest of the bytes expected there before publication (or that the target is
expected absent) and the digest of the bytes it intends to write. For an existing note the mutation first
validates the note's identity and markers, reads its current bytes and renders the managed regions into
them, so operator content outside the regions is preserved and the digest of those current bytes is the
expected previous digest. The artifact is then rendered from committed state, written to a temporary name
in its store, synced and renamed into place, the containing directory is synced, and the dirty entry is
cleared. The next mutating command repairs unfinished publications, and reconciles pending retention
intents (section 4.5), before new work: a target already carrying the intended digest is a completed
publication, and its entry is cleared only after its containing directory is synced, the same sync the
normal path performs before clearing; a target holding the expected previous bytes, or absent when
expected absent, is published over; any other bytes are refused (exit 3 `target_modified`) rather than
overwritten. A sync that fails on either path is exit 1 and leaves the entry pending. Read-only commands
never repair.

### 4.5 Retention

Off by default. When `[retention] enabled = true`, each store has one hold time (`[retention.hold]`, a
duration string such as `90d`, per store key, `0` meaning never expire), measured from a file's own
mtime. `vpt retention run` (and `vpt run`, when retention is enabled) moves every expired artifact in a
store to the system Trash through the helper, one call per file, and prints one line per moved file plus
a summary; `--json` prints the list. `--dry-run` prints the same list and moves nothing.

Five rules:

1. Retention considers only files the ledger records as vpt's own: an audio clone, an engine output, a
   transcript or analysis note, a brief, a private report or a released copy, current artifacts included.
   Before a move it verifies that the path still identifies that artifact: for a note or brief, its
   canonical ledger-owned path, its `vptRecording` or `vptOccasion`, its `vptStage` or `vptKind` and its
   required markers must all identify the selected artifact, the same test `verify-note` applies (section
   8.2); for every other file, the recorded digest must match. An untracked regular file in a store and a
   path whose identity, stage or digest names another artifact are kept and reported under `kept` with
   the reason. An expired artifact moves, and the ledger records `trashed_at` against it and keeps its
   pinned path. `vpt run` never recreates an artifact retention moved; `vpt note write <id>` or
   `vpt brief <occasion-id>` regenerates it deliberately.
1. Every move is journaled. Before the helper is invoked the ledger commits an intent naming the
   artifact, its pinned path and the identity and stage, or the digest, the validation above found there,
   and automatic regeneration of that artifact is prohibited while the intent is pending; on success the
   same command commits `trashed_at` and completes the intent. After a restart or an unknown helper
   outcome (a deadline, an unreadable reply) the next mutating command reconciles every pending intent
   before new work: an absent path completes the expiration, an unchanged original may be moved again,
   and replacement content is a refusal naming the path (exit 3 `retention_target_modified`). A pending
   or completed intent never triggers automatic regeneration.
1. The `audio` store is excluded unless `[retention] include_audio = true`, because the clone is the only
   copy once Apple evicts the original; when included, an expired clone moves and the recording's row
   records `audio_trashed_at`.
1. Nothing is ever unlinked. If the helper is absent, `retention run` refuses with exit 3.
1. What moved is reported in the run output and delivered as one `vpt.event/1` (`event = "retention"`,
   counts per store) when `[notify]` is not `off`.

## 5. Stage 1: Extract

### 5.1 The source, read-only

The source is Apple's Voice Memos group container, `[source] recordings_dir`, default
`~/Library/Group Containers/group.com.apple.VoiceMemos.shared/Recordings`. Every file handle into it is
opened read-only. vpt never creates, renames, rewrites or deletes anything inside it, and a full sweep
leaves every entry's size, mtime and flags unchanged.

The database, `CloudRecordings.db` beside `Recordings/`, is used for exactly one thing: the human title
(`ZCUSTOMLABEL` keyed by `ZPATH`). It is read from a private copy: the database plus its `-wal` and
`-shm` files are copied over the fixed paths under `<state_dir>/title-copy/` (directory 0700, files
0600), overwritten in place on every sweep that is not a dry run and never removed, and that copy is what
is opened and read. The live database is never opened, because a read-only open still lets SQLite create
journal files inside Apple's directory. A copy that fails, a schema that has changed, or a join that
finds no row yields `title_source = "unavailable"` and the recording is ingested untitled.
`[source] read_titles = false` skips the database entirely. The database supplies the title and nothing
else: duration and ingestion eligibility come from the audio container alone, and nothing read from the
database ever defers or refuses a recording.

### 5.2 The sweep

`vpt ingest` lists `recordings_dir` at depth one, takes `*.m4a` entries and nothing else, and never
recurses: `Capture/`, `CaptureRecovery/`, `CloudRecordings_ckAssets/` and `EncryptedCloudRecordings/` are
never entered. `vpt doctor` reports the entry count of each subdirectory so the operator can see whether
recordings are landing somewhere the sweep does not look.

For each candidate, in this order, cheapest first:

1. Skip an entry whose `st_flags` has `SF_DATALESS` (`0x40000000`) set, without opening it; record it as
   deferred with reason `dataless`. Opening one triggers a silent iCloud download.
1. Skip an entry whose `(filename, size, mtime)` triple is unchanged in `seen` and already ingested. This
   is an optimization only; deleting the ledger costs one full rehash and changes no outcome.
1. Open the source through one read-only descriptor (a regular file, never a symbolic link) and record
   its device, inode, size and nanosecond mtime. Every gate below reads that descriptor's metadata and
   bytes, never the path again.
1. Defer with `audio_too_large` when the descriptor's size exceeds `[source] max_audio_bytes` (default
   2,147,483,648), before reading any content.
1. Run the wholeness gate over the descriptor: walk the top-level MPEG-4 boxes reading bounded headers
   with checked offsets, never the whole file; the sum of box lengths must equal the file size exactly
   and a `moov` box must be present (Voice Memos writes `moov` last, so a truncated download loses it).
   An invalid length, an overflow, a box extending past the file or a missing `mvhd` defers with
   `invalid_container`. About forty lines, no audio library.
1. Require the file at rest: the descriptor's mtime at least `[source] quiet_period_secs` (default 30) in
   the past, and for an entry deferred on an earlier sweep, size unchanged since that sweep.
1. Stage, verify, hash, publish: clone the descriptor to a private staging name inside the `audio` store;
   read the descriptor's metadata again and defer with `changed_during_read` if anything moved; run the
   wholeness gate again over the staged bytes and derive the capture time, the duration, the digest
   (hashed in 64 KiB buffers) and the identity from the staged bytes alone, never from the source; set
   mode 0600; publish the staged file at `<id>.m4a` (section 5.3); and insert the recording and seen rows
   in one transaction. The record is part of the run's result document.

A candidate that fails any gate is deferred with its reason and retried next sweep. It is never partially
ingested: the clone is the first durable act and happens only after every source gate passes, and a
staged gate that fails defers the candidate, moves the staged file to the Trash (section 11) and
publishes nothing.

### 5.3 Cloning and duplicates

Publication syncs the completed staging file to disk (on the clone path and the byte-copy path alike),
moves it to `<id>.m4a` in one step that never replaces an existing target, and syncs the archive
directory before the recording and seen rows commit, so a committed ingestion always has its archive on
disk. A sync that fails is exit 1 and never records an ingestion. An existing `<id>.m4a` is a refusal
naming the path, and that refusal is the duplicate guard between two racing sweeps, which needs no lock
beyond the one every mutating command holds. The plan verifies the exclusive-rename primitive available
on the platform before choosing it. On that refusal vpt verifies the existing archive: its full digest
must equal the staged digest and its container must validate. It then syncs the verified archive and its
directory, recovers any missing recording or seen row in one transaction, moves the staged duplicate to
the Trash, and reports `already ingested`. A digest mismatch is exit 3 `archive_collision`, and nothing
is replaced. The staging clone is the `clonefile` family through `libc`, from the opened descriptor;
`EXDEV` (the store on another volume) falls back to a byte copy into a unique mode-0600 temporary file
that takes the same publish path, with one log line saying the archive is not copy-on-write. `ENOSPC`
aborts the sweep. At the start of every sweep, an archive file with no ledger row is recovered by
validating and digesting it, even when its source is gone.

### 5.4 Deleted, edited and moved recordings

A recording whose source path disappears after ingestion is marked `source_gone_at` in `seen` on the next
sweep, reported in `doctor`, and nothing else happens: the clone stays. An edited recording is a new
recording (section 4.3). A recording renamed inside Voice Memos keeps its bytes, so its identity, and the
sweep sees a new `(filename, size, mtime)` triple that hashes to a known identity; it updates the source
path and takes nothing.

### 5.5 Failure modes and pages

| Condition                                                | Sweep behavior                                | Event                                                                                       |
| -------------------------------------------------------- | --------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `recordings_dir` unreadable                              | abort, exit 1, nothing ingested               | `ingest_failed`, once per run                                                               |
| readable, zero `.m4a`, store previously seen non-empty   | abort, exit 1                                 | `ingest_failed` (an empty store is the silent failure)                                      |
| database copy or read fails                              | continue, untitled                            | none, recorded per recording                                                                |
| a gate fails                                             | defer, retry next sweep                       | `deferred` after `[source] deferral_page_threshold` (default 4) consecutive deferrals, once |
| `SF_DATALESS`                                            | defer, never open                             | as above                                                                                    |
| entry larger than `max_audio_bytes`                      | defer, never read                             | as a gate                                                                                   |
| source changed while staged                              | defer with `changed_during_read`              | as a gate                                                                                   |
| publish finds `<id>.m4a` with the same digest            | recover missing rows, report already ingested | none                                                                                        |
| publish finds `<id>.m4a` with a different digest         | refuse, exit 3 `archive_collision`            | `ingest_failed`                                                                             |
| `ENOSPC`                                                 | abort, exit 1                                 | `ingest_failed`                                                                             |
| a sync of the staged file or the archive directory fails | abort, exit 1, no row recorded                | `ingest_failed`                                                                             |
| audio store parent missing                               | refuse at startup, exit 2                     | `config_refused`                                                                            |

`vpt ingest --dry-run` runs every gate and writes nothing durable: no clone, no row, no event and no
title copy; it reports the title from accepted ledger state when the recording is already known and
`title_source = "unavailable"` otherwise, and no gate depends on a title. `--once <path>` ingests exactly
one file by path, gates included.

### 5.6 What stage 1 does not do

It does not transcribe, tag, write a note or notify about content. FluidVoice is not a source: it is a
dictation tool and vpt does not touch the dictation path. The `.waveform` sidecars in Apple's store are
ignored.

## 6. Stage 2: Transcribe

### 6.1 Two engine slots

`[engines] main` names the engine whose text becomes the transcript of record. `[engines] checker` names
the second engine, or is empty. `[engines] checker_runs` is `always` (both engines run on every recording
and are compared) or `on_failure` (the checker runs only when the main engine fails, and its output
becomes the record with a `single-engine` flag). An empty checker means one engine runs and the two
confidence and risk-class signals apply alone.

Each engine is a table `[engines.<name>]` with `kind = "apple" | "whisply" | "command"` and the adapter's
own keys. The shipped default is `main = "apple"` with an empty checker: local, free, on device, no
download. Cloud engines reach vpt through the `command` kind and declare `local = false` so that
`vpt doctor` and the first-run wizard can state whether audio leaves the machine. vpt never writes "audio
never leaves the machine" as a product rule; it reports the configured posture.

Per-language pairs: `[engines.by_language.<language-tag>]` carries `main` and `checker` for that
language; the pair for a run is chosen by `[recording] language` (default `en-US`), overridden per
invocation with `vpt transcribe --language`. An engine that reports a language different from the one it
was asked for produces a `language-mismatch` flag on the whole recording.

### 6.2 The engine contract

Every engine adapter produces one `vpt.engine/1` document:

```json
{
  "schema": "vpt.engine/1",
  "engine": {
    "name": "apple",
    "tool": "vpt-macos",
    "version": "1.0.0",
    "model": "SpeechTranscriber",
    "family": "apple"
  },
  "language": "en-US",
  "local": true,
  "segments": [
    {
      "start": 0.0,
      "end": 4.2,
      "text": "The quarterly invoice has not cleared yet.",
      "words": [
        {
          "text": "The",
          "start": 0.0,
          "end": 0.3,
          "confidence": 0.97
        }
      ]
    }
  ]
}
```

`confidence` is optional per word, because whisply on Apple's MLX framework reports none; an absent
confidence is never fabricated. `family` is what the same-family check compares: `apple`, `whisper`, or
whatever a `command` engine declares in its config (`family = "..."`, required).

- `apple` spawns `vpt-macos transcribe <path> --locale <lang>`, family `apple`, local.
- `whisply` spawns `whisply run -f <path> -o <dir> -d <device> -m <model> -l <lang> -e json` with
  `[engines.whisply] device` (default `mlx`) and `model` (default `large-v3-turbo`), parses its JSON,
  family `whisper`, local; with `device = "cpu"` per-word scores are read, with `mlx` they are absent.
- `command` spawns `[engines.<name>] command` (an argv list) with the tokens `{audio}`, `{language}` and
  `{out_dir}` substituted, reads a `vpt.engine/1` document from its stdout, and carries the declared
  `family` and `local`. Its output supplies text only: it takes part in no disagreement, confidence or
  agreed-unverified classification, and when it is the transcript of record no lexical flag is generated
  for that recording (diagnostic flags still are). The config comment says so at the key.

An engine document is accepted only when its text is non-empty; every time is finite and
`0 <= start <= end <= duration` of the recording; segments are in non-decreasing start order; each word
range lies inside its segment; each confidence, when present, is in `[0, 1]`; and the document's engine
name, family and locality match the adapter's configuration. A violation is an engine failure, and an
invalid document never replaces accepted state.

Engines run sequentially. Each has a wall-clock deadline, `[engines] timeout_secs` (default 1800); a
deadline kill is an engine failure. An empty transcript (zero words) is an engine failure. A run in which
only one engine produced a transcript, whatever the reason, writes that transcript with one
whole-recording `single-engine` flag. Both engines failing keeps the recording at stage `ingested`, marks
it `transcription-failed` for review, and pages `transcribe_failed`; the audio is never touched.

### 6.3 Same-family refusal

At startup, when both slots are set, vpt resolves both engines' declared families and refuses to run when
they are equal, naming both engines and the family, exit 3, no engine spawned.
`[engines] allow_same_family = true` lets the pair run. Two runtimes of one Whisper model were measured
to produce byte-identical transcripts, so a same-family pair costs double and detects nothing; a refusal
is chosen over a warning because identical output looks like a clean recording.

### 6.4 Reconciliation: normalize, align, classify

The normalizer, applied to both transcripts and used only for alignment: Unicode normalization form KC,
case folding, drop every non-alphanumeric character per token (an emptied token is dropped and its index
remembered), digit groups lose separators (`12,400` becomes `12400`), ordinal suffixes are stripped
(`23rd` becomes `23`). Nothing else: no stemming, no stop words, no number-word conversion.

Alignment is a Myers-style word diff over the normalized streams. When the edit distance exceeds
`[reconcile] max_divergence_ratio` (default 0.35) of the token count, span comparison stops and one
whole-recording `divergent` flag is raised instead of a span list.

Flags come in three shapes. A lexical flag covers one or more word ranges of the transcript. A diagnostic
flag covers the whole recording and has no range: `single-engine`, `language-mismatch` and `divergent`. A
verification finding (section 8.2) belongs to one claim of one artifact and may have no source range.
Diagnostic flags appear in `vpt review`, are counted separately (`diagnostics` in events,
`vptDiagnostics` in frontmatter), and overlap no span, so they neither hold back the readability pass nor
omit text from a redacted copy. `divergent` additionally marks the transcript unverified for consumers (a
brief and a handoff report `reviewed: false`) while confidence and risk-class classification continue
over the record engine's text.

Lexical candidates are every disagreement between the two engines, every record word below
`[reconcile] confidence_floor` (default 0.35), and every agreed or single-engine span in a risk class (a
proper noun, number, date, time or money amount). Each candidate becomes one flag of exactly one class by
the first matching rule below, which is also the surfaced ranking; a `command` engine's output is exempt
as section 6.2 states:

| Rank | Class               | Rule                                                                                                 | Surfaced          |
| ---- | ------------------- | ---------------------------------------------------------------------------------------------------- | ----------------- |
| 0    | `formatting`        | one side is the other plus insertions from `{the, of, a, an, and, to, at, is}`, or an ordinal suffix | no, recorded only |
| 1    | `numeric`           | either side holds a digit run and the runs differ                                                    | yes               |
| 2    | `proper-noun`       | the engines disagree and either side's span is capitalized in the original and not sentence-initial  | yes               |
| 3    | `low-confidence`    | no disagreement, and the record's word confidence is below the floor                                 | yes               |
| 4    | `agreed-unverified` | agreed or single engine, not low confidence, in a risk class, and not in `known-terms.txt`           | yes, aggregated   |
| 5    | `other`             | any other lexical disagreement                                                                       | yes               |

`agreed-unverified` is aggregated by surface form for presentation: one row per distinct string with its
occurrence count and first timecode, while the flag itself stores every occurrence range, so every
occurrence is protected by the readability pass and omitted by redaction. A term present in
`known-terms.txt` (whole-token, case-insensitive) never produces an `agreed-unverified` flag, and still
produces a `numeric` or `proper-noun` flag when the engines disagree about it. A lexical flag's
identifier is derived from the recording identity, its class, its occurrence ranges, its record text and
its alternative text, so a resolution survives only an unchanged flag; a re-run keeps every resolution
whose identifier still exists, adds the new flags, and recomputes the open count from the lexical flags
currently surfaced plus the verification findings of the recording's owned artifacts.

### 6.5 The readability pass

Off by default (`[readability] enabled = false`). When on, it rewrites the note body only, never the
engine output in the `engine_outputs` store and never the record text on a flag: sentence-initial
capitalization, a terminal full stop at a segment boundary that lacks one, and removal of tokens from a
closed filler list (`um`, `uh`, `erm`, `like` only when followed by a comma). It never touches a span
that carries a lexical flag of any class, including `formatting`: the flagged words and their inline
markers are copied through byte for byte. The pass is a pure function of (segments, flags, config) and
the note records `vptReadability: true` when it ran.

### 6.6 What gets written

- `engine_outputs/<id>.<engine>.json`: each engine's raw document, verbatim, kept.
- the accepted transcript of record (segments and word timings) in the ledger's `transcripts` repository,
  committed before the note is published.
- the `flags` rows in the ledger, with state `open`.
- the transcript note (stage 3 renders it; stage 2 supplies its body).
- one `vpt.event/1` (`event = "review_needed"`) when any surfaced flag is open, carrying counts per
  class, the identity and the note path; never a flagged span, an alternative, a title or any transcript
  text. Past `[notify] aggregate_after` (default 3) recordings needing review in one run, one aggregate
  event carries the count instead of one per recording.

### 6.7 Review and confirmation

```
vpt review <id> [--json]                     print open flags, ranked: record text, alternative, timecode
vpt review <id> --resolve <flag-id> --confirm | --correct <text> | --dismiss
vpt confirm --term <text>                      append to known-terms.txt (idempotent)
vpt confirm <id> --tag <t> | --reject-tag <t> | --relation <kind>:<target>
```

`--confirm` on a `proper-noun` or `agreed-unverified` flag appends the record's text to
`known-terms.txt`, which is what makes the list shrink. `--correct` stores the operator's text on the
flag, marks it `corrected`, and appends the corrected text to `known-terms.txt` (idempotently), so a
correction improves every future transcript and redaction; the transcript of record is never rewritten
and the note's inline marker is re-rendered as `[corrected: <text>]`. `--dismiss` closes the flag. A
resolved flag stays resolved across re-runs. Every resolution re-renders the note, so a correction
changes the synthesis input and makes a succeeded synthesis `pending` again (section 3.3). The inline
marker in the note is the exact word wrapped as `[unverified: <record text> | <alternative>]` for a
disagreement and `[unverified: <text>]` for the other classes; the note's frontmatter carries
`vptOpenFlags`.

## 7. Stage 3: Vault note

### 7.1 Two profiles

`[note] profile` is `portable` (shipped default, what every test runs against) or `obsidian`. Both write
the same body; the profile decides the frontmatter and the link style.

Portable frontmatter, flat keys only, every key prefixed `vpt` in camelCase because that is what a
Dataview query reads as `p.vptRecording`:

```markdown
---
vptSchema: 1
vptRecording: 2026-08-24T144736-4f3ab19c02de
vptStage: transcript
vptCapturedAt: 2026-08-24T14:47:36-06:00
vptDurationSecs: 612
vptOpenFlags: 6
vptDiagnostics: 0
vptEngines:
  - apple:SpeechTranscriber
  - whisply:large-v3-turbo
vptReadability: false
tags:
  - invoice
vptSuggestedTags:
  - billing/quarterly
---
```

Obsidian frontmatter puts the vault's documented keys first, in the vault's documented order, and the
`vpt` keys after them:

```markdown
---
aliases:
  - Invoice call
reference: "[[2026-08-24T144736-4f3ab19c02de.m4a]]"
hub: "[[transcripts]]"
tags:
  - invoice
status: active
description: Voice memo recorded 2026-08-24
startDate: 2026-08-24
vptSchema: 1
vptRecording: 2026-08-24T144736-4f3ab19c02de
...
---
```

`hub` is `[note.obsidian] hub_transcripts`, `hub_analysis` and `hub_briefs`, the folder note names
(defaults `transcripts`, `analysis`, `briefs`). On first write `status` is `active` and `startDate` is
the note's own creation date, the local date on which vpt first writes it, which is the vault's rule for
a regular note; the capture instant stays in `vptCapturedAt` and the capture date in the filename. A
rewrite preserves whatever `status`, `startDate` and `description` the note carries, so a value the
operator set (`complete`, `archived`, `capture`, `needs-correction`) is never undone. Uncertainty is
carried by `vptOpenFlags` and the inline markers, never by `status`. A wiki link in frontmatter is always
quoted, because the unquoted form parses as a nested sequence. `vptSchema` is an integer and the only
compatibility signal: a note whose `vptSchema` is higher than the build understands is read and never
rewritten, and the mismatch is reported. `createdDate` and `createdTime` are never written.

`[note] link_style` is `markdown` (default; `[Name](relative/path.md)`, destination URL-encoded) or
`wiki` (`[[Name]]`). The `obsidian` profile does not force `wiki`; the operator sets both.

### 7.2 The body

```markdown
# 2026-08-24-invoice-call-4f3ab19c

<!-- vpt:links start -->
- Audio: `audio/2026-08-24T144736-4f3ab19c02de.m4a`
- Analysis: [2026-08-24-invoice-call-4f3ab19c](../analysis/2026-08-24-invoice-call-4f3ab19c.md)
- Continues: [2026-08-24-invoice-prep-91b2c740](2026-08-24-invoice-prep-91b2c740.md)
- Mentions: [Rajesh Muthukrishnan](../../contacts/Rajesh%20Muthukrishnan.md)
<!-- vpt:links end -->

<!-- vpt:content start -->
## Transcript

[00:00] The quarterly invoice has not cleared yet.
[00:07] I spoke to [unverified: Siobhan Kovalchuk | Shavon Kovalchik] about it.
[00:09] Rajesh [unverified: Muthakrishnan] asked about the 23rd.
<!-- vpt:content end -->
```

The H1 matches the filename. Each segment is one paragraph prefixed with its `[mm:ss]` timecode; the
timecode is the source reference every consumer uses. Transcript uncertainty appears in exactly two
places, the inline markers and `vpt review <id>`: there is no summary block and no separate document.

A note has two managed regions, `vpt:links` and `vpt:content`, each between a start and an end marker.
Generated content (the transcript or the synthesis, with its inline markers and annotations) lives in
`vpt:content`; links live in `vpt:links`. A rewrite replaces the two regions and the frontmatter keys vpt
owns (every `vpt` key, and `tags` merged so a tag the operator added by hand is kept) and preserves every
other byte, so prose a person adds above, between or below the regions survives. A note whose required
markers are missing, doubled or unbalanced is refused, not repaired, exit 3 `markers`, with the path
named. Both directions are written: when the analysis note appears, both notes' link regions link each
other. vpt never writes into a note it did not create; a `mentions` link points out only.

Every external string is data when it is rendered into a note. A frontmatter scalar (a title, an alias, a
tag) is YAML-encoded; body text and a link label are Markdown-escaped, and an embedded newline stays
inside the paragraph or bullet that carries it; only the renderer constructs the managed delimiters, the
uncertainty annotations and the links. Engine text, proposal text, a Voice Memos title, an indexed note
name and a review correction can therefore never introduce a marker, an annotation, a link or a
frontmatter key.

### 7.3 Tags and the auto-create switch

Tags arrive from a `vpt.proposal/1` document (stage 4) or from `vpt confirm`. Every proposed tag is
processed in this order, and a rejection is a log line naming the step, never a page:

1. Normalize: a multi-word proposal is joined into kebab-case, and a proposal that differs from a
   vocabulary tag only by case or separator is folded onto that tag's existing spelling. An existing tag
   is never reshaped.
1. Syntax: letters, digits, `_`, `-`, `/`; at least one non-numeric character; no leading `#`. A tag that
   still fails after normalization is rejected.
1. Classify: a tag in the vocabulary or in `known-tags.txt` is `confirmed`; any other tag follows
   `[tags] new_tags`.
1. Deduplicate, then cap: at most `[tags] max_per_note` (default 5) confirmed tags and `max_suggested`
   (default 10) held ones, excess dropped in proposal order and recorded.

The vocabulary and the note index are rebuilt each run and read only. In the `obsidian` profile they
cover the `tags` values, note names and aliases of every Markdown note under `[note.obsidian] vault_root`
(a required, existing, absolute directory; symbolic links are not followed), which is what makes an
established vault tag `confirmed` under `hold` and what `mentions` resolves against. In the `portable`
profile they cover the transcripts, analysis and briefs stores. Both add their confirmation lists.
Indexing grants no write authority: vpt still writes into no note it did not create.

`[tags] new_tags` is the auto-create switch:

- `write` (fully automatic): a new tag is written to `tags` as `confirmed` and appended to
  `known-tags.txt`; nothing is ever held and nothing asks.
- `hold` (shipped default): a new tag is written to `vptSuggestedTags` as `suggested` until
  `vpt confirm <id> --tag <t>` promotes it or `--reject-tag` drops it.

Relations and links are always written, in both settings. The switch governs tags only.

### 7.4 Relations and filing

Four relation kinds, a closed set: `continues` (derived: the previous recording's capture ended within
`[relations] session_gap_minutes`, default 60), `mentions` (derived: a confirmed known term occurs in the
recording's accepted transcript as a whole-token, case-insensitive match and resolves to exactly one note
name or alias in the index of section 7.3, exactly, never fuzzy; the matched transcript ranges are
recorded on the relation; two index matches write no link and record the ambiguity), `related` (proposed
by the agent command, written with its provenance shown in the link block), `supersedes` (recorded only
by `vpt confirm --relation`). A fifth kind is a schema change.

Filing is by store: the transcript note goes to `transcripts`, the analysis note to `analysis`, the brief
to `briefs`, flat, named by the template above. That gives the five properties the ledger asks of
deterministic filing: total (every stage has a store), pure (a path is a function of stage, capture date,
title and identity, and of nothing else, not of tags, not of the clock, not of the directory's contents),
stable (the path is pinned in the ledger at first write), explainable (`vpt path <id> --stage <stage>`
prints it and writes nothing), collision-free (the name carries `hash8`, and a target that exists
carrying a different `vptRecording` is refused naming both).

Slug sanitization: Unicode normalization form KC, case folding; every character that is not a letter,
digit or hyphen becomes a hyphen; runs collapse; leading and trailing hyphens trimmed; truncate to
`[note] slug_max_chars` (default 60) at a hyphen boundary; an empty result is `untitled`. A resolved path
that escapes its store or traverses a symbolic link in any component is refused, never repaired.

When the operator renames a note inside their editor, vpt finds it again: the pinned path is missing, so
vpt scans the store for the one note whose `vptRecording` equals the identity, re-pins to it and logs one
line. No match means the note is rewritten at the pinned path; more than one match is a refusal naming
every claimant.

### 7.5 A home inside a git-tracked vault

vpt never runs `git`, never stages, never commits. It writes files and lets whatever watches the
directory commit them. Consequences vpt states rather than hides:

- `vpt doctor` reports, for each store, whether it resolves inside a git working tree (a walk up for a
  `.git` entry) and whether the `audio` store is one of them, so the operator can confirm their ignore
  rules exclude audio extensions.
- vpt writes no folder note, no ignore rule and no plugin configuration. Vault furniture is the
  operator's, written once.
- The `drafts` and `engine_outputs` stores default under the home like every other store. With a home
  inside a vault, the operator either accepts that those JSON files are committed or points the two
  stores elsewhere; `doctor` names the two as sensitive when they sit in a git tree.

## 8. Stage 4: Synthesis and extras

### 8.1 The agent command

`[synthesis] command` is an argv list, empty by default, which means synthesis is off: `vpt run` skips it
and `vpt synthesize` refuses with exit 2. Automatic redaction is not part of that switch; it follows
`[share] automatic` alone (section 8.5). When set, `vpt synthesize <id>` runs the command with
`{transcript}` (the note path), `{id}` and `{language}` substituted, writes the transcript note's content
region on the command's stdin, records the digest of exactly those bytes as the proposal's input digest
(section 3.3), and reads one `vpt.proposal/1` document from its stdout within `[synthesis] timeout_secs`
(default 600):

```json
{
  "schema": "vpt.proposal/1",
  "by": "<whatever the command says>",
  "summary": [
    {
      "text": "The quarterly invoice has not cleared.",
      "sources": [
        "04:12-04:19"
      ]
    }
  ],
  "actions": [
    {
      "text": "Bring the last three statements.",
      "sources": [
        "02:07-02:11"
      ]
    }
  ],
  "tags": [
    {
      "tag": "invoice",
      "reason": "three mentions"
    }
  ],
  "relations": [
    {
      "kind": "related",
      "target": "2026-08-19T174030-6b1c0ddc4410",
      "reason": "same invoice number"
    }
  ]
}
```

The config comment at the key says plainly that the command receives the full transcript, so whether that
text leaves the machine is the operator's choice made at that line. Every string in the proposal is
rendered as data under the escaping rule of section 7.2.

vpt writes the analysis note from the proposal: frontmatter as in stage 3 with `vptStage: analysis`, a
`vpt:links` region, and a `vpt:content` region holding a `Summary` section and an `Actions` section. The
layout is fixed: one bullet per proposal line, in proposal order, each ending with its `[mm:ss-mm:ss]`
source reference and any verify-note annotation, and no headings of the agent's own. Tags and relations
go through the stage 3 gates.

### 8.2 verify-note

`vpt verify-note <path> --recording <id>` checks a Markdown note against the transcript and the flags. It
reports findings for any readable note, and it writes into a vpt-owned artifact only, which means all
three of: the path, resolved through no symbolic link, is the artifact path the ledger registers for
`--recording` (a path re-pinned by committed rename recovery, section 7.4, included); the note's
`vptRecording` and `vptStage` match that artifact; and its required markers validate. Any other readable
file is report-only: it is left byte-identical, its findings go to the output alone and no ledger state
changes. A file carrying a different `vptRecording` is refused, exit 3 `ownership`. Claims are read from
the `vpt:content` region (from the whole body of a file vpt does not own), excluding frontmatter,
headings, code fences, the links region and any earlier verify-note annotation. Every list item and every
sentence in a paragraph is a claim line. Four classes, in order:

1. `unsourced`: no `[mm:ss-mm:ss]` range on the line.
1. `bad-reference`: a range that is empty, inverted, or outside the transcript's bounds.
1. `unsupported`: the claim's digit runs, money amounts, dates and capitalized tokens do not appear in
   the cited span or within `[synthesis] grounding_window_secs` (default 15) of it. Paraphrase is not
   checked; that needs a model, and a model checking a model's summary reintroduces the error class this
   exists to catch.
1. `built-on-flagged-text`: an open surfaced lexical flag of any class (`numeric`, `proper-noun`,
   `low-confidence`, `agreed-unverified` or `other`) intersects a cited source range. The claim inherits
   that uncertainty, and its source ranges and inherited uncertainty are persisted with the accepted
   proposal (section 4.4), which is what analysis redaction reads (section 8.5).

A finding is written as an annotation at the end of its claim line, inside the content region:
`[unsourced]`, `[bad-reference]`, `[unsupported: <token>]` or `[built-on-flagged-text]`. A finding is the
third flag shape of section 6.4, identified by the artifact, the claim's index in the region and the
SHA-256 of the claim text with its annotations stripped, so a changed claim never inherits a resolution;
its source range is null for `unsourced` and `bad-reference`. Each invocation replaces only the selected
artifact's finding set, keeps a resolution only for an identifier that is unchanged, and recomputes the
recording's `open_flags` from the current transcript flags and the findings of every artifact the ledger
owns for that recording. Findings alone exit 0; a read or write failure exits 1. `vpt synthesize` runs
`verify-note` on the analysis note it just wrote before it notifies.

### 8.3 Briefs

A brief is an assembled, cited evidence pack for one occasion. vpt writes no sentences of its own in it:
every quoted line is a verbatim transcript span with its recording identity and timecode.

```
vpt brief <occasion-id> [--json] [--explain] [--dry-run]
vpt brief --title <t> --at <rfc3339> [--duration-minutes <n>] [--participant <name>]... [--tag <t>]...
vpt brief --upcoming [--json]
vpt occasions [--json]
```

An occasion has an identity, `<local date and time>-<12 hex>`, the hash over a canonical key that is
exactly one of: provider-anchored (`dam` plus the object id, or `google` plus the calendar id and the
event or occurrence id), or manual (the RFC 3339 start plus the sanitized title). A provider occasion
keeps its identity when its title changes; a manual one is reproducible from what was typed. A provider
occasion is looked up by its canonical provider key, unique in the ledger, before an identity is derived,
so a rescheduled event keeps its identity and its file. Re-running a brief rewrites one file rather than
creating a second. A brief's file is named `{occasion-date}-{slug}-{hash8}.md` from the occasion's own
date, sanitized title and identity hash, pinned at first write.

Candidates are recordings that have a transcript artifact, deduplicated by identity. Selection is four
exact selectors over confirmed data, capped at `[brief] max_notes` (default 12), ordered by selector rank
then capture time newest first, the remainder counted and reported:

| Selector      | Matches a note when                                                                                                                                          |
| ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `participant` | a confirmed known term for the participant's name resolves to exactly one indexed note, and the recording carries a `mentions` relation to that note         |
| `term`        | a confirmed known term in the occasion title resolves to exactly one indexed note, and the recording carries a `mentions` relation to that note              |
| `tag`         | the recording carries a confirmed tag that `occasion.tags` also carries (the `--tag` values or the provider's labels, persisted on the occasion)             |
| `recent`      | the recording was captured inside `[brief] lookback_days` (default 180) and shares a `continues` chain or a confirmed tag with an already selected recording |

Quoted spans come from the accepted transcript: for `participant` and `term`, the segments containing the
term, in start-time order; for `tag` and `recent`, segments in start-time order. `max_spans_per_note` is
applied after deduplication and the omitted segments are counted. A participant with no confirmed term
selects nothing and appears in a `Not selected` section with the `vpt confirm --term` command that would
fix it. `--explain` prints every candidate, its selector and the matched value, rejected ones included,
and writes nothing.

The brief's Markdown carries `vptKind: brief`, `vptOccasion`, `vptOccasionAt`, `vptSource`, `vptNotes`,
`vptUnresolved`, `vptContext`, a managed `vpt:brief` block with `Occasion`, `Unresolved`,
`From your notes` (at most `[brief] max_spans_per_note`, default 5, per note), `Open tasks` and
`Not selected` sections. A line resting on a flagged span carries `[unverified]` in place, inside the
line, as well as being counted, because a consumer that quotes one bullet drops a footer and keeps the
sentence. An empty section says so; it is never omitted.

`vpt brief --json` prints `vpt.brief/1`: `occasion` (id, at, duration_secs, title, source, participants
with `matched_note` and `term` state), `unresolved` (flags, notes_with_flags, reviewed), `items` (each
with `text`, `certainty` of exactly `confirmed` or `unverified`, `sources` of recording, `at_secs` and
note, and `selector`; an item without `certainty` and `sources` fails to serialize), `not_selected`,
`context` (`calendar` and `tasks`, each
`{status: present|absent, reason: null|disabled|unsupported|failed, detail}`), and `generated_at`. This
is the consumer contract for an assistant reading vpt.

A requested brief notifies nothing. The calendar trigger is `[brief.trigger] enabled` (default false)
with `lead_time` (default `24h`): `vpt brief --upcoming` asks the context source for occasions starting
inside the lead time, writes a brief for each not yet briefed (by occasion identity), and delivers one
`vpt.event/1` (`event = "brief_written"`, occasion identity, counts, path) per brief written. A scheduler
outside vpt runs `--upcoming` on an interval; with the trigger disabled the flag refuses with exit 2
naming the key. A brief is never a redaction source in this version (section 8.5) and never automatically
handed off.

### 8.4 Calendar and task context

`[context] type` is `none` (default), `dam` or `google`. The context source answers two questions,
occasions in a window and open tasks, and every answer is normalized into one `vpt.context/1` document
(`occasions` with source, id, title, at, duration_secs, participants of name and email; `tasks` with
source, project, id, content, due, completed, priority) before anything reads it.

Selection is the only scope that exists, because no provider offers a per-calendar or per-project
credential: `[context] calendars` and `[context] projects` are lists, an empty list refuses at startup
naming the key whenever the selected provider supports that half and the verb asks for it, and the
selection is in the request, never a filter afterwards.

- `dam`: vpt spawns `[context.dam] command` (default `["dam"]`) as
  `dam ls "kind:event & path:<calendar> & start:<YYYY-MM-DD>" --json --no-pull` once per configured
  calendar path and per day in the window, and
  `dam ls "kind:task & path:<project> & !done" --json --no-pull` once per configured project. Each
  calendar is a dam path prefix (the `path` a Google Calendar remote is mounted at in dam's config, such
  as `calendar/`), each project a task path prefix. dam's `attendees` become participants by email, its
  `organizer` supplies a name when present, `subject` is the title, `start` and `end` give `at` and
  `duration_secs`. `--no-pull` is always passed so a brief never triggers a remote pull. A non-zero exit
  or an error document on stderr is a collector failure.
- `google`: a native reader over the Google Calendar events list endpoint with `singleEvents=true` and
  `timeMin` and `timeMax` bracketing the window, per configured calendar id, following `nextPageToken`
  until exhausted within the same scope; because `timeMin` filters on end time, vpt applies the trigger
  predicate `now <= start < now + lead_time` itself to the returned occurrences, and a collection that
  exceeds its deadline or a page limit is reported incomplete and marks no occasion briefed. It is
  authenticated with `[context.google] client_id`, `client_secret` and `refresh_token` (values in the
  config file) exchanged for an access token bearing only the `calendar.events.readonly` scope; a token
  reply that grants any other scope is refused. It answers no tasks. TLS through `rustls`, no system
  OpenSSL.
- `none`: both halves absent with reason `disabled`.

Each half of the context is `present` or `absent` with a reason: `disabled` (`type = "none"`, or the half
not requested), `unsupported` (Google supplies no tasks), or `failed` with a bounded detail (binary
missing, non-zero exit, network error, malformed answer, deadline). A manual brief survives a collector
failure: the brief is still written and the status appears in the `Occasion` section and in `context`.
`brief --upcoming` exits 1 when occasion discovery fails, because it has nothing to brief. Fetched text
is untrusted: titles go through the slug sanitizer before any path use, and every fetched string is
rendered as quoted data with its source named.

### 8.5 Redaction and sharing

Sharing is redaction at share time, into a folder the operator sends from. There is no approval step, no
expiry, no transmission and no cloud service.

```
vpt redact <id> [--stage transcript|analysis] [--to <dir>] [--title <t>] [--json]
```

Two ways to produce a copy: `[share] automatic = true` makes `vpt run` redact every recording's
transcript note as soon as it is written, with synthesis configured or not, and its analysis note as soon
as that artifact exists; `vpt redact` does it on demand for one recording. Both write into
`[stores] released`, and `--to <dir>` names a different destination for that run. The destination may not
resolve inside a private store (`audio`, `transcripts`, `analysis`, `briefs`, `engine_outputs`,
`drafts`), the state directory, the config directory or the Voice Memos container (a refusal naming
which); any other directory is allowed, including one that syncs elsewhere, because sending is the point.

The released file is assembled, not filtered. Its frontmatter carries at most `title` (from `--title`,
else absent), `date` (when `[share] include_date`), and `source` (`[share] source_line`, default
`redacted extract, not a verbatim record`, empty disables). None of the `vpt` keys, `tags` or the vault's
keys cross. The links region is discarded, the content region's markers are removed and the text inside
it is what the pass works on; sections named in `[share] deny_sections` (default `[]`) are dropped from
it. Every link target is removed; link text survives only if it survives the removal pass. Timecodes are
removed unless `[share] keep_timecodes`.

The removal pass is deterministic and pure: confirmed terms from `known-terms.txt` and note names and
aliases from the index are always masked, whole-token, case-insensitive, possessives handled, no fuzz;
the pattern classes in `[share.redact] classes` (default `email`, `url`, `phone`, `number` of four or
more digits, `money`) are masked; each removed value becomes a mask label from `[share.redact] mask`
(default `[{class} {n}]`), numbered from 1 in every copy with no mapping kept from one copy to the next.
The filenames and the surviving content still let a recipient holding two copies of one recording relate
them, and vpt claims nothing else. A span with an open lexical flag is omitted and counted by default
(`[share.redact] flagged_spans = "omit"`); `"mark"` keeps it with its `[unverified]` marker, which the
pass may not strip. In an analysis copy the unit is the claim: `flagged_spans` applies to every claim
carrying `built-on-flagged-text` (section 8.2), `omit` removing and counting the whole claim and `mark`
keeping it with its `[unverified]` annotation, and a transcript word offset is never applied to
synthesized text.

The pass runs over parsed Markdown: character references are decoded first; HTML, comments, reference
definitions and every link destination are discarded; retained text is re-escaped on render. Candidate
matches are resolved longest first, ties in the order confirmed term, note name, `email`, `url`, `phone`,
`money`, `number`. `email` is a whitespace-delimited token containing `@`; `url` a token beginning with a
scheme and `://` or with `www.`; `phone` a run of at least seven digits separated only by spaces,
parentheses, plus signs, periods or hyphens; `number` at least four consecutive digits; `money` a
currency symbol adjacent to a digit run. The residue scan runs over the same decoded representation.

The residue scan runs on the assembled bytes: no real value the pass removed, no confirmed term and no
note name may appear as a whole token. Residue is a refusal, exit 3, naming the mask label and never the
value; the copy is not written. The private report (mode 0600, named above) maps every mask label to its
real value and every released line to its source span, and a candidate report lists every capitalized
non-sentence-initial token that survived, as a review prompt the operator reads with `--json` or in the
run output. Confirming one of those tokens with `vpt confirm --term` masks it in every future copy.

Names in the destination are standardized: `<date>-<stage>-<hash8>.md`, never the slug, because a title
survives a perfect body redaction by riding in the filename. Before writing, vpt digests every regular
file directly in the destination and compares each with the bytes it would write, whatever its name: any
identical file is a refusal, exit 3 `duplicate`, naming that file. When the standard name exists with
different content, vpt writes `<date>-<stage>-<hash8>-<YYYYMMDDThhmmss>.md` instead (the moment of
writing, local time) and reports the collision; a further collision on that name appends `-2`, `-3` and
so on, the smallest unused suffix. A released file is never overwritten. The comparison and the write
happen under the write lock of section 4.4, so two concurrent runs cannot both write. The source note is
byte-identical before and after.

Every release is an immutable row in the ledger's `releases` repository: recording, source stage,
absolute destination, content digest, source-artifact digest, creation sequence, report path and the
provenance snapshot of the source artifact at that moment (capture instant, engines, open lexical flags,
the complete `open_flags` count including verification findings, diagnostics and reviewed state). The
creation sequence is allocated in the ledger, unique and durable, before the copy is published. The
private report is named `drafts/<id>.<stage>.<creation-sequence>.report.json` and records both the
released-content digest and the source-artifact digest, so every release keeps its own mask map even when
another release of the same recording has byte-identical redacted text.

Audio is never a redaction source. A brief is not a redaction source in this version. The released file
does not name vpt.

### 8.6 The Open Notebook handoff

```
vpt handoff <id> --stage released|transcript|analysis|brief [--json]
```

A pure function from (record, artifact bytes, stage) to one `vpt.handoff/1` document on stdout. It opens
no socket, resolves no name, reads no credential and writes no file:

```json
{
  "schema": "vpt.handoff/1",
  "kind": "released",
  "identity": {
    "recording": "2026-08-24T144736-4f3ab19c02de",
    "note": "released/2026-08-24-transcript-4f3ab19c.md",
    "content_sha256": "9f2c..."
  },
  "title": "vpt released 2026-08-24-transcript-4f3ab19c",
  "content": "# vpt released copy, 2026-08-24-transcript-4f3ab19c\n\n...",
  "provenance": {
    "produced_by": "vpt",
    "captured_at": "2026-08-24T14:47:36-06:00",
    "engines": [
      "apple:SpeechTranscriber"
    ],
    "open_flags": 0,
    "reviewed": true,
    "redacted": true
  },
  "generated_at": "2026-09-20T22:40:00-06:00"
}
```

`kind` has four values and a fifth is a compile-time impossibility. By default only `released` is
permitted: `--stage transcript|analysis|brief` is refused with exit 3 unless
`[handoff] allow_private = true`. `--stage released` selects the recording's newest release by creation
sequence whose file still exists and verifies its digest; no release is exit 2, and a file whose bytes
changed since release is exit 3 `artifact_changed`. Its `provenance` is the release row's snapshot, and
its `content` is rendered from the immutable released body: the released file's frontmatter is dropped,
the fixed declarative header is prepended, and the body follows byte for byte. Neither half is
reconstructed from the recording's current transcription or review state, so a later re-transcription or
flag resolution never relabels an older copy. `content` is text always; the schema has no field that can
carry audio, and an identifier that resolves to a recording rather than to a rendering is refused listing
the stages that exist. The note's frontmatter does not cross; the fixed declarative header carries the
identity, the unresolved count (the snapshot's `open_flags` total for a release) and the derived-copy
sentence, with no imperative sentence in it. An unreviewed artifact is emitted, labelled, never refused.
The header names vpt. The function is pure apart from `generated_at`, so the digest is meaningful and a
lost copy is recoverable by re-running one command.

No configuration key and no production source line names Open Notebook's vocabulary: a test greps the
shipped Rust and Swift sources and the shipped configuration template (documentation, tests and fixtures
excluded) for `open.notebook`, `notebook_id`, `5055`, `8502` and `surreal` and finds nothing. The mapping
from `vpt.handoff/1` to the notebook's own source-creation call, the credential holder, the transport
recipe, the return path for a note authored there, and a remote identifier that would let a re-handoff
replace rather than duplicate, all wait until that service is deployed; a re-handoff today duplicates,
and the deterministic `title` is what makes the duplicate findable.

### 8.7 Notifications through the producer API

`[notify] mode` is `desktop`, `command` or `off`, default `desktop`. Every event vpt raises is one
`vpt.event/1`:

```json
{
  "schema": "vpt.event/1",
  "event": "review_needed",
  "state": "needs_attention",
  "recording": "2026-08-24T144736-4f3ab19c02de",
  "detail": "6 spans to review, 11 agreed spans unverified",
  "counts": {
    "numeric": 3,
    "proper_noun": 2,
    "low_confidence": 4,
    "unsupported": 1,
    "agreed_unverified": 11,
    "diagnostics": 0
  },
  "paths": {
    "note": "<transcripts store>/2026-08-24-invoice-call-4f3ab19c.md"
  },
  "occurred_at": "2026-09-20T22:40:00Z"
}
```

Event names: `review_needed`, `transcribe_failed`, `ingest_failed`, `deferred`, `brief_written`,
`retention`, `config_refused`. `state` is `needs_attention`, `failed` or `done`. `counts` carries one
entry per lexical class plus `diagnostics`. An event carries identities, counts, classes and paths, and
never a transcript span, an alternative, a title, a tag, a mask value or a brief line.

- `desktop`: `vpt-macos notify` with a title of `vpt: <event>` and the `detail` as body.
- `command`: vpt runs `[notify] command` (an argv list) with the tokens `{event}`, `{state}`, `{id}`,
  `{detail}`, `{count}` and `{path}` substituted into the argument list, and writes the event document on
  the command's stdin, both, on every invocation, with no sub-mode key. The command's exit code is
  load-bearing: non-zero means vpt attempts its desktop notice once as a fallback, logs the exit code,
  and keeps the work's own exit status. The config comment at the key states that the command receives
  vpt's JSON (not any downstream tool's), and that a wrapper must pass through its final delivery's exit
  code.
- `off`: nothing is raised.

A notification failure never fails the work it reports on: the note, the flags and the clone are already
written when the event is raised. vpt's source contains no name of any notification engine.

## 9. The command-line surface

Every verb accepts `--json`. Machine-readable output is withheld until the final status is known: on
success one document goes to stdout; on failure one error document goes to stderr, stdout stays empty,
and the error carries a `completed` field listing the work already committed (for example the recordings
a sweep ingested before it aborted). A result with no schema of its own carries `schema: "vpt.result/1"`
and `command: "<verb>"`; an error carries `schema: "vpt.error/1"`. Without `--json`, success prints human
lines and failure prints `vpt: <message>`. `--config <file>` overrides the config path; `VPT_CONFIG` does
the same. An unknown argument or subcommand prints usage to stderr and exits 2, as an error document
under `--json`.

| Verb                                                                                          | Writes                                                                                                                                | `--json` shape                                                                                                        | Exit codes                            |
| --------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- | ------------------------------------- |
| `vpt setup [--force]`                                                                         | `config.toml` (the main engine prompted for, every other key at its default), the home, the state directory, the default store leaves | `{"written": "<path>", "main_engine": "<name>"}`                                                                      | 0, 2 exists or no terminal            |
| `vpt doctor`                                                                                  | nothing                                                                                                                               | `{"checks": [{"name", "ok", "detail"}]}` when all pass; `vpt.error/1` with the same array in `error.checks` otherwise | 0 all ok, 3 any failed                |
| `vpt run [--dry-run]`                                                                         | everything a stage writes, for new work                                                                                               | `{"ingested": [..], "transcribed": [..], "notes": [..], "synthesized": [..], "redacted": [..], "retention": {..}}`    | 0, 1 any stage failed                 |
| `vpt ingest [--dry-run] [--once <path>]`                                                      | clones, ledger rows                                                                                                                   | `{"ingested": [record...], "deferred": [{"path", "reason"}], "skipped": n}`                                           | 0, 1 aborted                          |
| `vpt transcribe <id> [--language <tag>] [--dry-run]`                                          | engine outputs, flags, transcript note                                                                                                | the record plus `{"flags": {"<class>": n}}`                                                                           | 0, 1 engines failed, 3 refused        |
| `vpt review <id> [--resolve <flag> --confirm\|--correct <t>\|--dismiss]`                      | flag state, note rewrite, known-terms                                                                                                 | `{"flags": [{"id", "class", "at", "record", "alternative", "state"}]}`                                                | 0, 2 no such flag                     |
| `vpt confirm --term <t>` / `vpt confirm <id> --tag <t>\|--reject-tag <t>\|--relation <k>:<t>` | the two lists, ledger, note rewrite                                                                                                   | `{"confirmed": {...}}`                                                                                                | 0, 2                                  |
| `vpt note write <id>`                                                                         | re-renders managed regions of both notes                                                                                              | the record                                                                                                            | 0, 3 markers                          |
| `vpt path <id> --stage <stage>` / `vpt path --occasion <id> --stage brief`                    | nothing                                                                                                                               | `{"path": "<abs>"}`                                                                                                   | 0, 2                                  |
| `vpt synthesize <id> [--dry-run]`                                                             | analysis note, tags, relations                                                                                                        | the record plus `{"verify": {"<class>": n}}`                                                                          | 0, 1 command failed, 2 not configured |
| `vpt verify-note <path> --recording <id>`                                                     | the owned artifact's annotations                                                                                                      | `{"flags": [{"class", "line", "range"}]}`                                                                             | 0, 1, 3                               |
| `vpt brief <occasion> [--explain] [--dry-run]` / `--title --at ...` / `--upcoming`            | brief note, occasion row, events                                                                                                      | `vpt.brief/1`, or `{"written": [..]}` for `--upcoming`                                                                | 0, 2, 3                               |
| `vpt occasions`                                                                               | nothing                                                                                                                               | `{"occasions": [..]}`                                                                                                 | 0                                     |
| `vpt redact <id> [--stage] [--to <dir>] [--title <t>]`                                        | released copy, draft report                                                                                                           | `{"written": "<path>", "report": "<path>", "masks": {"<class>": n}, "candidates": [..]}`                              | 0, 3 duplicate or residue             |
| `vpt handoff <id> --stage <kind>`                                                             | nothing                                                                                                                               | `vpt.handoff/1`                                                                                                       | 0, 2, 3 private refused               |
| `vpt show <id>` / `vpt list [--stage <s>]`                                                    | nothing                                                                                                                               | the record / `{"recordings": [..]}`                                                                                   | 0, 2                                  |
| `vpt storage`                                                                                 | nothing                                                                                                                               | `{"stores": {"<key>": {"files", "bytes", "oldest", "newest"}}}`                                                       | 0                                     |
| `vpt retention run [--dry-run]`                                                               | moves to Trash                                                                                                                        | `{"moved": [{"store", "path"}], "kept": [{"path", "reason"}]}`                                                        | 0, 3 helper absent                    |
| `vpt symlink deploy` / `vpt symlink verify`                                                   | the link / nothing                                                                                                                    | `{"link", "target", "ok"}`                                                                                            | 0, 3                                  |
| `vpt --version`                                                                               | nothing                                                                                                                               | `{"version", "helper_version"}`                                                                                       | 0                                     |

Exit codes, one mapping for every verb:

| Code | Meaning                                                                                                                              |
| ---- | ------------------------------------------------------------------------------------------------------------------------------------ |
| 0    | the command did what it was asked                                                                                                    |
| 1    | vpt failed: an engine, the helper, a command it spawned, the filesystem or the ledger                                                |
| 2    | the input was wrong: an unknown argument, a config value vpt refuses to read, an unknown id, a verb that needs a key that is not set |
| 3    | vpt refused by one of its own rules, and the message names the rule                                                                  |

The error document:

```json
{
  "schema": "vpt.error/1",
  "error": {
    "kind": "refused",
    "rule": "same_family",
    "message": "engines apple and whisply both report family whisper",
    "ids": [],
    "completed": []
  }
}
```

`kind` is one of `refused`, `usage`, `config`, `engine`, `helper`, `command`, `store`, `ledger`; `rule`
names the rule for `refused` and is null otherwise; `ids` lists the identities the message names, in
order; `completed` lists the identities of work committed before the failure; `checks` appears only for
`doctor_checks` and carries `doctor`'s complete check array. Every rule vpt keeps reports code 3 and
nothing else does.

`--dry-run`, wherever a verb offers it, opens existing state read-only and performs no migration,
directory creation, ledger write, artifact write, notification or Trash move. `run`, `transcribe` and
`synthesize` spawn no engine and no agent command under it and report the operations they would perform;
`brief --dry-run` may collect configured read-only context and prints the pack it would write;
`ingest --dry-run` neither creates nor refreshes the title copy and creates no durable state.

## 10. Configuration

One file, `~/.config/vpt/config.toml`. `vpt setup` needs a controlling terminal: it prompts for the main
engine with `apple` (the local option) preselected, writes that selection, writes every other key
uncommented at its default with one concise comment so the file shows the real posture, creates the home
and the state directory with mode 0700 and the default store leaves beneath the home, and refuses to
overwrite an existing file without `--force`. Without a terminal it exits 2 and writes nothing; with
`--json` the prompt still uses the terminal and stdout carries only the result document. A secret is a
value in this file; a key holding a secret is marked in the table. A key that is not in this table is a
startup refusal naming it. The file's first key is `config_version = 1`, an integer `vpt setup` emits: a
missing or unsupported version is a configuration error, exit 2, which `doctor` reports as a failed
check, and a supported older version, once one exists, is read only through an explicit, tested
migration.

Value rules: a ratio is a finite number in `[0, 1]`, and `max_divergence_ratio` is above zero; a timeout,
a threshold and the slug length are positive integers; a count, a gap and a lookback are non-negative
integers; every integer fits 32 bits except `source.max_audio_bytes`, which fits 64. A duration is `0` or
a positive decimal integer followed by one of `s`, `m`, `h`, `d`, converted to seconds with overflow
checked. A value outside its rule is exit 2 naming the key.

A secret value has a redacted `Debug` and `Display` form. No log line, error document, notification,
trace or test failure ever contains a secret value, an authorization header, a token body, the full
configuration text or raw child output; a diagnostic names the key, the operation, the exit status and a
structural location, never the rejected value. Each failure path that handles a secret is covered by a
canary-secret test.

| Key                                  | Type           | Default                                                                   | Meaning                                                                                                      |
| ------------------------------------ | -------------- | ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `config_version`                     | int            | `1`                                                                       | the configuration schema version; missing or unsupported refuses                                             |
| `home.path`                          | string         | `~/.vpt`                                                                  | the home; every store defaults under it                                                                      |
| `home.state_dir`                     | string         | `~/.local/state/vpt`                                                      | the ledger's directory                                                                                       |
| `home.symlink_target`                | string         | `""`                                                                      | when set, `~/.vpt` is a managed symlink to this directory and `home.path` must stay at its default           |
| `helper.path`                        | string         | `vpt-macos`                                                               | the macOS helper, resolved on `PATH` when relative                                                           |
| `stores.audio`                       | string         | `<home>/audio`                                                            | archive clones                                                                                               |
| `stores.transcripts`                 | string         | `<home>/transcripts`                                                      | transcript notes                                                                                             |
| `stores.analysis`                    | string         | `<home>/analysis`                                                         | synthesis notes                                                                                              |
| `stores.briefs`                      | string         | `<home>/briefs`                                                           | briefs                                                                                                       |
| `stores.engine_outputs`              | string         | `<home>/engine-outputs`                                                   | raw engine documents                                                                                         |
| `stores.drafts`                      | string         | `<home>/drafts`                                                           | private redaction reports                                                                                    |
| `stores.released`                    | string         | `<home>/released`                                                         | the shared folder                                                                                            |
| `source.recordings_dir`              | string         | `~/Library/Group Containers/group.com.apple.VoiceMemos.shared/Recordings` | Apple's store                                                                                                |
| `source.read_titles`                 | bool           | `true`                                                                    | read titles from a private copy of the database                                                              |
| `source.quiet_period_secs`           | int            | `30`                                                                      | seconds a file must be unchanged before it is at rest                                                        |
| `source.deferral_page_threshold`     | int            | `4`                                                                       | consecutive deferrals of one recording before one event                                                      |
| `source.max_audio_bytes`             | int            | `2147483648`                                                              | a larger source file defers with `audio_too_large` before any content is read                                |
| `recording.language`                 | string         | `en-US`                                                                   | the language engines are asked for                                                                           |
| `engines.main`                       | string         | `apple`                                                                   | the transcript of record                                                                                     |
| `engines.checker`                    | string         | `""`                                                                      | the second slot; empty runs one engine                                                                       |
| `engines.checker_runs`               | enum           | `always`                                                                  | `always` or `on_failure`                                                                                     |
| `engines.allow_same_family`          | bool           | `false`                                                                   | let two engines of one family run                                                                            |
| `engines.timeout_secs`               | int            | `1800`                                                                    | per-engine wall-clock deadline                                                                               |
| `engines.by_language.<lang>.main`    | string         | none                                                                      | main engine for one language                                                                                 |
| `engines.by_language.<lang>.checker` | string         | none                                                                      | checker for one language                                                                                     |
| `engines.apple.kind`                 | enum           | `apple`                                                                   | the Apple Speech adapter through the helper                                                                  |
| `engines.whisply.kind`               | enum           | `whisply`                                                                 | the whisply adapter                                                                                          |
| `engines.whisply.command`            | string list    | `["whisply"]`                                                             | how whisply is invoked                                                                                       |
| `engines.whisply.device`             | string         | `mlx`                                                                     | `mlx` reports no confidence, `cpu` does                                                                      |
| `engines.whisply.model`              | string         | `large-v3-turbo`                                                          | the model name                                                                                               |
| `engines.<name>.kind`                | enum           | none                                                                      | `command` for a generic engine                                                                               |
| `engines.<name>.command`             | string list    | none                                                                      | argv with `{audio}`, `{language}`, `{out_dir}` tokens                                                        |
| `engines.<name>.family`              | string         | none, required for `command`                                              | the model family for the same-family check                                                                   |
| `engines.<name>.local`               | bool           | none, required for `command`                                              | whether audio stays on the machine                                                                           |
| `reconcile.confidence_floor`         | float          | `0.35`                                                                    | word confidence below this is flagged                                                                        |
| `reconcile.max_divergence_ratio`     | float          | `0.35`                                                                    | past this edit-distance ratio, one `divergent` flag                                                          |
| `reconcile.known_terms_path`         | string         | `~/.config/vpt/known-terms.txt`                                           | the confirmed-terms list                                                                                     |
| `readability.enabled`                | bool           | `false`                                                                   | the readability pass over the note body                                                                      |
| `note.profile`                       | enum           | `portable`                                                                | `portable` or `obsidian`                                                                                     |
| `note.link_style`                    | enum           | `markdown`                                                                | `markdown` or `wiki`                                                                                         |
| `note.slug_max_chars`                | int            | `60`                                                                      | slug length cap                                                                                              |
| `note.obsidian.hub_transcripts`      | string         | `transcripts`                                                             | folder note name for `hub`                                                                                   |
| `note.obsidian.hub_analysis`         | string         | `analysis`                                                                | folder note name for `hub`                                                                                   |
| `note.obsidian.hub_briefs`           | string         | `briefs`                                                                  | folder note name for `hub`                                                                                   |
| `note.obsidian.vault_root`           | string         | `""`                                                                      | the vault's root directory, required by the `obsidian` profile; the tag vocabulary and note index cover it   |
| `tags.new_tags`                      | enum           | `hold`                                                                    | `write` (automatic) or `hold` (suggested until confirmed)                                                    |
| `tags.known_tags_path`               | string         | `~/.config/vpt/known-tags.txt`                                            | the confirmed-tags list                                                                                      |
| `tags.max_per_note`                  | int            | `5`                                                                       | confirmed tags per note                                                                                      |
| `tags.max_suggested`                 | int            | `10`                                                                      | held tags per note                                                                                           |
| `relations.session_gap_minutes`      | int            | `60`                                                                      | `continues` window                                                                                           |
| `relations.max_suggested`            | int            | `5`                                                                       | agent-proposed `related` links kept per note                                                                 |
| `synthesis.command`                  | string list    | `[]`                                                                      | the agent command; empty disables synthesis only (automatic redaction is `share.automatic`)                  |
| `synthesis.timeout_secs`             | int            | `600`                                                                     | the command's deadline                                                                                       |
| `synthesis.grounding_window_secs`    | int            | `15`                                                                      | verify-note's tolerance around a cited span                                                                  |
| `brief.lookback_days`                | int            | `180`                                                                     | how far back selection reaches                                                                               |
| `brief.max_notes`                    | int            | `12`                                                                      | selection cap                                                                                                |
| `brief.max_spans_per_note`           | int            | `5`                                                                       | quoted spans per selected note                                                                               |
| `brief.trigger.enabled`              | bool           | `false`                                                                   | the calendar trigger behind `--upcoming`                                                                     |
| `brief.trigger.lead_time`            | duration       | `24h`                                                                     | how far ahead `--upcoming` looks                                                                             |
| `context.type`                       | enum           | `none`                                                                    | `none`, `dam` or `google`                                                                                    |
| `context.calendars`                  | string list    | `[]`                                                                      | dam path prefixes or Google calendar ids; empty refuses                                                      |
| `context.projects`                   | string list    | `[]`                                                                      | dam task path prefixes; empty refuses                                                                        |
| `context.dam.command`                | string list    | `["dam"]`                                                                 | how dam is invoked                                                                                           |
| `context.google.client_id`           | string, secret | `""`                                                                      | OAuth client id                                                                                              |
| `context.google.client_secret`       | string, secret | `""`                                                                      | OAuth client secret                                                                                          |
| `context.google.refresh_token`       | string, secret | `""`                                                                      | a refresh token bearing only the read-only events scope                                                      |
| `share.automatic`                    | bool           | `false`                                                                   | redact every recording's notes during `vpt run`                                                              |
| `share.source_line`                  | string         | `redacted extract, not a verbatim record`                                 | the one frontmatter line telling a recipient what they hold                                                  |
| `share.include_date`                 | bool           | `false`                                                                   | put the capture date in the released frontmatter                                                             |
| `share.keep_timecodes`               | bool           | `false`                                                                   | keep `[mm:ss]` references in released text                                                                   |
| `share.deny_sections`                | string list    | `[]`                                                                      | sections dropped from the content region before redaction                                                    |
| `share.redact.classes`               | string list    | `["email", "url", "phone", "number", "money"]`                            | pattern classes; terms and note names are always masked                                                      |
| `share.redact.mask`                  | string         | `[{class} {n}]`                                                           | rendered in place of a removed value                                                                         |
| `share.redact.flagged_spans`         | enum           | `omit`                                                                    | `omit` or `mark`                                                                                             |
| `handoff.allow_private`              | bool           | `false`                                                                   | let transcripts, analysis notes and briefs be handed off                                                     |
| `notify.mode`                        | enum           | `desktop`                                                                 | `desktop`, `command` or `off`                                                                                |
| `notify.command`                     | string list    | `[]`                                                                      | argv with `{event}`, `{state}`, `{id}`, `{detail}`, `{count}`, `{path}` tokens; receives vpt's JSON on stdin |
| `notify.aggregate_after`             | int            | `3`                                                                       | recordings needing review in one run before one aggregate event                                              |
| `retention.enabled`                  | bool           | `false`                                                                   | the opt-in retention run                                                                                     |
| `retention.include_audio`            | bool           | `false`                                                                   | let the audio store expire                                                                                   |
| `retention.hold.<store>`             | duration       | `0` for every store                                                       | files older than this move to the Trash; `0` never                                                           |

### 10.1 The 28 low-priority defaults, for the operator to strike at review

Each row is a reversible value the design tree left open and this spec set. Strike a row to change it;
the node number is the tree's.

| Node  | Default set here                                                                              |
| ----- | --------------------------------------------------------------------------------------------- |
| 1.2   | `vpt ingest` is an idempotent sweep run by an outside scheduler; no daemon, no watch paths    |
| 1.3   | the scheduler's interval is 15 minutes (a plist value, not a vpt key)                         |
| 1.5.1 | the identity's timestamp is local time; UTC with offset in the ledger                         |
| 1.5.2 | an edited or trimmed recording is a new recording                                             |
| 1.6.1 | `source.quiet_period_secs = 30`                                                               |
| 1.11  | a recording deleted in Voice Memos is marked `source_gone_at`, reported by `doctor`, no event |
| 1.12  | `source.deferral_page_threshold = 4`                                                          |
| 3.14  | no transcode before a cloud upload; a `command` engine receives the archived path as is       |
| 3.15  | engines run sequentially                                                                      |
| 3.16  | WhisperX is reachable through the `command` kind and is not a first-class adapter             |
| 4.6   | `reconcile.confidence_floor = 0.35`                                                           |
| 4.9   | `reconcile.max_divergence_ratio = 0.35` and one whole-recording `divergent` flag              |
| 4.11  | a re-run preserves resolutions by flag identifier and adds only new flags                     |
| 5.8   | four relation kinds, closed: `continues`, `mentions`, `related`, `supersedes`                 |
| 5.10  | vpt never writes into a note it did not create                                                |
| 5.12  | note names are `{date}-{slug}-{hash8}`, slug from the Voice Memos title only                  |
| 5.14  | filing is flat, one store per stage, no rule table                                            |
| 8.12  | the artifact keeps the name `brief`; `vpt brief` is the verb                                  |
| 9.7   | mask numbering restarts at 1 in every released copy                                           |
| 9.10  | a released file does not name vpt                                                             |
| 9.12  | the shared folder defaults to `<home>/released`                                               |
| 10.8  | one event per recording, one aggregate event past `notify.aggregate_after = 3`                |
| 10.9  | which route a configured command posts to is the command's business, not vpt's                |
| 12.4  | secrets are values in `config.toml`; rendering them is the operator's concern                 |
| 14.6  | vpt installs to `~/.cargo/bin`                                                                |
| 14.8  | vpt ships no bash; the only non-Rust code is the Swift helper                                 |
| 15.6  | an unreviewed artifact is handed off labelled, never refused                                  |
| 15.7  | the handoff header names vpt                                                                  |

### 10.2 An example profile: a home inside an Obsidian vault

A configuration for a home inside an Obsidian vault differs from the shipped defaults in these keys and
no others; it is an example, not the default. The operator's own values are rendered by their dotfiles
into `~/.config/vpt/config.toml`.

```toml
[home]
path = "~/notes/vpt" # inside the Obsidian vault, which ignores audio extensions

[stores]
engine_outputs = "~/.vpt/engine-outputs" # kept out of the vault
drafts = "~/.vpt/drafts"                 # the mask map never enters git

[note]
profile = "obsidian"
link_style = "wiki"

[note.obsidian]
vault_root = "~/notes"


[tags]
new_tags = "write"

[context]
type = "dam"
calendars = ["calendar/"]
projects = ["work/"]

[handoff]
allow_private = true

[notify]
mode = "command"
command = [
  "<a producer on PATH>",
  "send",
  "--producer",
  "vpt",
  "--event",
  "{event}",
  "--state",
  "{state}",
  "--detail",
  "{detail}",
]
```

## 11. Errors and refusals

vpt fails closed. A condition it cannot make safe is a refusal with the rule named, never a repair and
never a silent fallthrough.

Refused at startup, before any work, exit 2 or 3 as marked:

- config file missing (2, with the `vpt setup` command printed; `vpt setup` and `vpt --version` are the
  two verbs that run without one), unparseable (2), a missing or unsupported `config_version` (2),
  unknown key (2), a value of the wrong type or outside its range (2).
- a configured root that is relative after expansion or whose required parent does not exist; a store
  overlapping another store (the home may contain its stores); a writable store, staging path, state
  directory or release destination overlapping the Voice Memos container; a release destination
  overlapping a private store, the state directory or the config directory (2, naming the keys).
- `engines.main` naming an engine table that does not exist, an engine whose binary or helper is absent,
  or a `command` engine missing `family` or `local` (2).
- both engine slots reporting one family without `allow_same_family` (3, `same_family`).
- `context.type` set with the needed selection list empty, when a verb asks for that half (2).
- `notify.mode = "command"` with an empty command (2).
- the `obsidian` profile with `note.obsidian.vault_root` empty, relative or not an existing directory
  (2).
- the helper present with a different major version (3, `helper_version`).
- `brief --upcoming` with the trigger disabled (2); `synthesize` with no command (2); `handoff --stage`
  private without `allow_private` (3, `private_handoff`).

`vpt doctor` never refuses at startup: it runs every check, prints the whole result and exits 3 when any
failed, so a machine that cannot start still gets a diagnosis. Human output lists every check either way.
Under `--json`, all checks passing puts the complete `checks` array in a `vpt.result/1` document on
stdout; any check failing leaves stdout empty and puts a `vpt.error/1` document on stderr with
`kind = "refused"`, `rule = "doctor_checks"` and the complete array in `error.checks`. Every other verb
validates only what it needs: `vpt symlink deploy` validates its target and nothing else, `vpt storage`
needs the stores and no engine, and only a verb that will spawn an engine resolves one.

Refused during a run, the recording or artifact left as it was:

- a wholeness or rest gate (deferred, not refused); an unreadable or empty recordings store (1).
- a note whose managed markers are missing, doubled or unbalanced (3, `markers`).
- a target path carrying a different `vptRecording` or `vptOccasion` (3, `collision`).
- two notes claiming one identity (3, `ambiguous_note`).
- a resolved path escaping its store or traversing a symlink (3, `path_escape`).
- a `vptSchema` above the build's (3, `schema_ahead`, the note read and never rewritten).
- residue after redaction (3, `residue`); a byte-identical released duplicate (3, `duplicate`).
- a released destination inside a private store, the state directory, the config directory or the Voice
  Memos container (3, `private_destination`).
- retention with the helper absent (3, `no_trash`); a retention target whose content changed since its
  intent was recorded (3, `retention_target_modified`).
- an engine, agent or context command exceeding its deadline, exiting non-zero, or answering with a
  document of the wrong schema or major version (1 for engines and the agent; a collector failure
  degrades the brief and is reported).

Nothing vpt does deletes. The opt-in retention run moves durable artifacts to the Trash, and a failed
archive staging (section 5.3) moves its staged file to the Trash; with the helper absent such a file
stays where it is, mode 0600, and `doctor` reports it as `cleanup_pending`. vpt never unlinks and never
targets a file it did not write.

## 12. Testing

New behavior is written test-first without exception: the failing test first, seen to fail for the
intended reason, then the code. Unit tests live beside their implementation under `#[cfg(test)]`, in a
private `tests.rs` child when large. Every test finishes within one second under
`cargo test --workspace`. Mutation verification is by hand per behavior, against an unmutated control,
and its table goes in each pull request.

What the tests use instead of the world:

- **Fake engines.** A Rust test binary, `vpt-fake-engine`, behind `required-features = ["dev-tools"]` in
  the command crate, prints a `vpt.engine/1` document chosen by an argument (`--fixture <name>`,
  `--fail`, `--empty`, `--hang`), so the `command` adapter, the same-family refusal, the deadline kill
  and both-engines-fail are exercised with no engine installed. The Apple and whisply adapters are tested
  by pointing `helper.path` and `engines.whisply.command` at the fake.
- **A fake recorder store.** A fixture directory built by the test: the wholeness gate is tested over
  bytes the test assembles (`ftyp`, `mdat`, `moov` with an `mvhd` creation time, and the truncated and
  `moov`-less variants); the sweep is tested over a directory holding those files, the four Apple
  subdirectories, a `.waveform` sidecar, and a fixture `CloudRecordings.db` with the `ZCUSTOMLABEL` and
  `ZPATH` columns plus a `-wal`. `SF_DATALESS` is tested through the port with a fake that reports the
  flag, never by setting it on disk.
- **A fake helper.** The same dev-tools binary answers `transcribe`, `notify` and `trash` with recorded
  documents, so `desktop` notify and retention are tested without posting or moving anything.
- **A fixed clock**, an **in-memory ledger** that runs the same contract suite as the SQLite one, and
  **fake context commands** whose stdout is a fixture; the Google reader is tested against a local HTTP
  listener the test starts on a loopback port.
- **Temporary `HOME`.** Every binary run in a test sets `HOME`, `XDG_STATE_HOME` and `VPT_CONFIG` to a
  temporary directory. No test reads the operator's config, ledger, vault or Voice Memos store, and no
  test reaches the network except the loopback listener.

Behaviors that pin the constraints most likely to erode, written first in each stage: the source
container's every entry has the same size, mtime and flags after a full sweep, a dry run leaves the title
copy untouched, a publication interrupted after its rename and before its dirty entry clears is repaired
without a refusal, and an untracked file in a store survives a retention run (stage 1); two engines of
one family refuse before any spawn, and an agreed proper noun not in `known-terms.txt` produces exactly
one aggregated flag (stage 2); the portable profile emits none of the vault's keys and a full run against
a store with no `.obsidian` directory produces no vault syntax, and a rewrite preserves every byte
outside the markers (stage 3); the released file contains no removed value, no `vpt` key and no source
slug, a byte-identical duplicate is refused, `verify-note` leaves a note the ledger does not own
byte-identical, a proposal line holding a managed delimiter renders as text, `vpt handoff` writes no file
and succeeds with name resolution failing, and the grep for notebook vocabulary finds nothing (stage 4).

The Swift helper has its own `swift test` suite in `helper/vpt-macos/Tests`, and it reaches no real
destination: speech results are injected through a protocol the transcriber implements, the notification
poster is a stub, and the Trash is a temporary-directory adapter. The tests cover the conversion of
speech results into `vpt.engine/1` (schema, confidence in `[0, 1]`, non-inverted and contained ranges,
segment order), argument handling and exit codes, and the printed documents of `notify` and `trash`. A
real `SpeechAnalyzer` run and a real move to the Trash are operator-run smoke checks (`just smoke`),
never part of the automated suite.

CI runs on a macOS runner: `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace --no-fail-fast --features dev-tools`,
`RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`, `gitleaks git --no-banner` over the tree,
the file-size check (no handwritten `.rs` file above 500 lines, tests included, measured after `rustfmt`
with the standard's awk command; 300 total and 200 implementation as warnings), and
`swift build -c release` plus `swift test` in `helper/vpt-macos`. A `justfile` in the repository names
each gate as a recipe and `just ship` runs them all in CI order.

## 13. Packaging and deployment

- **The repository.** `github.com/webdavis/vpt`: the workspace, `helper/vpt-macos/`, `docs/` (this spec,
  the plans, decision records), `.github/workflows/ci.yml`, `justfile`, `rust-toolchain.toml` pinning
  `stable`.
- **Installing elsewhere.** `cargo install --git https://github.com/webdavis/vpt vpt` installs the `vpt`
  binary into `~/.cargo/bin`. The helper is built with `swift build -c release` in `helper/vpt-macos` and
  copied beside it; the README states both steps and `vpt doctor` reports a missing helper with those two
  commands. Without the helper, vpt still ingests, transcribes with whisply or a command engine, writes
  notes, briefs, redactions and handoffs.
- **The dotfiles builder.** The author's dotfiles repository builds from a local clone, the same shape as
  its scalebar builder: a data file declaring the clone's path and the install directory (`~/.cargo/bin`,
  the rust tools' declared home), one `run_onchange_after_*` script hashing the clone's manifests and
  sources, deferring with exit 0 and a retry marker when the clone or a toolchain is absent, running
  `cargo build --release --locked` and `swift build -c release`, and installing `vpt` and `vpt-macos`
  with `install -m 755`; a LaunchAgent plist running `vpt run` on a `StartCalendarInterval` every 15
  minutes plus `vpt brief --upcoming` when the trigger is enabled, with its loader; and a rendered config
  template carrying only what differs from the defaults plus the secret lookups. Those files are the
  dotfiles repository's, proposed in their own pull request; vpt's repository holds none of them and
  nothing in vpt reads them.
- **Runtime dependencies.** `whisply` and any `command` engine are the operator's to install; vpt
  declares nothing and spawns what config names.
- **Permissions.** The Voice Memos container is mode 0700 and owned by the user, so vpt needs no
  privilege. Whether a launchd-started job can read it under the privacy system is measured on the first
  scheduled run: `vpt doctor` reports `recordings_dir readable` and the plist's first run logs it. If it
  is refused, the same binary runs from a login shell or on demand and `doctor` says which world it is
  in.

## 14. Staged delivery

Four plans, one per stage, in the July order. Each ships a working tool: every verb it introduces is
complete, tested and documented, and `main` is installable after every merge.

### Stage 1: Extract

Ships the workspace skeleton (five crates, `Cargo.lock`, CI, `justfile`, the file-size check), the config
loader and `vpt setup`, the home, stores and `stores` refusals, the managed symlink and `vpt symlink`,
the SQLite ledger with its in-memory twin, migrations, the write lock, the dirty-artifact publication
protocol and the retention intents, `vpt ingest` with the wholeness gate, the title copy, staging and
publication, `vpt show`, `vpt list`, `vpt storage`, `vpt doctor` (config, stores, helper presence,
recordings directory readability, symlink, git trees, subdirectory counts), the `vpt-macos` helper
package with `notify` and `trash`, `[notify]` with all three modes and `vpt.event/1`, and
`vpt retention run`. Depends on nothing.

### Stage 2: Transcribe

Ships the `Engine` port, the `apple` (helper `transcribe`), `whisply` and `command` adapters,
`vpt.engine/1`, the two slots and `checker_runs`, per-language pairs, the same-family refusal, the
normalizer, aligner and classifier, the accepted transcript and the flags in the ledger,
`known-terms.txt`, `vpt transcribe`, `vpt review`, `vpt confirm --term`, the readability pass, the
`review_needed` and `transcribe_failed` events, and a minimal transcript note in the portable profile
(frontmatter, H1, the content region with the timecoded transcript and its inline markers) so a
transcript is readable before stage 3. Depends on stage 1.

### Stage 3: Vault note

Ships the full note renderer for both profiles, the managed link block, `vpt note write`, `vpt path`, the
tag gate with `new_tags`, `known-tags.txt`, `vpt confirm --tag|--reject-tag|--relation`, the four
relations, the output-tree index, rename recovery, the vault-related `doctor` checks, and `vpt run`
composing stages 1 to 3. Depends on stage 2.

### Stage 4: Synthesis and extras

Ships `[synthesis]`, `vpt.proposal/1`, `vpt synthesize`, `vpt verify-note` and the analysis note;
occasions, `vpt brief` in all three forms, `vpt occasions`, `vpt.brief/1`, `vpt.context/1`, the `dam`,
`google` and `none` context sources, the calendar trigger and `brief_written`; `vpt redact`, `[share]`,
the release rows and the draft reports; `vpt handoff` and `vpt.handoff/1`; and `vpt run` extended with
synthesis and automatic redaction. Depends on stage 3.

### Deferred until the external notebook is deployed

The mapping from `vpt.handoff/1` to the notebook's source-creation call, the transport recipe and where
it lives, who holds the notebook's password and whether an agent gets write access, the return path for a
note authored in the notebook, and a remote identifier that lets a re-handoff replace rather than
duplicate. None of these is vpt code today; the handoff verb ships complete in stage 4 without them.
