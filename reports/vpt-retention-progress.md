# Retention command progress

The full roadmap in `docs/vpt-remaining-work.md` remains active. This report tracks audio retention and
the extension to later stores.

## Implemented

- Routed `retention run` and its dry run to the application service with human and JSON output.
- Opened dry-run state observationally, without a write lock, migrations, log recovery, hooks, or Trash calls.
- Added private-state validation and a shared write-lock composition boundary.
- Connected log recovery, required run history, optional report hooks, and visible export/hook warnings.
- Reconciled pending retention intents before new candidates, preserving completed identities and actual move counts.
- Kept symlinks and ledger paths outside store authority with explicit reasons; regular candidates in the same store remain eligible.
- Mapped replacement refusal to the specified `retention_target_modified` rule.
- Preserved committed identities after later helper failure and avoided JSON serialization panic on an invalid UTF-8 path.
- Split filesystem inventory, digesting, and publication into separate modules; moved retention report tests out of production source.

## Evidence recorded during implementation

Compiling behavioral tests failed before command routing, pending-intent reconciliation, the specified refusal rule, invalid UTF-8 output handling, redirected-path reporting, and outside-authority reporting were implemented.

`TMPDIR=/private/tmp cargo test --locked -p vpt --features dev-tools --test retention -- --test-threads=1`
passed all 18 CLI cases, including a later helper failure and event-log export failure followed by recovery. Recovery did not repeat Trash moves or resend hooks.

The invalid UTF-8 output unit test passed after reproducing the serialization panic. Temporary paths must use `/private/tmp`; `/var` in the default macOS temporary path is a symlink refused by checked-root fixtures.

The first workspace run passed the retention application/adapter tests. It failed setup terminal tests because the sandbox denied `/dev/tty`, and it failed the then-unimplemented ingestion dry-run tests. All 18 setup tests passed when rerun outside the sandbox.

## Verification still required

Concurrent changes to ingestion and shared event/clock composition were observed during this work. Later workspace/clippy runs encountered intermediate missing modules and duplicate ingestion methods. Re-run the full gates after those edits settle. The final retention refactor uses the shared `SystemClock` and event helpers. The 18-case CLI run passed with these shared helpers; Clippy also passed before subsequent concurrent ingestion edits.

The shared mutation lock now waits up to five seconds and reports exit 1 on contention, as required by section 4.4. Both tests first failed, then passed: a waiting writer acquires after release, and a bounded failed attempt returns an operational error.

The latest completed full workspace run failed only `ingest_receipts::ordinary_failure_saves_one_safe_event_with_original_time` (observed occurrence time was one second later than expected). Ingestion is being modified concurrently. The presenter now includes completed identities in human partial-failure output. Both presenter tests pass, including invalid UTF-8 handling.

The 30 retention adapter/application integration tests passed after the filesystem split, along with all 798 adapter library tests. Audio-retention requirements have corresponding implementation and behavioral evidence; workspace-wide completion remains unproven because ingestion and later stages are unfinished.

## Latest workspace run

`/private/tmp/vpt-workspace-verification.log` records the latest full run after engine-decoder changes. It failed five targets: doctor, ingest, retention, the adapter library, and ingest recovery. Doctor/ingest/retention integration failures and the adapter title-copy failure hit their existing 500 ms watchdogs. Ingest recovery also failed a new assertion that an earlier committed renamed-source identity appears in a later collision error.

The 30 retention application/adapter cases and all 47 protocol cases passed in this same run. The earlier focused 18-case retention CLI run passed, but final workspace verification is not green. Keep the existing deadlines and revalidate with ongoing work coordinated; do not infer an unmeasured cause for the timing failures.

## Ownership metadata for later stores

Added the `RetentionArtifacts` port and implementations for SQLite and memory. They return audio,
engine outputs, transcript notes, analysis notes, briefs, private redaction reports, and releases.
Owners distinguish recording identities from occasion identities. Each entry contains only its kind,
owner, pinned path, digest, and publication/expiration status. Pending publication takes precedence over
an expired note stage. Releases and reports remain pending until both files have finalized.

