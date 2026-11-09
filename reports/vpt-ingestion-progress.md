# Ingestion identity and recovery acceptance

## Fixed archive verification gaps

Two behavioral tests exposed recovery problems:

1. Synchronizing an existing archive checked its directory but accepted content changed during
   synchronization. The regression initially returned `Ok(())` after the file changed.
2. Orphan recovery read container metadata and computed the digest through separate opens. Replacing
   the archive after its first open allowed recovery using the first file's metadata and the second
   file's digest. The regression initially reported that orphan as recovered.

Archive readers now retain the descriptor, initial metadata, and path. Container reads and digesting
use that descriptor. Hashing is bounded by the captured size and uses the existing 64 KiB buffer.
Before returning a digest, the adapter checks device/inode, size, nanosecond modification/change times,
the named file, and the retained directory. Edits, growth, truncation, and replacements refuse the read.

Existing-archive synchronization now requires the expected full digest, retains the opened file, and
checks content/version/path before and after synchronizing that file and its directory. Recovery and
source reassociation remain uncommitted when verification fails. Changed operator bytes remain in
place. The archive implementation is split into root/listing, port, reader, verification, publication,
and staging modules; the changed production files stay below 150 lines. No unsafe blocks, dependencies,
or schema changes were added.

These checks do not resolve the broader existing-artifact compare-and-replace races tracked in the
shared roadmap section. They verify the observed read/sync boundaries; they do not make filesystem
operations and the ledger commit one atomic transaction with external writers.

## Command acceptance

New command tests exercise the real SQLite ledger, archive adapter, command output, history, and a
fixture Trash helper:

- Ingest in UTC, rename the source, then ingest in Pacific/Kiritimati. The identity, archive path,
  digest, capture offset, and archive bytes remain unchanged. The source association moves and the
  redundant staged copy goes through the helper. Editing the source then produces a distinct
  recording while retaining the first archive.
- Recover an orphan with a saved offset of -06:00 while the command runs at +14:00, both with and
  without its source. Preserve its capture instant, offset, duration, full digest, and archive bytes.
  Recovery is idempotent; a surviving source is reassociated, and dry runs leave all files unchanged.
- Refuse a valid but different container occupying the expected archive path with exit 3 and
  `archive_collision`, empty stdout, no recording/source rows, and one safe failure event. Preserve
  both source and foreign archive bytes.

Injected integration faults also replace an orphan after open and change an archive after digest
verification. Neither may create a recovery row or update an existing source association.

## Verification evidence

The behavioral failures are preserved in `/private/tmp/vpt-archive-sync-red.log` and
`/private/tmp/vpt-orphan-snapshot-red.log`. The fixed recovery suite passed all eight cases in
`/private/tmp/vpt-orphan-snapshot-green.log`. Archive unit tests passed 31 cases in
`/private/tmp/vpt-archive-snapshot-final.log`.

All build/test/lint work is sequential, with `TMPDIR=/private/tmp` and serial CLI/SQLite tests.
Existing deadlines are unchanged.

Final broader validation passed:

- 74 adapter ingestion tests: `/private/tmp/vpt-ingest-adapters-final.log`.
- 23 ingestion command tests: `/private/tmp/vpt-ingest-commands-final.log`.
- Workspace Clippy, all targets with `dev-tools` and warnings denied:
  `/private/tmp/vpt-ingest-archive-clippy.log`.

Together with the 31 archive unit tests, these cover 128 distinct passing Rust tests.
The focused doctor source-count check still exceeded its existing 500 ms deadline
(`/private/tmp/vpt-ingest-source-gone-doctor.log`). Its roadmap row remains open; the cause of the
latency is not established. The three accepted ingestion rows leave 81 checked and 26 unchecked
items at this point in the roadmap.

Final Rust formatting check passed (`/private/tmp/vpt-ingest-native-format.log`), and
`git diff --check` passed.

## Doctor startup timing investigation

The source-count command still failed its unchanged 500 ms budget in both the tool sandbox and a
host-access comparison. A temporary diagnostic at the first statement of `main` produced no bytes
before the sandboxed command timed out. The live doctor test executable was separately sampled
while waiting to start: all 879 samples were at `_dyld_start`, before Rust test execution.

Evidence:

- Instrumented failure: `/private/tmp/vpt-doctor-profile.log`.
- Live process sample: `/private/tmp/vpt-doctor-process-sample.txt`.
- Host comparison: `/private/tmp/vpt-doctor-host-timing.log`.
- Two later plain version invocations: `/private/tmp/vpt-startup-measurement.log`, measured at
  241 ms and 112 ms.

This demonstrates a pre-main delay in the observed failure; it does not establish the underlying
macOS cause or prove that every earlier doctor timeout had that cause. All temporary diagnostics
were removed, and no production behavior or test deadline was changed. Doctor acceptance remains open.

## Ingestion selection and deferral acceptance

Four command-level acceptance cases now cover the remaining selection and retry behavior:

- A full sweep sees only immediate lowercase `.m4a` files. Sidecars, uppercase extensions, symlinks,
  and nested files are excluded. `--once` selects only its named recording. The source-tree snapshot
  retains file bytes, type, inode, link count, mode, flags, size, mtime, and ctime across ingestion.
- Both observational and mutating `--once` enforce oversized, incomplete-container, and quiet-period
  deferrals. Mutating runs persist the reason/count without inserting a recording; observational
  runs preserve the entire fixture snapshot.
- Observational `--once` rejects linked and nested paths as named refusals and directories as invalid
  input. Each leaves stdout empty and the fixture unchanged. The first version of this test wrongly
  expected exit 3 for a directory; it was corrected to the specified invalid-input exit 2. No product
  error mapping was changed.
- Across separate CLI invocations, default-four and configured-two thresholds save consecutive
  deferrals and exactly one alert at the threshold. The event carries the safe source path, exact
  reason/count, and no audio content. Further retries and dry runs do not send another alert; the
  report channel is not invoked. A repaired file first waits for a stable size, then ingests and
  clears its deferral state.

These complement the adapter gate tests for dataless-before-open, oversized-before-read,
source-descriptor metadata, exact quiet-period boundaries, changed-during-read and staged-container
failures, and atomic seen/history/threshold-event persistence. The earlier adapter observation tests
also prove known-title preservation, no title-copy refresh, and no staging/history/Trash work.

Evidence:

- Initial expanded command run: 26 passed, one incorrect test assertion failed:
  `/private/tmp/vpt-ingest-selection-acceptance.log`.
- Corrected command run: all four new cases passed, with 26 total passes and one existing startup-note
  recovery timeout: `/private/tmp/vpt-ingest-selection-final.log`.
- That startup-note case passed on a focused rerun at the unchanged 3-second deadline:
  `/private/tmp/vpt-ingest-startup-repeat.log`. The failed full run remains recorded; its maximum
  polling gap was 447 ms. This does not establish a cause or guarantee stable command latency.

Together the final run and focused rerun cover 27 distinct passing ingestion command tests. There
were no production-code changes in this acceptance work; the existing implementation passed the
new behavior checks. The doctor deadline failure remains unresolved.

The typed-defaults test passed (`/private/tmp/vpt-ingest-defaults.log`), confirming the 2 GiB limit,
30-second quiet period, four-deferral threshold, and disabled-by-default hooks. The ingestion command,
candidate gates, persistent deferrals, and `--once`/dry-run acceptance rows are now checked. This
leaves **86 checked and 21 unchecked items**. Startup durability, logging-wide guarantees, doctor and
other observational timing, shared publication races, and installation retain their separate open rows.

Final workspace Clippy passed with all targets, `dev-tools`, and warnings denied
(`/private/tmp/vpt-ingest-selection-clippy.log`). Rust formatting passed
(`/private/tmp/vpt-ingest-selection-format.log`), and `git diff --check` passed. Builds, tests, and
lint ran sequentially; CLI tests remained serial and all deadlines stayed unchanged.

## Staged container and digest consistency

A new regression reproduced a recording whose digest came from the originally staged bytes while
its capture time and duration came from replacement bytes. `stage()` had hashed the file before
`open_and_publish()` opened it again for container inspection; that later inspection was not checked
against the staged digest. Evidence: `/private/tmp/vpt-staged-consistency-red.log`.

Ingestion now hashes through the same retained descriptor used for container inspection, checks the
snapshot size against the staged size, and requires the staged digest to match. The archive reader's
existing version/path checks detect edits or replacement during inspection. Failure stops before
publication or ledger insertion, saves the normal safe failure event, and retains the staged file
with a cleanup-pending diagnostic. It does not send unverified replacement bytes to Trash.

The regression covers changed bytes before container open, an in-place edit after open, and a
same-content inode replacement after open. The replacement fixture preserves both file names; an
initial assertion wrongly expected only one leftover and was corrected to require both files to
remain. That intermediate result is `/private/tmp/vpt-staged-consistency-green.log`.

This closes the observed gap between staging and inspection. It does not make the later publication
and ledger commit atomic with external filesystem writers. The broad durable-publication row remains
open, along with the descriptor-publication acceptance row until its remaining boundaries are proved.

## Abort after an earlier recording committed

New adapter integration cases use three ordered recordings and fail on the second. They check source
open failure, no space before publication, synchronization failure before publication, and a placed
file whose directory synchronization is unconfirmed. Every case preserves the first recording and
archive, reports its completed identity, leaves the second unrecorded, and never opens or stages the
third source. Exactly one safe failure event and a failed run outcome are recorded.

For pre-publication failures, only the owned staged copy reaches Trash. If the helper is absent, the
stage remains and the result includes the safe helper-unavailable diagnostic. A placed-but-unsynced
archive stays available for recovery and never reaches Trash. Source bytes remain unchanged. These
cases exercise the real SQLite ledger and archive adapter with faults at the relevant port boundary.

The six adapter targets passed all 56 tests after the fix: abort (2), failures (11), gates (28),
recovery (8), staging consistency (1), and sweep (6). Evidence:
`/private/tmp/vpt-staged-abort-adapters.log`. This includes existing staged-container failure and
cleanup checks, so stable-but-invalid staged clones still defer and use the established cleanup path.

All 29 ingestion command tests passed in one final run, including the two new abort acceptance
cases (`/private/tmp/vpt-staged-abort-commands.log`). An unexpectedly empty previously populated
source returns exit 1 with empty stdout and the expected JSON/human failure output, retains its
archive, and records one failure event per run. An invalid title database still produces an untitled
recording, preserves the Voice Memos tree, and leaks no database bytes into output or events.

Together these runs cover 85 distinct passing tests. The abort-behavior row is checked, leaving
**87 checked and 20 unchecked items**. The production change is confined to staged validation in
`crates/vpt-application/src/ingest/staging.rs` (123 lines); no dependencies, schema changes, or unsafe
blocks were added. The extra hash binds container inspection to the staged digest and adds another
bounded read of the staged audio. Doctor timing and the later publication races remain open.

Final workspace Clippy passed with all targets, `dev-tools`, and warnings denied
(`/private/tmp/vpt-staged-abort-clippy.log`). Rust formatting passed
(`/private/tmp/vpt-staged-abort-format.log`), and `git diff --check` passed. The build/test/lint
sequence was serial, CLI/SQLite tests used one test thread, and deadlines remained unchanged.

## Logging recovery and configuration refusals

A new command acceptance case blocks `events.jsonl` with a directory and makes the configured alert
exit 7 after accepting its JSON event. Ingestion retains its result and reports both safe warnings.
The test then writes an already-exported record without its database acknowledgement and appends an
incomplete tail. The next invocation repairs the tail, preserves recognizable duplicate identifiers
and original occurrence times, acknowledges the recovered records, and saves one evidence-based
summary. Neither recovery nor another invocation resends the alert. A dry run preserves the full
fixture snapshot even while export is unavailable.

The first version of this test incorrectly rejected the generic word `private`, which also occurs
in `/private/tmp` paths. It was corrected to reject the specific audio and hook-output sentinels;
this was a test assertion correction, not a product fix. Evidence:
`/private/tmp/vpt-logging-command-acceptance.log` and
`/private/tmp/vpt-logging-command-final.log`.

The 74 adapter logging/private-log tests passed in `/private/tmp/vpt-logging-adapters.log`. These
cover private exclusive creation and unsafe-log refusal, complete synced acknowledgements, partial
append repair, failed acknowledgements, interrupted recovery progress, summary watermarks, missing
evidence, independent literal-argument hooks, and no operation or hook replay. The hook status test
now also runs a real sleeping child against the unchanged five-second deadline and verifies a typed,
content-free timeout fact.

The `config_refused` event previously had no producer. A failing CLI regression now demonstrates the
missing history when a configured external store's parent exists but refuses creation of its leaf.
Ingestion saves the typed configuration failure and failed run, exports it, and invokes the alert
after the state root is validated and locked. Required history failures still stop through the
existing commit path; export/hook failures preserve the original configuration exit status.
Evidence: `/private/tmp/vpt-config-refusal-red.log` (missing event) and
`/private/tmp/vpt-config-refusal-green.log` (passing regression).

The related source-failure helper was renamed to `ingest/startup_failure.rs`, and both startup failure
paths share the existing commit/export behavior. No raw error, configuration value, or child output
is added to history. The pipeline's source-failure call follows the rename without changing its
behavior. The touched production files remain at or below 150 lines.

All 33 ingestion command tests passed in `/private/tmp/vpt-logging-config-commands.log`, including
three configuration refusal cases. They cover original exit 2 with simultaneous export and hook
failure, observational dry runs, and output-only refusal before root validation with both absent and
existing state. Together with the adapter suite, this is 107 distinct passing tests.

The recovery summary, recognizable replay, and hook-failure acceptance rows are checked, leaving
**90 checked and 17 unchecked items**. Other command producers, broader history/export/output wiring,
startup acceptance, doctor timing, publication races, shared contracts, and installation retain their
open rows. In particular, the new configuration producer currently covers ingestion's late store
creation failure; it does not establish completion of every command's configuration failure path.

Workspace Clippy passed with all targets, `dev-tools`, and warnings denied in
`/private/tmp/vpt-logging-config-clippy.log`. The build/test/lint sequence was serial; CLI and SQLite
tests used one test thread, and all command and subprocess deadlines remained unchanged.
Rust formatting passed (`/private/tmp/vpt-logging-config-format.log`), and `git diff --check` passed.

## Standalone installation acceptance

The current checkout installed successfully with `cargo install --offline --locked --path crates/vpt`
into an isolated prefix under `target/`. The default installation contains the production `vpt`
binary and no development fixture binary. The Swift helper built in release mode with `just swift-build`
and was copied beside it with mode 0755. These checks use the current local source; they do not claim
that unpublished work is available from the remote Git installation command.

Evidence:

- `/private/tmp/vpt-install-rust.log`: successful optimized Rust install.
- `/private/tmp/vpt-install-helper.log`: successful native Swift release build.
- `/private/tmp/vpt-install-runtime.log`: both installed-binary acceptance tests passed.

The optional `installation` integration target requires `VPT_TEST_INSTALL_ROOT` and uses the installed
release binaries, independent fixture HOME/config/state directories, a minimal PATH, and a working
directory outside the source checkout. Version checks identify Rust 0.1.0 and helper 1.0.0. Doctor
passes with the native helper present, reports both installation steps when it is absent, and keeps
all fixture files unchanged. Those observations retain the existing 500 ms command budget.

With the helper absent, real external fixture executables exercise both the command-engine protocol
and the Whisply adapter protocol. Each pipeline ingests, transcribes, and publishes a note. A second
run exits without repeating completed stages or calling the engine again. Brief creation, redaction,
and released handoff also succeed; handoff is observational and logs omit private fixture text. An
expired owned audio file remains intact when retention refuses with `helper_absent`. Pipeline and
later-stage command budgets remain 8 and 5 seconds; retention keeps its existing 500 ms budget.

These tests validate installation and VPT's external-engine integration, not third-party model
accuracy or a separately installed Whisply release. Native Apple transcription was validated earlier
with generated speech and the authorized en-US assets, as recorded in `reports/vpt-stage2-progress.md`.
No user binaries, shell settings, LaunchAgents, personal audio, or dotfiles were changed.

README installation and external scheduling instructions are updated. The standalone installation
row is checked, leaving **91 checked and 16 unchecked items**. No production-code change was needed
for these acceptance cases.

## Current observational acceptance

The installed release's doctor checks completed within the existing budget, so the full doctor and
show/list/storage suites were checked in both build profiles without changing any deadline:

- Release: all 38 doctor and 31 show/list/storage tests passed in
  `/private/tmp/vpt-release-observations.log`. This includes native concurrent-writer fixtures,
  source-gone counts, engine availability/locality, vault checks, all store roots, and unchanged
  filesystem snapshots.
- Debug doctor: 37 passed and the active native-writer case failed its 500 ms deadline, with empty
  stdout/stderr, in `/private/tmp/vpt-debug-observations-current.log`. Source-gone counts passed.
  This observation alone does not establish the failure's cause or prove it matches the earlier
  pre-main delay. The doctor acceptance row remains open.
- Debug show/list/storage: all 31 passed, including the active native-writer case, in
  `/private/tmp/vpt-debug-storage-current.log`.
- All four ingestion liveness tests passed in `/private/tmp/vpt-source-liveness-current.log`,
  confirming vanished-source persistence, archive preservation, observational dry runs/once, and
  the empty-populated-source refusal.

The source-gone and show/list/storage rows are now checked. Alongside installation this closes three
rows in this continuation, leaving **93 checked and 14 unchecked items**. The release/debug comparison
does not resolve the remaining doctor timing failure. Existing publication races and the other open
history, startup, configuration, privacy, and process/document contracts remain separate work.

Workspace Clippy passed with all targets, `dev-tools`, and warnings denied in
`/private/tmp/vpt-install-observations-clippy.log`. Builds, tests, and lint ran sequentially, with
one test thread for CLI and SQLite tests. No production code or test deadline changed.
Rust formatting passed (`/private/tmp/vpt-install-observations-format.log`), and `git diff --check`
passed.

## Configuration event producers and startup ordering