The query is bounded to 65,536 entries and 16 MiB of metadata. It rejects duplicate paths, malformed
identities/digests/paths, and owned note/brief paths that disagree with their owner's pin. SQLite reads
all metadata in one snapshot without decoding or returning private bodies and retains the older-schema
read-only path without migration. The memory implementation indexes pending paths and recording owners.

Adding artifact kinds exposed an assumption in the existing journal completion: a transcript intent
could mark matching audio as expired. Both completion implementations now enforce the artifact kind,
and recovery rejects a mismatched kind before touching the file or calling Trash. This initial increment
kept execution audio-only. Later-store validation, journaled moves, and expiration timestamps are
covered by the increments below.

TDD and verification:

- `/private/tmp/vpt-retention-inventory-red.log`: four tests failed on the absent inventory and the
  missing kind check in both ledgers.
- `/private/tmp/vpt-retention-inventory-checked.log`: the first ten ownership and bounds tests passed.
- `/private/tmp/vpt-retention-kind-recovery-red.log`: recovery rejected the wrong kind using a path
  error; it now uses the named modified-target refusal and preserves the file and pending intent.
- `/private/tmp/vpt-retention-ledger-pins-red.log`: 89 checks passed and the new inconsistent-pin
  check failed. The query now checks transcript, analysis, and brief owner pins.
- `/private/tmp/vpt-retention-ledger-green.log`: all 90 selected retention ledger tests passed,
  including existing atomic history, rollback, post-commit failure, journal, and new inventory cases.
- `/private/tmp/vpt-retention-kind-recovery-green.log`: all 31 retention execution tests passed,
  including wrong-kind recovery, unchanged/missing/modified targets, and combined recovery/new moves.

Recovery orchestration, per-intent validation, and effect reporting now live in separate modules.
SQLite completion is also separate from intent preparation. These production modules are each under
150 lines. No deadline or durability setting was changed. The roadmap remains at 54 complete and
53 open; later-store retention is still unchecked.

The 67 application unit tests also pass in `/private/tmp/vpt-retention-inventory-application.log`.
The next increment below adds typed journal owners and durable expiration timestamps. Filesystem
validation and reconciliation for later stores remain open.

Workspace Clippy passed with all targets, `dev-tools`, and warnings denied in
`/private/tmp/vpt-retention-inventory-clippy.log`. Formatting and `git diff --check` passed.
The previously recorded review and synthesis-alert timing failures remain unresolved; these targeted
results do not establish a clean full-workspace test gate.


## Typed journal owners and durable artifact expiration

`NewIntent` and `RetentionIntent` now carry an `ArtifactOwner`, preserving occasion ownership for
briefs and recording ownership for other artifacts. Both ledgers reject owner/kind mismatches before
allocating an intent or appending history. SQLite bounds journal text before allocation and decodes
owners by artifact kind. Its legacy `recording` column remains compatible with older journals.

Schema 15 adds `artifact_expirations`, keyed by pinned path and bound to artifact kind, owner, and
saved digest. Both ledgers atomically save the first expiration time, update transcript/analysis
stage state, complete the journal, and append effect/work history. Repeated completion preserves the
original time and adds no history. A second intent for the same artifact also preserves the first
expiration time. Accepted transcript data, render inputs, pinned paths, and release provenance remain
available. Brief completion does not invent a recording identity.

Notes and briefs may contain operator prose, so the journal digest records the validated current
bytes while the expiration marker retains the ledger's saved digest. The application must validate
identity, stage, and markers before preparing these intents. The next increment below connects that
validation to application execution. Other artifact digests must match the saved ownership metadata at completion.

Deliberate note/brief regeneration clears its expiration marker in the producer transaction. Pending
retention blocks engine output republication, matching the existing note/brief protections. Reads
refuse expiration markers that disagree with owned metadata. Version 14 remains readable without
migration, and writable migration preserves pending occasion-owned intents.

Behavioral evidence:

- `/private/tmp/vpt-retention-owner-red.log`: three typed-owner failures before implementation.
- `/private/tmp/vpt-retention-owner-green.log`: 93 selected retention tests passed.
- `/private/tmp/vpt-artifact-expiration-red.log`: both ledgers refused later-artifact completion.
- `/private/tmp/vpt-artifact-expiration-green.log`: all-kind expiration and SQLite reopen passed.
- `/private/tmp/vpt-artifact-regeneration-red.log`: four failures exposed retained timestamps and
  engine republication during pending retention.
- `/private/tmp/vpt-artifact-retention-green.log`: 103 retention tests passed, including atomic
  rollback, digest/owner/path/effect refusals, regeneration, and the existing audio behavior.

The ledger-extension roadmap row is complete: all listed producer state and provenance are persisted,
and retention preserves accepted render inputs. The roadmap now has 55 checked and 52 unchecked tasks.
Later-store execution is covered by the next increment below. Full automatic-stage terminal
acceptance remains unfinished.

Additional verification:

- `/private/tmp/vpt-expiration-ledger-suite.log`: all 681 ledger tests passed, including schema
  migrations, rollback, version-14 observational reads, producer contracts, and expiration.
- `/private/tmp/vpt-expiration-preserved-inputs.log`: all 27 inventory/expiration tests passed after
  adding exact preservation checks for transcript/analysis notes, brief packs, and release payloads.
- `/private/tmp/vpt-artifact-expiration-execution.log`: all 31 retention execution tests passed.
- `/private/tmp/vpt-artifact-expiration-application.log`: all 67 application unit tests passed.
- `/private/tmp/vpt-artifact-expiration-clippy-final.log`: final workspace Clippy passed with all
  targets, `dev-tools`, and warnings denied. Formatting and `git diff --check` also passed.

All 18 touched adapter production modules are within 150 lines. No deadlines were changed. The
previously recorded review and synthesis-alert timing failures remain unresolved; this increment does
not establish a passing full-workspace test run.


## Retention execution and recovery across all stores

The later-store retention roadmap row is complete. The roadmap now has 56 checked and 51 unchecked
tasks. Standalone retention, pipeline retention, and mutating startup recovery share the expanded
application service. Each artifact uses its store's hold and its own file mtime. Audio retains its
separate opt-in. Completed expiration markers prevent another automatic Trash attempt.

`RetentionFiles` checks only the pinned path. Notes and briefs use the existing bounded readers and
ownership parsers, including identity, stage/kind, schema, and required markers. Operator prose may
change without losing ownership; the move journal saves the exact current digest. Other artifacts
require their saved digest. Missing files are distinguished from modified content, and malformed,
oversized, untracked, pending-publication, and already-expired files are preserved. Redirected paths
are refused. Retention scans refuse more than 65,536 entries per store without unbounded allocation.

The sweep commits and exports the intent before calling Trash, then rechecks bytes. Recovery validates
the intent against current ownership metadata and the configured store root. Missing targets complete
expiration, unchanged targets can move again, and replacements keep the intent pending with the named
modified-target refusal. Brief moves and recovered completions carry occasion IDs through reports,
partial failures, pipeline output, and startup output. Human completion labels now cover both identity
kinds. Saved render data and per-artifact timestamps retain the protections from the ledger increment.

TDD and verification:

- `/private/tmp/vpt-retention-validation-red.log`: five behavioral failures before filesystem
  validation; the missing-pinned-file observation already passed.
- `/private/tmp/vpt-retention-validation-green.log`: all 46 filesystem-store tests passed after
  sharing the bounded byte reader and ownership parsers.
- `/private/tmp/vpt-later-retention-red.log`: six failures showed the audio-only sweep and recovery;
  the existing modified-target refusal already preserved the replacement.