Store creation errors now use `compose/config_refusal.rs` after the caller has validated the roots,
opened private state, acquired the write lock, and established history/export handling. The helper
records only the typed `ConfigurationRefused` class. It either appends to the caller's active run,
finishes that run when the caller exits directly, or starts and finishes a failure run when the
normal work run has not started. This preserves the existing command error and prevents duplicate
run finishes. A required history failure replaces the configuration error through the existing
ledger failure path; optional export and hook failures remain warnings.

The producer is connected to ingestion, pipeline, transcription, note writing, synthesis,
recording confirmation, review resolution, redaction, and manual/upcoming brief creation. Ingestion's
source-failure helper retains its distinct `ingest_failed` event. No helper opens a state path or
reparses invalid configuration to obtain somewhere to log.

The initial cross-command test fixture mistakenly used an unsupported synthesis setting and passed
a recording ID to global term confirmation. Those early failures were not valid evidence of the
intended store-creation defect. The fixture was corrected, uses tag confirmation for the recording,
and now requires the exact `stores.drafts` error before asserting missing history. With the nine
original return-only error paths temporarily restored, all nine corrected cases failed on the missing
event (`/private/tmp/vpt-config-producers-corrected-red.log`). Restoring the producer wiring passed all
nine (`/private/tmp/vpt-config-producers-corrected-green.log`).

Two further cases exercise all three run lifecycles with simultaneous export/hook failure and with a
SQLite trigger refusing the configuration event transaction. Optional delivery failures preserve
exit 2 and one safe warning of each kind, send no report fallback, and retain pending exports.
Required history failure returns exit 1, emits no hook, and starts no new engine or artifact work.
All 11 configuration tests and the then-current 33 ingestion tests passed together in
`/private/tmp/vpt-config-producers-boundaries.log`.

A separate regression found that ingestion returned an unavailable-source error before reconciling
pending retention. The corrected order repairs publications and retention under the existing lock
before handling the source error, then attaches completed recovery identities to that error.
The failing regression is `/private/tmp/vpt-ingest-startup-source-red.log`; the passing fix is
`/private/tmp/vpt-ingest-startup-source-green.log`. The caller now attaches source-failure warnings
once, matching the pipeline's warning ownership.

The expanded command run passed all 328 tests in `/private/tmp/vpt-config-producers-command-suite.log`:
brief 34, configuration refusals 11, confirmation 18, ingestion 35, notes 62, pipeline 58, redaction 12,
retention 19, review 18, synthesis 32, and transcription 29. This includes successful and failed event
delivery, independent channels, automatic brief reports, manual brief silence, partial completion,
recovery without replay, privacy, output failures, and observational plans. Ingestion's second new
startup case confirms that simultaneous export/hook warnings appear exactly once after recovery and
source failure.

The startup, remaining event-producer, and ingest/retention output rows are checked, leaving
**96 checked and 11 unchecked items**. The broader command-history/export contracts retain their
own open rows, as do shared configuration/output/dry-run/document/process/privacy contracts. This
does not resolve the debug doctor timing failure or the publication races.

Both write-lock tests passed in `/private/tmp/vpt-startup-lock-current.log`, checking bounded writer
contention and waiting for the prior writer to release its lock.

All 21 adapter cleanup and recovery tests passed in `/private/tmp/vpt-startup-cleanup-current.log`
(abort 2, failures 11, recovery 8). Together with the command and lock suites, this change passed
351 tests. Workspace Clippy with all targets, `dev-tools`, and warnings denied passed in
`/private/tmp/vpt-config-producers-clippy.log`. Final formatting verification passed in
`/private/tmp/vpt-config-producers-format-final.log`, and `git diff --check` passed.

## Shared dry-run command acceptance

The six advertised dry runs (`ingest`, `run`, `transcribe`, `synthesize`, `retention run`, and
`brief`) now share a cross-command acceptance matrix in `tests/pipeline/dry_runs.rs`. Both human
and JSON invocations preserve complete filesystem snapshots: bytes, paths, modes, inodes, link
counts, flags, and modification/change times. Cases cover absent state and stores, an actual
version-one schema without migration, a refused future schema, pending note publication,
retention intent, history exports, a partial event-log tail, and a held writer lock. Old-schema
commands may report unavailable ledger data or a missing recording; neither result migrates it.
The pending-work case also checks existing-occasion `brief --dry-run` and `--explain` in both
output modes. Configured engine, synthesis, hook, and helper markers remain untouched. A separate
case enables automatic briefs and proves pipeline preview does not collect provider context.

These are acceptance tests for existing production behavior. Initial test setup errors included an
incorrect expectation that every success document carries `completed`, and an implicit Rust module
path that selected an existing sibling fixture. Both were corrected before using the results as
evidence; no product defect is inferred from those initial failures. The corrected five-case matrix
passed in `/private/tmp/vpt-dry-run-contract-acceptance.log`. The six command targets then passed
all 27 matching dry-run tests in `/private/tmp/vpt-dry-run-command-suite.log`, including title-copy
preservation, pending engine outputs, no publication repair, output failures, and the allowed scoped
read-only brief context collection. The expanded existing-occasion case passed in the subsequent
64-test pipeline suite in `/private/tmp/vpt-dry-run-protocol-command-suite.log`.

The shared observational dry-run row is checked: **97 checked and 10 unchecked items**. Broader
output, protocol, subprocess, privacy, history/export, and publication contracts remain open.

## Proposal versions and stage diagnostics

The proposal decoder and adapter retained an unknown numeric schema version, but the synthesis CLI
discarded it. The behavioral test in `/private/tmp/vpt-proposal-version-red.log` failed on the generic
message. The CLI now names `vpt.proposal/<version>`, including the full `u64` range, in both JSON and
human errors. Accepted proposal and analysis content survive the refused attempt. The test permits
the existing note preparation to refresh `updated_at`; all other saved analysis fields and file
content must match. The first post-fix run used an overly strict timestamp comparison; the corrected
test and pipeline version-refusal case passed in the 97-test run recorded in
`/private/tmp/vpt-dry-run-protocol-command-suite.log`.

A second behavioral regression demonstrated that `run` discarded successful stage diagnostics.
The three pipeline cases failed in `/private/tmp/vpt-stage-diagnostics-red.log`; standalone synthesis
also hid additive-field diagnostics from human output, demonstrated by
`/private/tmp/vpt-synthesis-diagnostics-red.log`. Pipeline reports now retain diagnostics from
transcription and synthesis, prefixed with their recording identity. Both output modes show them,
and a later failure or a broken stdout retains them alongside completed identities. Successful
ingestion diagnostics also survive later pipeline failures. Standalone synthesis includes the same
safe diagnostics in its human summary. Additive field values remain absent from these messages.

All four focused diagnostic regressions passed in `/private/tmp/vpt-stage-diagnostics-green.log`.
No schema, dependency, deadline, or unsafe block changed. Changed production files remain below
150 lines. The broader protocol and output rows stay open until their complete contracts are verified.

Final validation passed all **101 command tests** (pipeline 67, synthesis 34) in
`/private/tmp/vpt-command-diagnostics-final.log`. Workspace Clippy with all targets, `dev-tools`, and
warnings denied passed in `/private/tmp/vpt-command-contracts-clippy.log`. Formatting verification
passed in `/private/tmp/vpt-command-contracts-format.log`, and `git diff --check` passed. Commands
ran sequentially with unchanged deadlines and CLI tests restricted to one test thread. The checklist
remains **97 checked and 10 unchecked**; the full goal is still active.

## Protocol diagnostics at save boundaries

Valid engine and proposal replies lost their additive-field diagnostics if a subsequent ledger
commit or publication acknowledgement failed. Two behavioral cases failed in
`/private/tmp/vpt-protocol-save-diagnostics-red.log`. Transcription now retains engine diagnostics
around classification, persistence, and publication. Synthesis retains proposal diagnostics before
rechecking its input and carries them through its failure-history handling. Both paths attach the
diagnostics to the final error without changing its kind, exit code, or completed identities.
The synthesis content reader was extracted into its existing source module; changed production
files are 143, 147, and 36 lines, with no new unsafe code, dependency, schema, or deadline change.

All five focused cases passed in `/private/tmp/vpt-protocol-save-diagnostics-green.log`. The two
new save-boundary cases each cover commit rollback and publication acknowledgement failure in human
and JSON modes. They require each diagnostic exactly once, exclude private additive values and
trigger text, and distinguish rolled-back data from accepted data with pending publication.
Existing unknown-version tests also pass for both command error formats.

The protocol-level suite passed **93 tests** in `/private/tmp/vpt-protocol-contracts-current.log`.
The new brief encoder case validates a domain document below its raw-content capacity, then checks
that JSON escape expansion beyond the encoded-output budget returns `CapacityExceeded`. Its plain
text counterpart succeeds; the separate handoff case covers the same output expansion boundary.

The shared protocol requirement maps to these existing interfaces and checks:

| Interface | Boundary and evidence |
| --- | --- |
| Engine input | 16,777,216-byte capture/parse limit; version and engine identity validation; depth 8; 65,536 characters per string; 65,536 segment/word entries only at their declared locations. Protocol and adapter tests cover exact limits, overflow, malformed data, and additive diagnostics. Apple and Whisply converge on the same validated domain transcript. |
| Proposal input | 1,048,576-byte capture/parse limit; canonical version checking; summary/action arrays up to 65,536; other arrays up to 256; per-string and nesting limits. Unknown versions reach CLI errors; additive fields reach success and save-failure diagnostics. |
| Context input | 65,536-byte normalized document and provider-response limits, depth 8, arrays up to 256, and bounded strings. Provider collection and normalized encoding refuse oversized aggregates. Unknown versions and additive fields are checked by the context decoder. |
| Brief output | Emits `vpt.brief/1` from validated domain evidence, with bounded source arrays and text, a 16 MiB aggregate text budget, and a 32 MiB encoded-output ceiling. Tests cover invalid evidence and escaping expansion. |
| Handoff output | Emits `vpt.handoff/1` from validated artifact content and provenance; raw body and encoded-output ceilings reject oversized copies. Exact body preservation and schema/identity behavior are checked by protocol and command tests. |

Brief and handoff are output interfaces in the product specification; VPT has no import command for
either document. The incoming byte ceilings apply to engine, proposal, and context readers. Their
shared bounded reader checks bytes before decoding and rejects depth/array overflow while visiting
the document, including before decoding a malformed overflow element. It rejects repeated object
keys and emits fixed safe syntax errors. Unknown additive values remain subject to these limits.

Adapter validation passed **68 tests**: engine adapters 26 in
`/private/tmp/vpt-protocol-engines-current.log`, synthesis 9 in
`/private/tmp/vpt-protocol-synthesis-current.log`, and context 33 in
`/private/tmp/vpt-protocol-context-current.log`. This covers subprocess result decoding, exact byte
limits and overflow, bounded provider transport/aggregation, cleanup, and safe typed failures.

The command acceptance run in `/private/tmp/vpt-protocol-command-acceptance.log` passed handoff
(11), synthesis (35), and transcription (31). Pipeline passed 67 of 68; its new diagnostic case
incorrectly assumed only one publication could remain pending after an analysis acknowledgement
failure. Transcript and analysis publication form one batch. The corrected assertion checks the
requested stage's pending target, preserving all diagnostic and committed-identity assertions;
it passes in `/private/tmp/vpt-pipeline-publication-diagnostics-rerun.log`.

Brief passed 33 of 34 in that run, with an intermittent ledger conflict during the retention case.
The focused rerun passed in `/private/tmp/vpt-brief-conflict-rerun.log`. Investigation found a real
clock-boundary defect: occasion preparation and brief assembly read the clock separately, but
assembly used its later timestamp for the earlier occasion's history fact. The ledger correctly
rejects that mismatch. A command regression delays successful calendar collection by one second;
it fails with the same ledger conflict in `/private/tmp/vpt-brief-clock-boundary-red.log`. The initial
fixture used `sleep` without an absolute path, so its restricted PATH prevented the delay; that
fixture was corrected before recording the failing regression. Brief assembly now uses the
occasion change's timestamp for its fact, retaining the independently generated brief timestamp.
This shared assembly path also serves upcoming briefs. The original failure had no invocation
attribution, so it cannot independently establish which call crossed the second boundary.

All **35 brief tests** now pass in `/private/tmp/vpt-brief-clock-boundary-green.log`, including the
clock-boundary regression and the original retention case. The shared JSON test helper now uses
`track_caller` to identify the actual invocation on future failures. Workspace Clippy with all
targets, `dev-tools`, and warnings denied passes in `/private/tmp/vpt-protocol-brief-clippy.log`.
Changed production files remain below 150 lines. No deadline, schema, dependency, or unsafe code
changed. The bounded versioned-document row is complete: **98 checked and 9 unchecked items**.

## Subprocess ownership acceptance

All production external-command paths use `spawn::run` or `spawn::run_with_env`: generic and Apple
engines through `engines/process.rs`, Whisply through its bounded variant, synthesis, the helper,
dam collection, and report/alert hooks. The shared owner constructs `Command` directly from the
configured argv and starts its own process group. Input writes, concurrent output draining, and
waiting share the same deadline. Cleanup retains group authority through termination, observes the
one-second grace, kills survivors, revokes signal authority, and reaps the exact owned child.

The **66 shared process tests** pass in `/private/tmp/vpt-subprocess-ownership-acceptance.log`.
They cover real input/output exchange and pressure on both streams, bounded capture while draining
overflow, early input closure, missing pipes and I/O failures, deadline and interruption cleanup,
held participant pipes after the leader exits, cleanup with closed participant pipes, all three
installed interrupt handlers, observation/reaping failures, and cancellation during unwinding.
Existing fixture watchdogs and production deadlines are unchanged.

The adapter cases recorded above check configured engine and synthesis deadlines, argument
boundaries and single-pass substitution, exact stdin, output overflow, typed safe failures, and
Whisply workspace cleanup. Context tests check a single 30-second budget across every dam query,
or across Google token refresh and every calendar page, with interruption taking precedence over
a late reply. Google uses native HTTPS and does not spawn a process. The **8 helper tests** pass in
`/private/tmp/vpt-helper-process-acceptance.log`; they verify the five-second call budget and bounded
capture passed to the owner, compatible version caching, exact Trash path acknowledgements, and
safe decoding/status errors. Existing logging acceptance covers the hooks' five-second deadline.

A new command acceptance case delivers SIGINT, SIGTERM, and SIGHUP from the active collector during
both manual and upcoming brief creation. Each invocation returns one safe failure with empty stdout,
runs no second query, saves no occasion, and publishes no brief. This matters because an ordinary
collector failure is allowed to produce a manual brief with absent context, while interruption must
stop the command. It passes together with the two engine/agent pipeline interruption cases in
`/private/tmp/vpt-command-interruption-acceptance.log` (three tests, multiple invocations each).
No production change was needed for this acceptance case.

Workspace Clippy with all targets, `dev-tools`, and warnings denied passes in
`/private/tmp/vpt-subprocess-acceptance-clippy.log`. The subprocess row is complete:
**99 checked and 8 unchecked items**. Whole-command history/export, configuration/output/privacy,
doctor timing, and publication boundaries remain open.

## Brief count settings and configuration structure

Section 10 allows nonnegative 32-bit counts. The schema accepted that range for `brief.max_notes`
and `brief.max_spans_per_note`, but typed conversion and domain validation imposed an additional
65,536 ceiling even when a brief had only one available note and quote. The command regression in
`/private/tmp/vpt-brief-config-counts-red.log` failed with exit 2 at that extra configuration limit.
Both settings now use the shared 32-bit conversion, and the selection code applies them to actual
available data without allocating based on the requested count. Candidate and term counts, input
bytes, quote counts, output bytes, and work budgets retain their existing independent bounds.

The same command regression now passes in `/private/tmp/vpt-brief-config-counts-green.log`, selecting
one real source in both preview and publication with both counts set to `u32::MAX`. Domain tests cover
65,537 and `u32::MAX`, preserving selected evidence and omission counts. The prior test for the extra
configuration ceiling was replaced with these specified range checks; excessive actual term input
still produces `CapacityExceeded`. All 25 brief domain tests pass in
`/private/tmp/vpt-brief-count-domain.log`.

The 471-line configuration declaration file was split into seven feature modules, each under 150
lines. The 81 declarations retain their exact text and ordering, checked during the move. The
registry now exposes an allocation-free `keys()` iterator, used by validation and template rendering.
All **104 configuration tests** pass in `/private/tmp/vpt-config-contract-acceptance.log`, including
template loading, override/default handling, strict keys and value ranges, typed settings, secret
canaries, and protected configuration paths.

The oversized identity and time files contained inline tests; those tests were moved unchanged into
their own modules. Their production files are now 119 and 148 lines. All **262 domain tests** pass in
`/private/tmp/vpt-brief-count-domain-final.log`. The configuration row remains open until every key's
runtime behavior and command arguments have complete acceptance evidence. The checklist remains
**99 checked and 8 unchecked**.

Final command validation passes all **37 brief tests** in
`/private/tmp/vpt-brief-config-final.log`. Workspace Clippy with all targets, `dev-tools`, and warnings
denied passes in `/private/tmp/vpt-configuration-final-clippy.log`; formatting verification passes in
`/private/tmp/vpt-configuration-final-format-check.log`, and `git diff --check` passes. Build, test,
and lint processes ran sequentially; CLI and SQLite tests used one test thread and no deadlines
were changed. The full goal remains active.

## Configuration acceptance across commands

The new `tests/configuration.rs` matrix covers 26 command forms, including separate confirmation,
path, brief, handoff, and redaction variants. Both output modes select the default configuration,
then `VPT_CONFIG`, then explicit `--config` in the expected precedence. Distinct unknown keys identify
which file was loaded. Every invocation preserves the entire fixture's bytes and metadata, returns
exit 2 with empty stdout, and hides the invalid value. JSON is exactly one `vpt.error/1` document.
A second matrix covers missing/unsupported versions, an unknown enum value, and a count outside its
32-bit range before any command work. Both tests pass in
`/private/tmp/vpt-configuration-command-acceptance.log` (260 CLI invocations).