- `/private/tmp/vpt-later-retention-ledger.log`: all 120 retention-focused tests passed. Coverage
  includes real files and both ledgers for all six later artifact types, observational dry runs,
  independent holds, publication refusal, changed bytes after intent commit, and uncertain brief moves.
- `/private/tmp/vpt-retention-scan-bound-red.log`: a 65,537-entry store was incorrectly accepted.
- `/private/tmp/vpt-later-retention-stores-final.log`: all 47 store tests passed after bounding scans.
- `/private/tmp/vpt-later-retention-audio.log`: all 31 existing retention execution tests passed.
- `/private/tmp/vpt-later-retention-cli-rerun.log`: all 19 retention CLI tests passed with their
  original 500 ms deadlines.
- `/private/tmp/vpt-later-retention-brief-final.log`: three CLI tests passed for brief dry-run/move
  reporting, occasion-owned startup recovery, deliberate regeneration, pipeline reporting, and keeping
  an expired brief absent on a subsequent run.
- `/private/tmp/vpt-later-retention-pipeline.log`: all four existing pipeline-retention tests passed.
- `/private/tmp/vpt-later-retention-unit-final.log`: all 81 command-library and 67 application unit
  tests passed.
- `/private/tmp/vpt-later-retention-clippy.log`: workspace Clippy passed with all targets,
  `dev-tools`, and warnings denied. The 20 changed/new production modules checked for this increment
  are each within 150 lines. Formatting and `git diff --check` passed.

The first CLI suite run passed 17 tests and exceeded the unchanged 500 ms deadline in disabled
retention and absent-target recovery. Both isolated Cargo reruns also timed out. Direct invocation of
the disabled command took 4-30 ms, its test passed outside Cargo, and the final Cargo suite passed all
19 cases. These observations do not establish a cause. Earlier review and synthesis-alert timing
failures remain open; there is still no passing full-workspace test claim.

Next acceptance work: verify terminal expiration across automatic stages and explicit note/brief
regeneration, then complete the remaining pipeline and shared requirements in the roadmap.


## Terminal expiration and per-store retention events

The full pipeline and terminal-expiration acceptance rows are complete. The roadmap now has 58 checked
and 49 unchecked tasks. New end-to-end tests produce actual artifacts, move them through retention,
and verify behavior through the CLI without manually setting expiration stages:

- Automatic runs leave expired audio, engine documents, transcript/analysis notes, releases, and
  private reports absent. Expired releases stay absent even while their source notes remain live.
- Explicit note writing restores saved transcript and analysis content after the audio and raw engine
  document have expired. It clears only the regenerated notes' expiration markers; a later automatic
  run neither invokes engines/agents nor recreates expired copies.
- Startup finishes absent pending note moves before selecting pipeline stages. Replacement bytes
  retain the pending intent and refuse automatic or explicit note writing with exit 3.
- Changing rendered synthesis inputs or disabling/re-enabling synthesis preserves expired analysis.
  Upcoming discovery preserves an expired brief even when its calendar event is rescheduled; explicit
  brief writing still regenerates it.

The new event test exposed a separate bug: eight mixed-store moves were reported as eight audio moves.
Retention now saves typed counts for all seven stores in each new event. The SQLite history codec,
JSON log, and report hook preserve those counts. Legacy history without per-store counts keeps its
original audio-only interpretation. Malformed, missing, negative, oversized, or private string values
inside a stored count map are refused without exposing the value. The broader logged-event roadmap
row remains open, including combined startup-recovery and new-work notification acceptance.

Evidence:

- `/private/tmp/vpt-terminal-expiration-tests.log`: five terminal-expiration tests passed.
- `/private/tmp/vpt-terminal-brief-tests.log`: four retention/brief tests passed, including automatic
  upcoming discovery after actual expiration.
- `/private/tmp/vpt-terminal-full-pipeline.log`: all 49 pipeline tests passed before the event-count
  fix, including the previously timing-out synthesis-alert aggregation case. No deadline was changed;
  this passing run does not establish the cause of the earlier timing failures.
- `/private/tmp/vpt-retention-store-events-red.log`: the end-to-end log/hook test failed because
  `counts.audio` was 8 instead of 1.
- `/private/tmp/vpt-retention-store-events-green.log`: the same test passed with exact per-store counts,
  matching log/hook documents, no private content, and no duplicate hook on an unchanged run.
- `/private/tmp/vpt-retention-store-events-ledger.log`: all 123 retention-focused adapter tests passed,
  including persisted-count round trips, legacy history, corrupt counts, later-store execution,
  expiration markers, and bounded filesystem validation.
- `/private/tmp/vpt-retention-store-events-unit.log`: all 67 application and 92 protocol tests passed.
- `/private/tmp/vpt-retention-store-events-execution.log`: all 31 retention execution tests passed.

The seven changed/new production modules are each within 150 lines. No unsafe code was added.
The earlier review-command timing failure remains unverified; no full-workspace pass is claimed.

Final command validation after the count fix: `/private/tmp/vpt-terminal-command-suite.log` passed all
30 brief, 50 pipeline, and 19 retention command tests. This includes the original 500 ms retention
command deadlines and 8-second pipeline deadlines. The new brief assertion reports one brief move
and zero audio moves in exported history.

Final workspace Clippy passed with all targets, `dev-tools`, and warnings denied in
`/private/tmp/vpt-terminal-retention-clippy.log`. Formatting and `git diff --check` passed.


## One retention event for pipeline recovery and new moves

The later-stage logging/hooks roadmap row is complete. The roadmap now has 59 checked and 48 unchecked
tasks. The regression test first observed two report-hook invocations when one pipeline command both
recovered an old engine-output move and expired seven other files.

The pipeline now owns one history run across startup repair and new work. Pending retention still
reconciles before ingestion, engines, or artifact writes. Each move retains its committed intent,
effect result, and expiration marker, and uses the existing history exporter. The pipeline collects actual moves and saves
one combined retention event with its final history facts. Standalone retention and other commands'
startup recovery use the same event builder. An existing-run application entry point lets the pipeline
own the final event without starting a second retention reporting cycle.

Partial recovery refusals report only completed moves, preserve replacement files and their pending
intents, and do not run engines. A later source failure still reports successful recovery with its
actual counts. Required-history failures stop processing: a refused final summary commit preserves
already committed moves, reports completed identities, sends no hook, and a later unchanged run neither
moves the files again nor fabricates a replacement delivery. Publication-repair completion identities
and hook warnings remain attached to partial command failures.

Verification:

- `/private/tmp/vpt-combined-retention-event-red.log`: the behavioral regression observed two hooks
  instead of one for eight mixed-store moves.
- `/private/tmp/vpt-combined-retention-event-green.log`: that case passed with one combined event.
- `/private/tmp/vpt-combined-retention-report-tests.log`: all ten focused reporting tests passed,
  including partial recovery refusal and a source failure after a successful recovery.
- `/private/tmp/vpt-combined-retention-command-suite.log`: all 30 brief, 54 pipeline, 19 retention,
  and 20 transcription command tests passed. Coverage includes the final-summary commit fault,
  default logged failures, private-safe uncertainty alerts, independent hook channels, published-brief
  reports, hook failures, and publication/recovery ordering. No command deadline was widened.

All nine changed/new production modules are within 150 lines. No unsafe code was added. The older
review-command timing failure remains unverified; this is not a full-workspace test pass claim.

Final library verification passed: 31 retention integration tests in
`/private/tmp/vpt-combined-retention-execution.log` and 67 application tests in
`/private/tmp/vpt-combined-retention-application.log`. Workspace Clippy passed with all targets,
`dev-tools`, and warnings denied in `/private/tmp/vpt-combined-retention-clippy.log`. Formatting
and `git diff --check` passed.