The independent-store command case passes in `/private/tmp/vpt-configuration-stores-acceptance.log`.
It uses both `VPT_CONFIG` and `--config` to select a valid file while the default file is invalid,
then ingests, transcribes, synthesizes, redacts both note stages, and writes a brief. All seven stores
use separate external parents and custom leaf names. Every owned artifact is present in a configured
store, default leaves remain absent, private leaves have mode 0700, shared parents retain mode 0755,
the source recording is unchanged, and publication acknowledgements are complete.

All **21 argument-parser tests** pass in `/private/tmp/vpt-configuration-arguments-acceptance.log`.
The initial command selected the binary target, which contains no parser tests; the recorded run was
corrected to the library target before treating it as evidence. Later brief/redaction/handoff argument
acceptance is also exercised by their command suites.

The 81 static configuration declarations were compared with specification section 10: all 74 literal
defaults match; the remaining seven declarations expand `retention.hold.<store>` with zero for each
store. Dynamic engine declarations use the documented required command fields and optional language
selections. Runtime evidence is grouped below; typed conversion alone is not the acceptance criterion.

| Settings | Runtime use and behavior checks |
| --- | --- |
| `config_version`, `home.*`, `stores.*`, `helper.path` | Shared configuration loading and root resolution; setup/symlink and root tests; helper version/Trash/Apple calls; the command precedence and independent-store matrices above. |
| All five `source.*` keys | Ingestion selection, bounded staging, title-copy reads, quiet-period and consecutive deferral handling. Ingestion command cases cover default/custom thresholds, oversized input, title-read failure, and source preservation. |
| `recording.language`, all `engines.*` declarations and language overrides | Runtime engine selection and adapters; command pair-policy/outcome matrix, explicit language overrides, same-family opt-in, declared locality, Whisply argv/device/model, and engine deadlines. |
| `reconcile.*`, `readability.enabled` | Saved uncertainty classification, known-term reads and confirmations, and note rendering. Lexical command cases cover threshold/term effects and readability toggling while preserving lexical markers. |
| `note.profile`, `note.link_style`, `note.slug_max_chars`, all `note.obsidian.*` keys | Transcript, analysis, and brief target/rendering code, plus the vault index. Profile/link-style combinations, configured hubs, slug/rename behavior, pinned paths, and common-root wiki links have command/domain cases. |
| All `tags.*` keys | Synthesis tag selection, manual confirmation, vocabulary publication, and note rewrites. Command cases cover hold/write policy, default/custom caps, operator tags, configured vocabulary, and learning confirmed tags. |
| Both `relations.*` keys | Continuation selection and related-link filtering/rendering. Command cases cover session-gap boundaries, overlap refusal, default/custom related caps, and tag-policy independence. |
| All `synthesis.*` keys | Agent argv/stdin and deadline; initial grounding and later verification/rechecks. Command cases cover disabled/enabled transitions, input changes, changed grounding windows, and retained review decisions. |
| All `brief.*` keys | Evidence selection and scheduled discovery. Command/domain cases cover zero and full-range counts, omission counts, lookback, trigger enablement, and lead-window selection. |
| All `context.*` keys | Provider dispatch, scoped dam argv, Google credentials and read-only requests, normalized occasions/tasks. Collector cases cover exact scope and window selection, pagination/deadlines, private errors, and manual/upcoming command behavior. |
| All `share.*` keys | Pipeline automatic-redaction selection and redaction preparation/rendering. Command/domain cases cover automatic sharing independent of synthesis, source/date headers, timecodes/sections, pattern classes/masks, and omit/mark flags. |
| `handoff.allow_private` | Handoff authorization and provenance/content encoding. Command cases cover default private refusal and explicit allowance for each private stage. |
| `report.command`, `alert.command`, `alert.aggregate_after` | Logged event routing and aggregation. Command/adapter cases cover default-empty hooks, independent literal argv, configured aggregation threshold, bounded failures, and preserved work. |
| All `retention.*` keys | Standalone and pipeline selection/startup recovery. Command/adapter cases cover opt-in enablement, audio inclusion, each store's hold, zero/never behavior, expiration, and explicit regeneration. |

The broad runtime run in `/private/tmp/vpt-configuration-runtime-acceptance.log` completed with
**370 passed and 2 failed** across 14 command suites. Both failures were review-resolution calls
exceeding their unchanged 500 ms budget with empty stdout/stderr. The first full review rerun
(`/private/tmp/vpt-review-timing-rerun.log`) passed those two cases but failed the absent/invalid-ID
case at the same deadline (17 passed, 1 failed). A later full rerun in
`/private/tmp/vpt-review-current-acceptance.log` again passed the previous failures but timed out
while retrying a publication acknowledgement (17 passed, 1 failed). These remain unresolved timing
failures; this is not a claim that the whole command suite passes. Eight isolated absent/invalid-ID
invocations took 12–26 ms in `/private/tmp/vpt-review-startup-probe.log`, so they did not reproduce
the timeout or establish its cause.

The current acceptance run in `/private/tmp/vpt-brief-history-acceptance.log` passes all 37 brief
cases and both configuration matrices. Combined with the recorded typed-settings, parser, default,
runtime-consumer, and independent-store evidence, this completes the configuration row. Timing and
history/publication/privacy/output acceptance retain their own open rows: **100 checked and 7
unchecked items**.

## Brief history ordering and later-command export recovery

The new pipeline history matrix covers synthesis, verification, redaction, manual briefs, and
upcoming briefs in both human and JSON modes. A trigger that rejects required history stops the
command with a safe ledger error, preserving accepted artifacts, occasion/release state, and agent
call counts. Replacing `events.jsonl` with a directory preserves successful work and returns exactly
one export warning. Once the blocker is removed, a later mutating command exports every pending
record exactly once with its stable event ID and original occurrence time, creates a mode-0600 log,
and leaves no pending exports. This passed in
`/private/tmp/vpt-later-history-export-acceptance.log`.

Additional regressions exposed an ordering defect: manual briefs created missing store directories
and both manual/upcoming briefs could invoke a collector before saving their run start. The two
regressions fail for those behaviors in `/private/tmp/vpt-brief-history-order-red.log`. Brief mutation
now saves and exports its run start after safe startup recovery, before either preparation or context
discovery. Disabled triggers/providers and invalid scope/credentials still refuse during preflight;
previews remain observational. Discovery failure leaves a failed run without saving an occasion or
publishing a brief. SIGINT, SIGTERM, and SIGHUP leave an interrupted run for both writing paths.
The earlier failed-discovery test expected no ledger at all; its expectation was replaced with
saved failure history and unchanged inputs, with no accepted occasion or published artifact, as
required by specification section 8.7.1.

Briefs and released-copy recovery now share the existing run-finalization behavior through a generic
history helper. It retains original failures and committed IDs and stops after required-history or
unsafe-state failures. All three focused history/export tests pass in
`/private/tmp/vpt-brief-history-order-green.log`. The five changed production files are 73–102 lines;
no deadline, dependency, schema, or unsafe code changed.

The broader brief/history run completes with **215 passing tests** in
`/private/tmp/vpt-brief-history-acceptance.log`: library 81, briefs 37, configuration-refusal producers
11, configuration matrices 2, pipeline 72, and redaction 12. This includes both the new required-start
checks and interrupted/discovery-failure outcomes, plus release recovery after sharing finalization.

## Configuration privacy before command work

The new configuration-permissions matrix initially failed: `ingest` read a mode-0644 configuration
and proceeded to source access rather than refusing it. The regression is recorded in
`/private/tmp/vpt-config-permissions-red.log`. File loading now checks the already-open directory and
file before reading configuration content. The directory must be owned by the current user with
mode 0700, and the file must have that owner with mode 0600. Existing containment/type errors retain
their established classification. Permission refusal is a safe configuration error; VPT never repairs
the selected file or its containing directory while loading it.

All **104 permission-refusal invocations** pass across 26 command forms in both output modes,
preserving the whole fixture's bytes and metadata. The existing 260 precedence/version/value cases
also pass in `/private/tmp/vpt-config-permissions-green.log` (three tests). File I/O was split from
TOML decoding so the loader and its new responsibilities each remain under 150 lines. The initial
refactor missed the `Table` import; compilation identified it and it was corrected. The file-loading
fixture previously depended on a shared temporary parent, which is now made explicitly private.
The version/observation fixtures likewise use private selected configuration paths so they continue
to test their original behavior rather than stopping at the new guard. All **104 configuration
adapter tests** pass in `/private/tmp/vpt-private-config-adapters-final.log`.

The command compatibility run in `/private/tmp/vpt-private-config-command-acceptance.log` passes
library 81, configuration 3, doctor 38, show/list/storage 31, symlink 17, and version 10. This doctor
run passes every debug case at the unchanged 500 ms limit, including the previously failing active
native-writer case; it does not establish the cause of earlier intermittent timing failures. Setup
passes 12 and fails 6 because the sandbox rejects `/dev/tty`, preventing the owned pseudo-terminal
fixtures from reaching their prompts. Rerunning only setup with approved terminal access passes all
**18 tests** in `/private/tmp/vpt-private-config-setup-terminal.log`. No deadline was changed.

The independent-store pipeline case passes again after the stricter loader and its file-I/O split
in `/private/tmp/vpt-private-config-stores-final.log`. Workspace Clippy with all targets, `dev-tools`,
and warnings denied passes in `/private/tmp/vpt-history-privacy-clippy.log`. Formatting verification
passes in `/private/tmp/vpt-history-privacy-format-check.log`. The command/library build, test, and
lint processes ran sequentially; CLI and SQLite tests used one thread. The roadmap remains
**100 checked and 7 unchecked**. These changes do not complete the remaining history/export,
publication, command-output, timing, or shared-state/path requirements.

## Remaining command history and output acceptance

The new history guard matrix exercises 17 ledger-backed mutating forms: ingest, pipeline,
transcription, note rewrite, synthesis, verification, review resolution, term/tag/rejected-tag/relation
confirmation, existing/manual/upcoming briefs, transcript/analysis redaction, and retention. Both
output modes refuse a trigger that prevents run-start persistence and an unsafe state directory.
It preserves recording/artifact/occasion/history state, invokes no engine/agent/collector, and returns
no false completed identifiers. The unsafe-state cases preserve the full filesystem snapshot. The
required-history cases preserve every file outside state, including an unprocessed source recording.
The initial matrix passes in `/private/tmp/vpt-history-command-guards.log`; the added unprocessed-source
and outside-state snapshot assertions are included in the broader command run below.

The success-output matrix covers those 17 mutating forms and nine observational forms (review,
occasions, three path variants, and four handoff stages) in human and JSON modes. Observational
invocations preserve every fixture file's bytes and metadata. The first run identified an actual
protocol omission: upcoming briefs returned an unversioned object instead of the required
`vpt.result/1` document. `/private/tmp/vpt-command-output-matrix-red.log` records that failure.
Upcoming briefs now use the shared result envelope with `command` naming `brief`. The next run reached
redaction and exposed a fixture mistake: testing both modes against the same explicit redaction
correctly produces a duplicate refusal. Each mutating mode now has its own fixture. Both output
matrix tests pass in `/private/tmp/vpt-command-output-matrix-final.log` (52 command invocations).

A separate regression in `/private/tmp/vpt-synthesis-finish-diagnostics-red.log` found that a failed
final run-history commit discarded diagnostics from the already accepted proposal. Synthesis now
retains safe proposal diagnostics and the original completed identifiers when notification preparation
or the final history commit fails. All five diagnostic cases pass in
`/private/tmp/vpt-synthesis-finish-diagnostics-green.log`, including human and JSON checks of the new
failure path and unchanged published analysis bytes.

Current transaction validation in `/private/tmp/vpt-history-ledger-acceptance.log` passes 694 ledger
tests and has one 500 ms timeout in the durable verification-replacement contract. Its child had
entered the scenario before timing out, so the prior startup-only sample does not explain it. That
exact case passes at the unchanged deadline in `/private/tmp/vpt-ledger-verification-timing-rerun.log`.
The failure remains recorded as an unresolved intermittent timing issue. All 37 exporter tests
pass in `/private/tmp/vpt-history-export-acceptance-final.log`, covering export-before-hook ordering,
warning preservation, unsafe state and required-history stops, replay identities, private logs,
incomplete tails, and recovery. All 67 application tests pass in
`/private/tmp/vpt-history-application-acceptance.log`.

The broader run in `/private/tmp/vpt-history-output-command-acceptance.log` passes all **149 tests**:
brief 37, pipeline 77, and synthesis 35. It includes the stronger 68-invocation guard matrix with a
pending source recording and exact preservation of files outside state, plus the 52 success-output
invocations and the final-history diagnostic regression.

Combined exporter/startup faults exposed two additional command gaps. Transcription and retention
returned the correct `publication_target_modified` refusal but discarded the warning collected while
recovering an unavailable log. Both failures are reproduced independently in
`/private/tmp/vpt-startup-warning-propagation-red.log`. A unit regression in
`/private/tmp/vpt-startup-warning-dedup-red.log` also reproduces duplicated warning text when nested
startup error handling has already attached it. A shared error-attachment helper now adds each safe
warning once, retaining other diagnostics. Transcription uses it for both recovery and startup errors.
Retention reuses the shared note/release publication-repair function and attaches accumulated warnings
on error, preserving completed IDs from earlier repair work. Both command regressions pass in
`/private/tmp/vpt-startup-warning-propagation-green.log`, covering human and JSON output, unchanged
operator replacement bytes, preserved pending publication, and no engine rerun.

| History requirement | Current acceptance evidence |
| --- | --- |
| Run start before new work | All 17 mutating forms refuse a failed run-start insert without new ledger work, external calls, or changes outside state; separate brief regressions cover collection and store creation. |
| Ingestion and retention state/history together | Application ingestion/retention tests, both ledger contract implementations, and command abort/recovery cases cover saved recordings, source associations, deferrals, intents, observed completion, and partial failure. |
| Accepted text, engine output, notes, and review | Ledger transcript/engine-output/note/flag contracts couple accepted state, render bytes, publication intents, and history; failed final facts roll back the corresponding logical transaction. Command cases preserve completed identities through later publication and output failures. |
| Confirmation, proposals, and verification | Term/tag/relation/proposal/verification contracts cover combined vocabulary/note/finding updates, missing or mismatched history, rollback, stale input, and preserved review decisions. The durable verification timeout and focused pass remain disclosed above. |
| Occasion, brief, and released-copy completion | Occasion/brief/release contracts distinguish saved bytes or reservation from completed publication, coupling briefed markers and events only to confirmed publication. Command cases cover interrupted acknowledgement and recovery without regenerated source or repeated hooks. |
| Export after saved history | Command composition uses `history::commit`; ingestion and retention use their corresponding receipt-to-sink boundaries. The exporter tests require durable export before hook invocation and preserve original hooks after an optional export failure. |
| Optional export failures and required-history stops | Command matrices and existing early-stage cases preserve result/status with one warning, stop on required history or unsafe state, and recover pending records with stable IDs and original occurrence times. The combined startup-refusal regressions cover the formerly missing warning paths. |
| Final output contract | All 26 forms emit one versioned document or nonempty human output on success. Configuration, history, unsafe-state, stage-failure, and publication cases cover errors 1/2/3, empty stdout, named refusals, safe diagnostics, and committed identifiers. Broken-stream tests retain completed work in the error path. |

The post-fix run in `/private/tmp/vpt-startup-warning-command-acceptance.log` passes all **143 tests**:
library 82 (including warning deduplication), configuration-refusal producers 11, retention 19, and
transcription 31. Existing success, partial-failure, hook/export-warning, and broken-output behavior
remains covered. The five changed production files remain below 150 lines. No dependency, unsafe
code, or existing deadline changed.

Final export-command validation passes all five history/export cases in
`/private/tmp/vpt-history-export-commands-final.log`. Workspace Clippy with all targets, `dev-tools`,
and warnings denied passes in `/private/tmp/vpt-history-output-clippy-final.log`. Formatting
verification passes in `/private/tmp/vpt-history-output-format-check.log`; `git diff --check` passes.
Build, test, and lint processes ran sequentially, with one test thread for CLI/SQLite runs and no
changed deadlines.

The command-history, per-commit export, and shared human/JSON output rows are complete. The roadmap
now has **103 checked and 4 unchecked items**: descriptor staging/archive publication, intermittent
timing failures, durable publication races, and shared configuration/state/path protections. The
ledger-suite timeout and prior review/doctor timing failures remain open; focused passing reruns do
not establish their cause or resolve their intermittency. The full goal remains active.


## Retained archive descriptor through publication

The archive publisher previously received only a pathname. A deterministic test changed staging
between container inspection and publication; ingestion saved a recording whose digest and capture
metadata described the original bytes while the published archive held different bytes. Separate
faults during file synchronization and directory synchronization also reached successful publication.
The failures are recorded in `/private/tmp/vpt-archive-publication-retained-red.log` and
`/private/tmp/vpt-archive-publication-sync-red.log`.

The archive port now receives the inspected handle together with the staged size and digest.
Publication verifies the retained file before and after file synchronization, syncs that same open
file, uses the existing exclusive rename, and verifies the destination against the retained file
and expected digest after directory synchronization. It checks private mode and a single link.
The check after rename permits rename's ctime change while requiring the original size, mtime,
identity, and content. A typed `ArchiveError::Changed` prevents application cleanup from moving
changed staging to the Trash; it reports the existing safe synchronization failure and records no
ingestion. Failure mapping now lives with the ingestion failure handling. Changed production
modules remain below 150 lines; no unsafe code or dependency was added.

Five ingestion cases cover changes after hashing, changes or replacement during inspection, and
changes or byte-identical replacement after inspection. They require unchanged source bytes, no
recording/seen rows, no published archive, no Trash attempt, preserved staging, and one safe failure
event. Adapter boundary tests additionally inject content and inode changes during file sync and
after rename during directory sync, requiring refusal and preservation of the changed bytes.

Validation:

- Archive unit cases: **33 passed**, `/private/tmp/vpt-archive-publication-acceptance-final.log`.
- All ten ingestion adapter suites: **81 passed**,
  `/private/tmp/vpt-archive-ingestion-acceptance-final.log`.
- The first archive acceptance run found two assertions expecting the later target path after root
  replacement. The retained-handle fixture now captures the handle before replacement, and the
  after-sync case expects the earlier staging-path refusal. The original log is retained at
  `/private/tmp/vpt-archive-publication-acceptance.log`.
- Moving failure mapping initially left two grouped imports pointing at its old module. Compilation
  caught these; both imports were fixed before the passing ingestion run. The original log remains
  at `/private/tmp/vpt-archive-ingestion-acceptance.log`.

This closes the reproduced inspection-to-publication and synchronization gaps. It does not make the
name check and rename atomic: another process can still replace a staging name after the final check,
and detection after rename does not undo that rename. The staging/publication row remains unchecked.
The roadmap still has **103 checked and 4 unchecked items**, and the full goal remains active.

The application library passes **67 tests** in
`/private/tmp/vpt-archive-application-acceptance.log`. Ingest and full-pipeline command acceptance
passes **114 tests** (35 ingest, 79 pipeline) in
`/private/tmp/vpt-archive-command-acceptance.log`. Combined with the archive and ingestion adapter
runs above, this is **295 passing tests**. Tests ran sequentially with one test thread and
`TMPDIR=/private/tmp`; existing command and database deadlines were unchanged.

Workspace Clippy with all targets, `dev-tools`, and warnings denied passes in
`/private/tmp/vpt-archive-publication-clippy.log`. Formatting verification passes in
`/private/tmp/vpt-archive-publication-format-check.log`. `git diff --check` also passes.
All build, test, lint, and formatting processes finished before the next check started.


## Shared state/path acceptance and setup permission refusal

A new command matrix exercises all 26 later command forms in human and JSON modes with a symbolic
link or a second hard link at each of `vpt.db`, `vpt.db-journal`, `vpt.db-wal`, and `vpt.db-shm`.
All **416 invocations** refuse with exit 3, preserve the external referent's bytes and metadata,
leave source/artifact/engine/context files unchanged, and return no completed identifiers. The nine
observational forms additionally preserve a complete immediate filesystem snapshot. The eight
matrix tests pass in `/private/tmp/vpt-state-path-command-matrix.log`. The read-only command list is
shared with output acceptance so both matrices cover the same forms.

The underlying database containment suite passes **132 tests** at its unchanged deadlines in
`/private/tmp/vpt-state-database-containment-acceptance.log`. This covers retained roots and companion
operations, refusal of linked/nonregular files, write and observation policies, WAL interoperability,
locks, shared memory, root replacement, cleanup, and resource lifetime. Passing this run does not
resolve the previously recorded intermittent timing failures.

The shared-directory requirement exposed a setup defect: `PlannedDirectory::create` unconditionally
changed an existing directory to mode 0700. Two adapter regressions failed before the fix in
`/private/tmp/vpt-setup-shared-directories-red.log`, and command acceptance failed because setup reached
the terminal instead of refusing the existing shared directory in
`/private/tmp/vpt-setup-shared-directory-command-red.log`.

Setup now checks existing final directories at preparation and again when using them, through the
shared descriptor-based ownership/mode validator. Only newly created directories receive mode 0700.
An unsafe existing home, state, or store directory produces `nonprivate_directory` before prompting
or writing configuration; a directory created or made shared after preparation is also preserved and
refused. Configuration-directory refusals keep their existing `nonprivate_config_directory` rule.
The former test expecting setup to repair a shared home was corrected to require preservation of an
already-private directory's metadata; separate regressions verify refusal of shared directories.
`--force` still permits configuration-file replacement and does not authorize chmod of directories.

All **13 setup adapter tests** pass in `/private/tmp/vpt-setup-shared-directories-green.log`. The new
command regression passes all eight home/state/audio/released and human/JSON scenarios in
`/private/tmp/vpt-setup-shared-directory-command-green.log`. The complete setup suite passes **19 tests**
in `/private/tmp/vpt-setup-private-directory-acceptance.log`, including owned controlling-terminal
cases. That suite ran with sandbox escalation solely because its existing tests require `/dev/tty`.
The combined configuration, contained-filesystem, and store adapter acceptance passes **222 tests**
in `/private/tmp/vpt-shared-path-adapter-acceptance.log`.

### Publication platform investigation

The local macOS SDK's `rename(2)` documents atomic swap and exclusive destination creation; its
`renameatx_np` signature accepts names, directory descriptors, and flags, with no expected digest or
expected file-identity parameter. From that interface, a digest check followed by rename cannot
itself be an atomic conditional replacement.

Apple's [lease implementation](https://raw.githubusercontent.com/apple-oss-distributions/xnu/main/bsd/vfs/vfs_subr.c)
checks the private `com.apple.private.vfs.file-leases` entitlement before granting a lease. A local
`F_SETLEASE(F_WRLCK)` probe returned `EPERM`, recorded in `/private/tmp/vpt-file-lease-capability.log`.
The probe used a temporary file and released it without changing any existing project artifact.
The [rename implementation](https://raw.githubusercontent.com/apple-oss-distributions/xnu/main/bsd/vfs/vfs_syscalls.c)
validates the supported flags and delegates to pathname-based rename.

A product clarification is pending: whether a final-boundary conflict may preserve both versions in
a reported conflict copy, or whether the operator's original pathname must remain untouched
throughout. No change to that publication contract has been assumed or implemented while waiting.


Shared command acceptance passes **15 tests** (3 configuration and 12 focused pipeline cases) in
`/private/tmp/vpt-shared-path-command-acceptance.log`. This reruns configuration precedence/version
and permission refusal, shared-parent preservation during publication to independent stores,
observational output, required-history and unsafe-state guards, and the full linked-database matrix.
Alongside the 222 adapter, 132 database, and 19 setup cases, the relevant acceptance totals
**388 passing tests**.

| Shared requirement | Current evidence |
| --- | --- |
| Private configuration and credentials | All command forms reject unsafe configuration file/directory modes without chmod or work; config loader checks the retained file's owner and 0600 mode and the directory's owner and 0700 mode. |
| Private state before new mutations | All 17 mutating forms stop on unsafe state, preserve the full snapshot, and start no engines/context/agents. State validation uses the retained directory and effective UID. |
| Database and companion containment | 416 real command invocations refuse symbolic/multiple links across main, journal, WAL, and SHM files. All 132 database adapter cases pass, including retained-root replacement, mappings, locks, synchronization, and final close. |
| Preserve shared folders | Setup refuses existing or later-substituted shared final directories without chmod. Root/store tests preserve shared parents, configured vaults, and existing store modes while setting required private file modes. |
| Create new private directories | Setup terminal cases and configuration/store cases create missing private leaves as 0700, keep existing parents unchanged, and place all seven stores at their independent configured roots. |
| Protect source and artifact paths | Source-overlap, traversal, linked/nonregular-leaf, and retained-root tests refuse unsafe operations; the command matrices preserve source and artifact snapshots and external referents. |

The separately tracked final publication-name races and intermittent command/ledger timing failures
are not resolved by these shared privacy/path checks.

Workspace all-target Clippy with `dev-tools` and warnings denied passes in
`/private/tmp/vpt-shared-path-clippy.log`; formatting verification passes in
`/private/tmp/vpt-shared-path-format-check.log`. `git diff --check` passes. All check processes ran
sequentially and finished; no existing deadline changed. The shared privacy/state/path roadmap row
is complete. The current checklist is **104 checked / 3 unchecked**: archive staging/publication,
durable publication conflicts, and intermittent command/database timing. The full goal remains active.


## Verification synchronization timing and doctor phase investigation

The verification scenario that previously timed out now emits test-only timings for retained-root
opening, ledger/database opening, operations, closing, and file/directory synchronization. A shared
trace assertion checks phase order, numeric durations, required synchronization markers, and absence
of the private fixture sentinel. It reports parent elapsed time and the eight longest completed
phases. These markers remain within the existing bounded child capture; production synchronization
and all deadlines are unchanged.

TDD evidence:

- `/private/tmp/vpt-verification-phase-trace-red.log`: the new verification trace failed because
  verification children did not emit `root.open.start`.
- `/private/tmp/vpt-verification-phase-trace-green.log`: the new trace passes after enabling that
  scenario family; parent elapsed time was 338 ms in this run.
- `/private/tmp/vpt-verification-sync-trace-red.log`: requiring synchronization timings failed on
  the missing `directory.sync.end` marker.
- `/private/tmp/vpt-verification-sync-trace-green.log`: the trace passes after timing the actual
  synchronization calls. This run took 265 ms; its longest directory and file syncs took 8 and 5 ms.
- `/private/tmp/vpt-ledger-sync-trace-acceptance.log`: all **828 database/ledger tests pass**
  sequentially, including the new trace case. This does not identify the earlier timeout's cause.

`/private/tmp/vpt-doctor-review-timing-current.log` passes all **18 review tests** and **37 of 38
doctor tests**. The active-native-writer doctor case still exceeds its unchanged 500 ms deadline
with empty output. A host-access run of that exact case completed in 90 ms; it ended before sampling
was needed (`/private/tmp/vpt-doctor-active-sampled.log`). That comparison alone proves no cause.

Temporary probes narrowed separate failures:

- `/private/tmp/vpt-doctor-main-entry-current.log`: the child emitted its main-entry marker before
  timing out, so that failure was not entirely before Rust execution.
- `/private/tmp/vpt-doctor-compact-phases-current.log`: configuration completed in 8,222 microseconds;
  the helper version check began at 9,321 microseconds and had not returned before the timeout.
- `/private/tmp/vpt-doctor-helper-phases-current.log` and
  `/private/tmp/vpt-doctor-helper-phase-rebuilt.log`: freshly rebuilt command runs timed out without
  the main-entry marker. A subsequent invocation completed and failed only the expected assertion
  against the deliberately added stderr markers (`/private/tmp/vpt-doctor-helper-phases-warm.log`).
- A later process-inspection attempt found no live matching test executable and produced no stack
  sample. It supplies no additional evidence about the timeout.

All temporary main, doctor, and subprocess probes were removed. The doctor fixture now includes
unexpected stderr in its assertion failure to improve future diagnosis. No executable-loading cause,
helper failure cause, or timing repair is established. The publication-policy clarification remains
pending, and the roadmap remains **104 checked / 3 unchecked**.

The final host-access comparison, after removing all temporary probes, also failed at the original
deadlines: **37/38 doctor** and **16/18 review** cases passed in
`/private/tmp/vpt-doctor-review-host-current.log`. The failures were the active-native-writer doctor
case, correction publication with known terms, and publication-acknowledgement recovery. All three
had empty captured streams at 500 ms. The managed sandbox alone therefore does not explain the
observed failures; no passing comparison has been substituted for those failed acceptance results.

Final checks: workspace all-target Clippy with `dev-tools` and warnings denied passes in
`/private/tmp/vpt-timing-diagnostics-clippy.log`; formatting verification passes in
`/private/tmp/vpt-timing-diagnostics-format-check.log`; `git diff --check` passes. All build, test,
and Clippy processes are terminal and ran sequentially. The goal remains active.


## Reuse the initialized WAL index within a writable ledger

A bounded watcher sampled only descendants of the owned command-test process after they had been
observed for at least 100 ms. The run passed all 38 doctor cases and 17/18 review cases. The failing
review child (39509) exceeded its unchanged 500 ms deadline. Its two captured stack samples include
history-export acknowledgement opening a SQLite connection, setting journal mode, rebuilding the
WAL index, and reading WAL bytes. The other seven sample files contain no captured stacks. This is
a location observed in a failing process, not a complete time profile or proof of every timeout's cause.
Evidence is in `/private/tmp/vpt-live-timing-932_ew2u/tests.log`, `sampling.log`, and `stack-39509.txt`.

The [SQLite WAL documentation](https://sqlite.org/walformat.html#the_wal_index_or_shm_file) explains
that the index coordinates live connections and is normally truncated by the first connection after
all clients have closed. VPT's per-operation connections consequently reinitialized the index and
recovered the WAL repeatedly, including between history acknowledgements in one command.

The new regression proved that even reading the schema through a writable ledger reset the WAL
index header. A second regression proved there was no lifetime lock between operations. Both failed
in `/private/tmp/vpt-wal-lifetime-red.log`; the observational-ledger case already passed.

A writable `SqliteLedger` now retains one idle observational SQLite connection. It is opened while
the migration connection still holds the initialized index, and completes its read before use. Each
operation continues to open and explicitly close its own connection, preserving commit/close error
reporting, root validation, transaction boundaries, and durability. The retained connection holds no
transaction or WAL read mark between operations and is dropped with the ledger. Observational
ledgers retain their existing behavior. No unsafe code, dependencies, schema changes, or deadline
changes were added; the lifetime module is 50 lines.

The three initial cases pass in `/private/tmp/vpt-wal-lifetime-green.log`. Native checkpoint and
subsequent-commit coverage initially had an incorrect fixture column name (`intended_digest` instead
of `intended`), recorded in `/private/tmp/vpt-wal-lifetime-interop.log`; that test fixture was corrected.
All **832 database/ledger tests pass** in `/private/tmp/vpt-wal-lifetime-adapter-acceptance.log`.
All **five focused lifetime tests pass** in `/private/tmp/vpt-wal-lifetime-recovery-acceptance.log`,
including a new process-termination case that recovers committed WAL after the previous owner dies.
The tests also verify that native checkpoint truncation and writes succeed while the ledger is idle,
that subsequent reads see the native commit, that no transaction/read-mark slots remain locked, and
that dropping the ledger releases the lifetime lock.

### Bind reused WAL indexes to the opened database files

The full command run completed before this additional protection: **653 passed, 0 failed, 2
installation cases ignored** across 28 suites in
`/private/tmp/vpt-wal-lifetime-command-acceptance.log`. Doctor passed all 38 cases and review passed
all 18 at their original deadlines. The ignored cases require an installed release through
`VPT_TEST_INSTALL_ROOT`. This passing run does not explain the previously observed pre-main and
helper delays, so the timing row remains open.

Retaining the WAL index requires checking that subsequent connections open the same main, WAL,
and shared-memory files. Three new regressions replaced each file with another regular inode.
All three originally returned a schema version instead of refusing the replacement:
`/private/tmp/vpt-wal-lifetime-replacement-red.log`.

The VFS now records identities from the descriptors it actually opens. Subsequent connections check
all three identities before opening SQLite and check each acquired descriptor before changing its
permissions or using it. Missing retained files are refused without recreation, including exclusive
create callbacks. History writes use the same protected connection path. This adds no unsafe code,
dependencies, schema changes, or deadline changes.

Descriptor checks alone initially refused replaced companions after opening the main file, which
changed metadata. The full preflight removed those writes; evidence is in
`/private/tmp/vpt-wal-lifetime-pinned-refusal.log` and
`/private/tmp/vpt-wal-lifetime-pinned-preflight.log`. Tests also replace or remove files after the
preflight and verify that the later descriptor acquisition refuses them without changing permissions
or recreating the missing file.

The expanded run passed 833 cases but failed six newly added teardown assertions. Each operation
preserved the exact snapshot; dropping the retained mapping changed only the original shared-memory
file's ctime. An isolated macOS mapping experiment reproduces this after earlier mapped writes,
including after fsync, without SQLite. See `/private/tmp/vpt-readonly-unmap-metadata.log` and
`/private/tmp/vpt-wal-identity-drop-metadata.log`. The teardown assertion now permits only that ctime
change on the original mapped inode. It still checks all other metadata and bytes, including the
complete replacement-file snapshot; the operation itself retains an exact snapshot assertion.

All **13 focused lifetime and late-replacement cases pass** in
`/private/tmp/vpt-wal-identity-focused-green.log`. They cover both read and history-write refusal,
missing/replaced files, late acquisition races, native checkpoints/commits, and owner termination.
The complete database/ledger run then passed **841 tests** at unchanged deadlines in
`/private/tmp/vpt-wal-identity-adapter-green.log`.
Command acceptance of the final identity checks passed all **70 doctor, review, configuration, and
configuration-refusal tests** in `/private/tmp/vpt-wal-identity-command-acceptance.log`.
The eight command path-refusal matrix cases also passed, covering **416 command invocations**, in
`/private/tmp/vpt-wal-identity-state-paths.log`.

A final extension to the late-removal regression caught SQLite's existence-query path returning
ordinary absence for a missing pinned file. `/private/tmp/vpt-wal-identity-access-red.log` records the
failure. That path now returns the same typed containment error, so SQLite cannot treat the missing
WAL as an ordinary absent companion and skip the identity refusal.

The subsequent full run passed 838 cases and hit three timing failures in
`/private/tmp/vpt-wal-identity-final-adapter.log`: the synthesis/reviewed-claim and verification
replacement cases reached their 500 ms deadlines after file synchronization, and the within-batch
recording-conflict case timed out during owned-child cleanup. This is further evidence that retaining
the WAL index has not resolved all timing failures. No deadlines or cleanup budgets were changed.

The writable VFS also reset mode 0600 on every successful open, even when the file was already private.
The new full-snapshot regression in `/private/tmp/vpt-private-reopen-red.log` proves that these opens
changed metadata. Permission repair now runs only when the acquired file's permission/special bits
differ from 0600. Existing permission-repair coverage remains in place. This removes unnecessary
metadata changes; it is not evidence that all synchronization or startup delays are fixed.
All **842 database/ledger tests pass** with the existence-query refusal and permission change in
`/private/tmp/vpt-private-reopen-adapter-acceptance.log`, at unchanged deadlines.
The final command run also passes all **70 doctor, review, configuration, and configuration-refusal
tests** in `/private/tmp/vpt-private-reopen-command-acceptance.log`.
Workspace/all-target Clippy with `dev-tools` and warnings denied passes in
`/private/tmp/vpt-private-reopen-clippy.log`. Formatting and `git diff --check` pass; every production
file touched by this change stays below 250 lines (the largest is 215). The roadmap remains
**104 checked / 3 unchecked**. Publication races still require the pending conflict-policy decision,
and intermittent timing remains open despite the latest passing runs.
