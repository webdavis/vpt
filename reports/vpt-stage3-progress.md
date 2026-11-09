# Stage 3 progress

## Tag selection and typed settings

The domain tag vocabulary now uses the shared Unicode compatibility normalization and case folding.
Case and separator variants reuse the first existing vocabulary spelling. New multiword proposals
become kebab-case, and invalid syntax is rejected before classification. The selector preserves
proposal order, deduplicates before applying separate confirmed/suggested limits, and records the
position and reason of syntax or capacity rejections without including tag text in debug output.

The default policy holds unknown tags, with five confirmed tags and ten suggestions. The write policy
confirms unknown tags and returns the retained new values for later known-tag publication; capped or
duplicate tags are not learned. Inputs, normalized text, retained bytes, vocabulary entries, and proposal
counts are bounded. Tests cover Unicode variants, existing spellings, independent caps, zero caps,
invalid syntax, duplicate proposals, and private debug output.

All four tag settings now reach typed runtime settings: `new_tags`, `known_tags_path`, `max_per_note`,
and `max_suggested`. The path expands relative to the supplied home, and direct typed conversion also
rejects an invalid policy. The storage and file adapters described below now support tag state and
transcript-note rendering. Confirmation/rejection commands now support transcript notes; synthesis
integration and analysis-note updates remain unfinished. The normalized vocabulary item is complete.

## Read-only note index

The index adapter recursively reads Markdown files under supplied roots, including untracked external
notes. It records filename stems, aliases, and tags and provides exact normalized name/alias lookup.
Matching uses shared Unicode normalization and case folding, with whitespace normalization. Repeated
aliases of one note do not create ambiguity; matches across distinct notes retain all candidate paths.
Root and note ordering is deterministic, and overlapping roots do not duplicate notes.

The shared YAML parser reads external metadata without requiring VPT ownership, managed markers, or a
supported VPT schema. It accepts sequence/scalar metadata, anchors, Unicode, and CRLF. Missing, empty,
or null properties contribute no values; quoted `null` remains literal. Malformed metadata retains the
filename entry and produces a path/class diagnostic without including the document's contents.

The filesystem walk skips symlink entries and refuses a redirected root. It reuses the contained note
reader, which now also accepts a caller-supplied byte ceiling before allocating the file buffer. The
reader checks file identity and metadata across the read, and the walk revalidates directories and their
entry lists. It creates no files, changes no permissions, and gives no write authority over indexed notes.

Limits cover 256 supplied roots, 65,536 visited entries, 64 directory levels, 256 MiB of read bytes, and
16 MiB of retained metadata/path/key costs. Empty strings also consume their retained container cost.
Behavioral tests first exposed null properties being interpreted as names, uncharged root paths, and
empty aliases bypassing the retained-data budget; all three now pass after targeted fixes.

Profile root selection and the manual tag vocabulary consumer are now connected. Derived mentions
and synthesis consumers remain unfinished. The full index roadmap item stays unchecked until that
integration is verified.

## Verification of the index and selection foundation

- All 1,378 workspace unit tests passed serially, including 1,030 adapter and 163 domain tests:
  `/private/tmp/vpt-tags-index-units-final.log`.
- All 56 selected command tests passed: confirmation 13, note/path 17, pipeline 11, and review 15:
  `/private/tmp/vpt-tags-index-commands-final.log`.
- Workspace Clippy across all targets with `dev-tools` and warnings denied passed:
  `/private/tmp/vpt-tags-index-clippy-final.log`. Formatting and `git diff --check` passed.
- New production modules are below 150 lines. No unsafe code or test-deadline changes were introduced.

The review suite passed with its existing 500 ms child deadline on this run. The prior intermittent
export-time deadline failures have not been explained or fixed; this pass establishes current behavior,
not a latency guarantee. Full integration, native Apple Speech, and real Whisply were not rerun.

## Durable tags and transcript-note updates

The memory and SQLite ledgers now store ordered confirmed, suggested, and rejected tags with manual
or proposal provenance. Proposal metadata and tag names stay out of debug output and history. Writes
require the expected prior snapshot and a matching completed-work fact. They can share a transaction
with note changes and pending vocabulary bytes; late history failures roll back all of those changes.

Schema version 6 extends the existing tag table with order and optional update time, and records
whether a recording has an explicit tag snapshot. This distinguishes an empty selection from legacy
state with no saved selection. Migration preserves existing tag rows in deterministic order without
inventing timestamps, along with notes, history, and pending publications. Read-only access to older
schemas does not migrate them. New tag reads use a single database snapshot.

Saved tag reads reject invalid states/provenance/digests, oversized data, normalized duplicates,
invalid ordinals, and inconsistent initialization state with a fixed error that excludes private text.
Changes make successful transcript notes pending while preserving terminal expiration. Rendering
uses confirmed tags and held suggestions, omits rejected tags, and retains operator-added tags. A
tag-only edit leaves the exact transcript content digest unchanged.

The note rendering test first failed because no saved tags appeared in the note. It now covers
confirmed/suggested/rejected rendering, later removal, operator-tag preservation, and content-digest
stability. Shared ledger tests first exposed missing stage invalidation; both implementations now pass.

## Known-tag file adapter and path protection

The known-tag reader validates the bounded vocabulary. The update adapter reuses the known-term
snapshot and private publication mechanism, while applying tag normalization and existing spelling
selection. Case/separator variants leave the original bytes and modification time intact; new
multiword tags become kebab-case. Invalid proposals and malformed saved vocabulary are refused.
Preparing an update is observational, and changes made before publication are preserved.

Configuration checks now refuse known-tag destinations inside state, Voice Memos, or the released
store, overlapping confirmation-list paths, and aliases of the configuration or term list. The
alias checks cover existing hard links and symlinks before mutation. This does not close the existing
concurrent-editor race between the final file check and replacement in shared publication code.

Known-tag normalization and path-protection tests were observed failing before their implementations.
The adapter now has startup publication replay, described below. Manual tag confirmation/rejection
and profile-selected index roots are connected as described in the final section.

### Verification of durable tags and known-tag adapters

- All 1,413 workspace unit tests passed serially, including 1,065 adapter tests and all schema and
  migration tests: `/private/tmp/vpt-tag-state-units-final.log`.
- All 58 selected command tests passed: confirmation 14, note/path 18, pipeline 11, review 15:
  `/private/tmp/vpt-tag-state-commands-final.log`.
- Workspace Clippy across all targets with `dev-tools` and warnings denied passed:
  `/private/tmp/vpt-tags-clippy-final.log`. Formatting and `git diff --check` passed.
- Changed production modules for this work remain below 150 lines. No new unsafe code or changes to
  test deadlines were introduced. The prior review timing concern remains unexplained; this run
  passed with its unchanged deadline. Native Apple Speech and real Whisply were not exercised.

## Durable known-tag publication and replay

Pending vocabulary updates now carry an explicit `Terms` or `Tags` kind through the application
ports, both ledger implementations, and publication recovery. They use the same bounded bytes,
digest checks, target reservation, history transaction, and acknowledgement mechanism. Tag updates
add tag-vocabulary validation; malformed tag syntax cannot be saved as a pending publication.
Saving a tag snapshot, rendered note, and known-tag bytes together is covered by the shared atomicity
tests, including rollback after a late history failure.

Schema version 7 adds the kind to the existing pending-publication table. Existing rows become term
updates without changing their bytes or journal entries. Read-only version 5/6 access recognizes
legacy term updates without migration. Tests cover version 6 migration and rollback while preserving
tag state and pending term history, read-only reopening of pending tag updates, and refusal of
invalid or oversized saved kinds.

Startup and normal publication repair select the configured confirmation-list path from the saved
kind. A pending tag update cannot be redirected by changing `tags.known_tags_path`; it remains
pending with a named refusal. Recovery recognizes already-published bytes without rewriting the
file, preserves operator edits, and acknowledges the pending bytes only after durable publication.
Errors use safe tag/term-specific classes, and logs omit vocabulary contents.

The first behavioral tests exposed the lost tag kind and acceptance of invalid saved tag syntax.
The first command tests exposed recovery treating tag updates as term updates. Those tests now pass.
The manual `--tag`/`--reject-tag` commands now use this publication path. Synthesis producers and
analysis-note updates remain unfinished.

### Verification of vocabulary publication and recovery

- All 1,421 workspace unit tests passed serially, including 1,073 adapter tests:
  `/private/tmp/vpt-tag-publication-units-final.log`.
- All 99 selected command tests passed: confirmation 18, note/path 18, pipeline 11, retention 18,
  review 15, and transcription 19: `/private/tmp/vpt-tag-publication-commands-final.log`.
- Workspace Clippy across all targets with `dev-tools` and warnings denied passed:
  `/private/tmp/vpt-tag-publication-clippy.log`. Formatting and `git diff --check` passed.
- Production modules changed for this extension remain below 150 lines. No unsafe code or deadline
  changes were introduced. Native Apple Speech and real Whisply remain unverified in these runs.

## Manual tag confirmation and rejection

`confirm <id> --tag <tag>` and `--reject-tag <tag>` now parse tag text privately, validate the
recording and input before mutation, and use the existing locked startup recovery. Manual confirmation
works without synthesis, audio, or an engine, including before transcription. Both commands return
the specified `confirmed` JSON object with safe state indicators and the recording result.

The domain edit preserves an existing tag's spelling and position, promotes held tags, and records
manual provenance. Repeated actions retain the tag timestamp. Confirmation honors the configured cap
without evicting other tags; rejection remains available when the cap is reached. Rejected tags remain
in saved state while disappearing from managed tags and suggestions. Other suggestions, operator tags,
and the global vocabulary are retained.

The per-run index now selects the configured Obsidian vault or all three portable note stores. The
manual consumer combines indexed tag values with `known-tags.txt`, using confirmation-list spelling
first and retaining a recording's existing spelling. Invalid external tag values contribute a safe
diagnostic count and are excluded from the vocabulary. External notes receive no write authority.
Confirmed spellings are passed to a dedicated vocabulary update entrypoint so a vault spelling such
as `Project_Finance` is preserved in both the note and the confirmation list.

Tag state, transcript-note bytes, known-tag bytes, and history commit together. Missing accepted text
does not block saving tags. Expired transcript notes stay expired; tag actions do not regenerate them.
Malformed owned notes refuse the transaction. Late history failures roll back every change, and a
failed publication acknowledgement reports the committed recording so the next invocation can repair
publication. Log-export failures preserve success and produce a safe warning.

The initial command tests failed because only `--term` was recognized. Later behavioral tests caught
lowercasing of a selected external spelling and the missing `confirmed` result envelope; both are fixed.
Tests cover both index profiles, held-tag promotion/rejection, caps, privacy, invalid/unknown input,
confirmation before transcription, terminal expiration, rollback, acknowledgement recovery, and export
failure. Existing analysis notes still cause a refusal until the Stage 4 renderer can update them.

### Verification of manual tag commands

- All 1,427 workspace unit tests passed serially: CLI 64, adapters 1,074, application 61, domain 167,
  protocol 61: `/private/tmp/vpt-tag-confirm-units-final.log`.
- All 110 selected command tests passed: confirmation 18, note/path/tag commands 29, pipeline 11,
  retention 18, review 15, transcription 19: `/private/tmp/vpt-tag-confirm-commands-final.log`.
- Workspace Clippy across all targets with `dev-tools` and warnings denied passed:
  `/private/tmp/vpt-tag-confirm-clippy-final.log`. Formatting and `git diff --check` passed.
- Changed production files remain below 150 lines. No new unsafe code or deadline changes were
  introduced. Native Apple Speech and real Whisply were not exercised. Relations, synthesis, and
  analysis-note updates remain unfinished.


## Relation derivation and durable state

The domain now has the closed four relation kinds, typed relation targets and state, and private
manual/derived/proposal provenance. Relation settings reach runtime with the specified defaults of
60 session-gap minutes and five suggestions; both accept configured zero values.

Continuation chooses the immediate predecessor by UTC capture instant and identity before checking
its end. Overlap never falls back to an older recording. The gap must be positive and no larger than
the configured window. Tests cover input order, identity ties, time-zone changes, empty input, zero
windows, exact boundaries, and extreme duration/instant arithmetic.

The new bounded whole-term matcher is shared by known-term uncertainty suppression and mention
derivation. It preserves punctuation, uses NFKC and full case folding, supports overlapping phrases,
and caps input size, trie nodes, traversal work, and retained source ranges. The note-index consumer
resolves only confirmed terms with unique exact names/aliases, combines aliases for the same note,
retains every distinct matched range, and reports ambiguities without creating links. Indexed
external notes remain unchanged. Existing uncertainty behavior is covered by the domain regressions.

Both SQLite and the in-memory ledger now save relation state through `WorkDelta`, with expected-state
checks and matching successful work history. The existing relations table stores kind, target, state,
provenance, derivation rule, and word ranges; this extension needs no schema migration. Reads return a
deterministic kind/target order. Changes invalidate unexpired transcript-note state, while unchanged
relations and expired notes preserve their stages. Relation state and note publication can commit or
roll back in the same transaction; completion receipts include the recording identity.

Storage validates bounded counts, text, encoded bytes, and ranges. It refuses duplicate relations,
self-relations, noncanonical note paths, malformed or unknown saved values, and nonmanual
`supersedes`. Debug output and failure diagnostics omit private targets and proposal text. SQLite
reads use a snapshot and check aggregate byte/count limits before loading rows. Saved state survives
read-only reopening without changing database files, and late SQL/history failures roll back both
relation changes and note invalidation.

Behavioral red runs demonstrated the missing derivation/matching behavior and the initially absent
relation persistence, stale-state refusal, and note invalidation. A later boundary test exposed
acceptance of noncanonical note paths; that is fixed. Logs include
`/private/tmp/vpt-continues-red.log`, `/private/tmp/vpt-term-matcher-red.log`,
`/private/tmp/vpt-mentions-red.log`, `/private/tmp/vpt-relation-ledger-red.log`, and
`/private/tmp/vpt-relation-boundaries-red.log`.

These are foundations for the relation workflow. Command integration, persistent ambiguity
diagnostics, proposal caps, explicit relation confirmation, target-path resolution, and managed-link
rendering remain unfinished. The roadmap's relation capabilities remain unchecked.


### Verification of relation foundations

- All 1,476 workspace unit tests passed serially: CLI 64, adapters 1,106, application 61,
  domain 184, protocol 61: `/private/tmp/vpt-relations-units-final.log`.
- All 110 selected command tests passed with `dev-tools`: confirmation 18, note/path/tag commands
  29, pipeline 11, retention 18, review 15, transcription 19:
  `/private/tmp/vpt-relations-commands-final.log`.
- Workspace Clippy across all targets with `dev-tools` and warnings denied passed:
  `/private/tmp/vpt-relations-clippy-final.log`. The only fixes after the full test run make
  single-range test collections explicit for Clippy; focused relation/index tests cover those edits.
- Formatting and `git diff --check` passed. Changed production files stay below 150 lines.
  No unsafe code or deadline changes were introduced. Native Apple Speech and real Whisply were
  not exercised.


## Managed relation links and automatic continuations

Note-input snapshots now include saved relations, their recording targets' pinned transcript paths,
and the immediate predecessor with its pin. SQLite selects a single predecessor by capture instant
then identity instead of loading the recording catalogue for every note. Both ledger backends return
these inputs consistently with the recording, accepted words, flags, tags, and owned notes.

Transcript notes render saved non-rejected relations in deterministic kind/target order. Markdown
links use escaped labels and encoded relative destinations; wiki links follow the independent link
style setting. Related suggestions include escaped agent/reason provenance and the configured cap;
zero hides suggestions. Recording targets without a published note remain visible with a note-unavailable
label. Indexed external notes receive no edits. Link-only updates preserve the exact content-region
digest, and operator prose outside managed regions survives rewriting.

Shared note preparation derives continuation state from the immediate predecessor before checking
its end and gap. This path serves explicit note writes, transcription publication, pipeline work,
review resolution, and tag confirmation. It replaces obsolete derived continuations, preserves manual
relations, resolves current target pins, and commits changed relations with note bytes and history.
The pipeline compares this prepared state for already-completed notes, so a late-arriving recording
updates the successor's link without rerunning its engine. Dry-run planning performs the same comparison
without writes. Expired automatic stages remain terminal under the existing pipeline gates.

The first command tests failed on empty managed link regions. Later tests exposed stale continuation
state for a late-arriving predecessor and in tag-confirmation note updates. Those cases now pass.
Tests also cover target rename recovery, overlapping predecessors, a disabled session window, escaped
private labels and provenance, suggestion caps, manual relations, unavailable target notes, content-digest
stability, external/operator byte preservation, and rollback after malformed note input or a late
history failure. The shared ledger contract checks capture-order and identity tie selection even when
the selected predecessor overlaps the current recording.

Automatic mention derivation, durable ambiguity diagnostics, explicit relation confirmation, synthesis
proposal selection, and reciprocal analysis links remain unfinished. The continuation checklist item
is complete; the other relation checklist items retain their remaining scope.


### Verification of managed links and continuation integration

- All 1,478 workspace unit tests passed serially: CLI 64, adapters 1,108, application 61,
  domain 184, protocol 61: `/private/tmp/vpt-relation-links-units-final.log`.
- All 118 selected command tests passed with `dev-tools`: confirmation 18, note/path/tag/relation
  commands 35, pipeline 13, retention 18, review 15, transcription 19:
  `/private/tmp/vpt-relation-links-commands-final.log`.
- Workspace Clippy across all targets with `dev-tools` and warnings denied passed:
  `/private/tmp/vpt-relation-links-clippy-final.log`. Formatting and `git diff --check` passed.
- Behavioral red logs: `/private/tmp/vpt-relation-render-red.log`,
  `/private/tmp/vpt-continues-command-red.log`, `/private/tmp/vpt-continues-refresh-red.log`, and
  `/private/tmp/vpt-continues-tag-red.log`.
- Changed production files remain below 150 lines. No unsafe code, database schema migration,
  or deadline changes were introduced. Native Apple Speech and real Whisply remain unverified.


## Durable mention ambiguities

Schema version 8 adds private `mention_ambiguities` rows with confirmed-term text and all matched
word ranges. An optional ambiguity change is part of the existing relation transaction, with an
expected-state check. Both ledger backends preserve ambiguity state when a relation-only update
omits that change, replace or clear it deliberately, and roll it back with relations and history on
failure. Ambiguity-only changes invalidate unexpired notes while preserving expiration. Note-input
snapshots now include the saved ambiguities.

The ambiguity codec bounds terms, Unicode-normalized keys, row counts, encoded bytes, and total
ranges. It refuses empty terms/spans, normalized duplicates, invalid control characters, invalid or
unsorted ranges, and oversized inputs. Relation and ambiguity storage share the range codec and
validation. Debug output omits confirmed-term text. SQLite checks stored aggregate sizes before
loading rows and refuses malformed, duplicate, oversized, or wrongly typed saved values with fixed
private diagnostics. Read-only reopening preserves both saved ranges and database bytes.

Version 7 remains observable without a migration and returns no saved ambiguities. The version 8
migration preserves existing relation state and history, rolls back on a precommit failure, and adds
`recordings_capture_order` for the predecessor query used by note preparation. Existing migration
coverage from versions 1 through 6 still passes with the new extension. No prior relation rows or
publication formats are rewritten.

Behavioral tests first exposed the ignored ambiguity changes and missing migration. Red evidence is
in `/private/tmp/vpt-ambiguity-ledger-red.log` and `/private/tmp/vpt-ambiguity-migration-red.log`.
The focused relation and migration checks pass. Automatic mention derivation still needs to connect
the per-run note index and known terms to note preparation; no mention capability is marked complete
on the strength of storage tests alone.


### Verification of ambiguity storage

- All 1,494 workspace unit tests passed serially: CLI 64, adapters 1,124, application 61,
  domain 184, protocol 61: `/private/tmp/vpt-ambiguity-units-final.log`.
- All 118 selected command tests passed with `dev-tools`: confirmation 18, note/path/tag/relation
  commands 35, pipeline 13, retention 18, review 15, transcription 19:
  `/private/tmp/vpt-ambiguity-commands-final.log`.
- Workspace Clippy across all targets with `dev-tools` and warnings denied passed:
  `/private/tmp/vpt-ambiguity-clippy-final.log`. Formatting and `git diff --check` passed.
- Focused checks include 49 relation tests and 11 migration tests:
  `/private/tmp/vpt-ambiguity-durability-green.log` and
  `/private/tmp/vpt-ambiguity-migrations-green.log`.
- Changed production files remain below 150 lines. No unsafe code or deadline changes were
  introduced. Native Apple Speech and real Whisply were not exercised.


## Automatic mention integration

Note writes, transcription publication, pipeline refreshes, review resolution, and tag updates now
derive mentions from the immutable accepted word stream. A shared context reads the profile's note
index and confirmed terms once per pipeline run. Unique name/alias matches become confirmed derived
links with all source ranges; ambiguous matches save private term/range rows without linking.
Only earlier derived mentions are replaced. Manual relations, operator text, external notes, and the
transcript content digest are preserved when only links change.

Review resolution includes a newly confirmed term in the same prepared note transaction as its
resolution and vocabulary update. Rendered correction text does not create invented accepted-word
mentions. Equivalent confirmations use Unicode compatibility normalization, full case folding, and
collapsed whitespace to keep the original term spelling. A failing command test exposed erroneous
capacity refusal when confirming an existing term in a full 65,536-entry list. The fix avoids adding
the duplicate; its full-capacity regression now runs directly against the composition and matcher
to avoid exceeding the command harness's unchanged 500 ms deadline.

Command tests cover both note profiles, repeated occurrences, ambiguity resolution after alias
changes, clearing removed terms, preservation of manual links, failed note preparation and late
history rollback, newly confirmed terms, and corrections. Pipeline tests verify an index change
refreshes a successful note without an available engine, dry runs leave state unchanged, and an
unchanged subsequent run skips publication. Publication-result formatting was extracted into its
own module to keep the changed production files below 150 lines.

The mention checklist item is complete. Synthesis consumers of the index, explicit relation
confirmation, proposal selection, and reciprocal analysis links remain unfinished.

### Verification of automatic mentions

- All 1,497 workspace unit tests passed serially: CLI 67, adapters 1,124, application 61,
  domain 184, protocol 61: `/private/tmp/vpt-mentions-units-final.log`.
- All 124 selected command tests passed with `dev-tools`: confirmation 18, note/path/tag/relation
  commands 38, pipeline 14, retention 18, review 17, transcription 19:
  `/private/tmp/vpt-mentions-commands-final.log`.
- Workspace Clippy across all targets with `dev-tools` and warnings denied passed:
  `/private/tmp/vpt-mentions-clippy-final.log`. Formatting and `git diff --check` passed.
- Behavioral red evidence: `/private/tmp/vpt-mentions-command-red.log` and
  `/private/tmp/vpt-mentions-capacity-red.log`. Focused matching/composition and command checks:
  `/private/tmp/vpt-mentions-capacity-green.log` and `/private/tmp/vpt-mentions-focused.log`.
- No unsafe code, schema migration, or deadline changes were introduced. Native Apple Speech
  and real Whisply remain unverified; the broader product checklist remains incomplete.


## Explicit relation confirmation

`vpt confirm <id> --relation <kind>:<target>` now supports all four relation kinds. It accepts a
recording identity or an indexed note's unique name, alias, or absolute path. `continues` and
`supersedes` resolve to recordings: a named note must have a saved ledger association and valid
current ownership markers. Both forms use the recording's current transcript pin for rendering.
An indexed external note grants no write authority. Unknown, ambiguous, redirected, and self targets
are refused; source identity checks also catch renamed and expired self notes.

Confirmation keeps existing proposal provenance and mention source ranges. New or previously derived
relations become manual confirmations and survive later derivation. Only the selected relation is
changed; ordinary note preparation still refreshes automatic links and ambiguities. Its prepared
relation update uses the original saved state as the transaction's expected snapshot. Relation state,
note bytes, and history commit together, and the existing publication recovery handles an interrupted
acknowledgement. Confirmation can precede transcription and never regenerates an expired note.

Tag and relation commands share the existing confirmation transaction/publication workflow, avoiding
another copy of its run bookkeeping and partial-success handling. Command dispatch was moved to its
own module to keep production files below 150 lines. Debug formatting and validation errors omit
raw relation text; logged facts omit target labels and proposal reasons. Output reports the kind
and whether its saved state changed.

Behavioral tests first failed on the missing CLI action, named owned-note target resolution, lost
mention ranges, and renamed self targets. The command tests cover these cases plus both note profiles,
all four kinds, idempotence, proposal provenance, external/operator content preservation, pre-transcript
and expired behavior, late history rollback, and publication acknowledgement recovery.

The combined related-relations checklist item remains open for synthesis proposal selection and
analysis-note updates. The analysis renderer and the broader Stage 4 scope remain unfinished.

### Verification of relation confirmation

- All 1,499 workspace unit tests passed: CLI 69, adapters 1,124, application 61, domain 184,
  protocol 61: `/private/tmp/vpt-confirm-relations-units-final.log`. The final CLI changes also
  passed all 69 CLI unit tests in `/private/tmp/vpt-confirm-relations-cli-final.log`.
- All 134 current selected command tests passed: confirmation 18, note commands 48, pipeline 14,
  retention 18, review 17, transcription 19. The broader run is recorded in
  `/private/tmp/vpt-confirm-relations-commands-final.log`; the final relation-target corrections and
  added rename test passed the full note suite in `/private/tmp/vpt-confirm-relations-note-final.log`.
- Workspace Clippy across all targets with `dev-tools` and warnings denied passed:
  `/private/tmp/vpt-confirm-relations-clippy-final.log`. Formatting and `git diff --check` passed.
- Red evidence: `/private/tmp/vpt-confirm-relations-red.log`,
  `/private/tmp/vpt-confirm-owned-target-red.log`, `/private/tmp/vpt-confirm-ranges-red.log`,
  `/private/tmp/vpt-confirm-renamed-self-red.log`, and `/private/tmp/vpt-confirm-expired-self-red.log`.
- No unsafe code, schema changes, or deadline changes were introduced. Native Apple Speech and
  real Whisply remain unverified. No commit, push, or merge was performed.

## Recording observations and engine diagnostics (acceptance pending)

`doctor` now reports every configured engine's executable availability and locality. Default and
language-override selections must be available; unused definitions are informational. Generic
commands are labeled declared local or declared remote. Doctor does not execute these commands or
print their arguments/family configuration. Apple uses the existing compatible-helper check. Existing
vault and Git-placement checks remain observational, including sensitive drafts/engine-output stores.

`show` and `list` now use an observation port that reads each result from one consistent ledger
snapshot. Memory and SQLite implementations expose accepted engine/language metadata, current open
flag and diagnostic identifiers/classes/counts, stale verification stages, note pins, and artifact
paths/digests/publication/expiration state. Transcript/proposal bodies are omitted. Inventory is
bounded to 65,536 recordings and 16 MiB of retained metadata; artifact inventory keeps its existing
bounds. Human list output includes stage states. Stage filters continue to select every state other
than succeeded, including disabled and expired. Storage continues to inventory all seven stores
independently of source/ledger availability.

### Evidence and remaining validation

- Behavioral red runs: `/private/tmp/vpt-doctor-engines-red.log` (missing engine check and missing
  refusal for unavailable selected engines); `/private/tmp/vpt-recording-observations-red.log`
  (missing accepted engine metadata).
- The full pipeline suite passed all 58 tests with unchanged deadlines and serial execution:
  `/private/tmp/vpt-observation-pipeline-suite.log`. Its three new observation tests cover actual
  expiration of eight artifacts, preserved note/engine metadata, storage after expiration, current
  lexical and analysis findings, stale-finding exclusion after confirmation, pending publications,
  and unchanged filesystem snapshots. The focused run is
  `/private/tmp/vpt-recording-observations-acceptance.log`.
- A prior complete show/list/storage run passed all 31 tests, including its native active writer:
  `/private/tmp/vpt-show-list-storage-suite.log`. Final runs after bounded observation queries were
  added passed 30 and timed out in the active-writer case:
  `/private/tmp/vpt-show-list-storage-final.log` and
  `/private/tmp/vpt-show-list-storage-final-repeat.log`. Final acceptance is therefore still open.
- Doctor runs passed 37 tests, including all five new engine tests, but the active-writer case
  repeatedly exceeded its existing 500 ms command deadline:
  `/private/tmp/vpt-observation-commands-final.log` and
  `/private/tmp/vpt-observation-commands-repeat.log`. Its earlier isolated pass is
  `/private/tmp/vpt-doctor-active-writer.log`.
- Excluding active-writer cases did not eliminate timing failures: the first compatible-helper test
  timed out, with the other 36 doctor cases passing. Cargo stopped before the second test target:
  `/private/tmp/vpt-observation-commands-without-active-writer.log`.
- Temporary tracing showed both delays before the doctor entry point and delay during the helper
  check. One traced command completed doctor checks in 118.724 ms, within 494.191 ms total command
  time: `/private/tmp/vpt-doctor-spawn-trace-warm.log`. That diagnostic run intentionally failed the
  clean-stderr assertion. These observations do not establish the cause of the delays. All temporary
  tracing and diagnostic-preview changes were removed; deadlines were not changed.

Both observation roadmap rows remain unchecked. The goal remains active. No schema migration,
unsafe block, commit, push, or external engine integration was introduced by this change.

Final checks after removing diagnostic instrumentation:

- Memory observation metadata/order/engine/flag behavior and capacity refusal: 2 tests passed,
  `/private/tmp/vpt-observation-memory-final.log`.
- Direct show/list/storage command behavior and safe invalid-path output: 4 tests passed,
  `/private/tmp/vpt-observation-command-units.log`.
- Workspace Clippy, all targets, `dev-tools`, warnings denied: passed,
  `/private/tmp/vpt-observation-clippy-final.log`.
- Formatting and `git diff --check` passed. Added/changed observation production modules are all
  below 150 lines. The roadmap remains 61 checked and 46 unchecked.

## Tag policy and related suggestion acceptance

Closed the tag-policy and related-suggestion checklist rows against specification sections 7.3,
7.4, and the configured defaults. This change adds acceptance tests; production behavior already
satisfies the tested requirements.

| Requirement | Evidence |
| --- | --- |
| Default `hold`, five confirmed tags, ten suggestions, and proposal-order deduplication | `synthesize/tag_defaults.rs` exercises interleaved known/new tags and normalized duplicates in both profiles. It checks selected tags, first-proposal reasons, rejection indices, both published notes, and private output. |
| Automatic `write` and preservation of operator tags | The same test checks exactly the retained new tags in `known-tags.txt`, empty suggestions, distinct operator tags and prose in each note, and unchanged external vocabulary notes. Existing metadata and recovery cases cover custom limits, manual rejection, and vocabulary publication recovery without rerunning the agent. |
| Related suggestions with provenance and default/configured caps | `synthesize/related.rs` covers limits five, two, and zero, both profiles/link styles, and both tag modes. Missing, ambiguous, self, duplicate, and already-confirmed targets do not consume suggestion slots. Both notes retain manual links and render escaped proposal provenance. |
| Four closed relation kinds; agent proposals cannot introduce `supersedes` | New command tests reject agent-proposed `supersedes`, `continues`, `mentions`, and unknown kinds while preserving accepted proposals, tags, relations, and note content. Existing note tests exercise explicit confirmation of all four kinds, preserved proposal provenance, and atomic updates to both notes. |
| Durable and bounded tag/relation state | Both ledger contract suites cover history coupling, note/vocabulary transactions, rollback, stale expected state, bounded inputs, private diagnostics, and expired-note behavior. SQLite cases additionally cover reopening and corrupt saved state. |

Validation ran sequentially with `TMPDIR=/private/tmp`; CLI and SQLite tests ran with one test
thread. All existing command and subprocess deadlines were retained.

- Synthesis: 27 distinct cases passed. The full run passed 25 cases in
  `/private/tmp/vpt-tag-relation-synthesis-acceptance.log`; the two relation cases passed after
  correcting test expectations about canonical ordering and refreshed preparation timestamps in
  `/private/tmp/vpt-related-acceptance.log`.
- Note and confirmation commands: 80 passed,
  `/private/tmp/vpt-tag-relation-commands.log`.
- Domain tags and relations: 15 and 12 passed,
  `/private/tmp/vpt-tag-relation-domain-tags.log` and
  `/private/tmp/vpt-tag-relation-domain-relations.log`.
- Proposal protocol: 9 passed, `/private/tmp/vpt-tag-relation-protocol.log`.
- Adapter tags and relations: 50 and 49 passed,
  `/private/tmp/vpt-tag-relation-adapter-tags.log` and
  `/private/tmp/vpt-tag-relation-adapter-relations.log`.

These are 242 distinct passing targeted tests. The checklist now has 64 checked and 43 unchecked
items. The earlier doctor/show/list timing failures and shared publication race work remain open.
Workspace Clippy passed for all targets with `dev-tools` and warnings denied:
`/private/tmp/vpt-tag-relation-clippy.log`. Formatting and `git diff --check` also passed.

## Owned notes, filenames, rename recovery, and index acceptance

Closed five remaining owned-note checklist rows against specification sections 7.1–7.4.
The new profile test first exposed a missing audio reference in the shared note body. Both transcript
and analysis notes now begin their managed link block with the archived audio reference. The same
bounded link renderer supplies the Obsidian frontmatter reference and both profiles' body links.
Wiki references retain `.m4a` and the path from the configured root; outside-root references use
encoded relative Markdown links. README note-writing guidance now describes this behavior.

| Requirement | Evidence |
| --- | --- |
| Portable and Obsidian profiles with independent link styles | `synthesize/note_profiles.rs` checks both notes under all four combinations: recording/stage identity, capture time/offset, duration, regional language, both engines, uncertainty/diagnostic counts, readability, suggestions, and the ordered Obsidian fields. Separate cases exercise audio inside the wiki root. Domain tests verify the local first-write date across a UTC date boundary. |
| Managed regions and preserved operator content | `synthesize/note_boundaries.rs` replaces both managed regions on disk and verifies restoration from saved content without an agent. Operator prose before, between, and after regions, tags, nested fields, comments, multiline descriptions, and date anchors survive. Damaging either note's markers, identity, stage, or schema refuses both writes and leaves the journal empty. Domain and formatter cases cover escaped external text, corrections, malformed/nested markers, CRLF, YAML types, and bounded rendering. |
| Flat deterministic filenames and immutable pins | `synthesize/filenames.rs` exercises nine title/limit cases, including compatibility normalization, case folding, decomposed Unicode, unsafe separators, empty titles, the default 60-character cap, and configured limits. Both note paths, first-write times, and content digests survive later title, slug-setting, and tag changes. |
| Unique rename recovery and safe refusal | Existing note rename, analysis, and reciprocal-link cases verify updated pins, preserved operator edits, one move event, missing-note regeneration, and refusal of ambiguous, foreign, malformed, or redirected claimants. Store tests cover escaped paths, redirected roots, and claimant mutation during discovery. Pipeline expiration cases prove automatic work keeps expired notes expired while explicit regeneration remains available. |
| Fresh, read-only note index | Existing note-index cases cover nested external Markdown, names/aliases/tags, duplicate aliases, overlapping roots, missing roots, malformed metadata, symlink files/directories/cycles, and capacity limits. Command cases exercise profile-specific vocabulary, changed aliases and terms, newly added vocabulary notes, and indexed relation targets without changing external bytes. |

Validation ran sequentially with `TMPDIR=/private/tmp`, serial CLI/SQLite tests, and unchanged
command and subprocess deadlines:

- Behavioral red: `/private/tmp/vpt-note-profile-audio-red.log` (missing managed audio link).
  Focused green: `/private/tmp/vpt-note-profile-audio-green.log`.
- Note and synthesis commands: 62 + 32 passed,
  `/private/tmp/vpt-note-profiles-commands-final.log`. Two prior relation tests now expect the audio
  link to remain when there are no relation links; their relation and content assertions are retained.
- Domain note rendering: 41 passed, `/private/tmp/vpt-note-profiles-domain.log`.
- Application link rendering: 3 passed, `/private/tmp/vpt-note-profiles-application.log`.
- Frontmatter/document formatting: 35 passed, `/private/tmp/vpt-note-profiles-format.log`.
- Full pipeline: 58 passed, `/private/tmp/vpt-note-profiles-pipeline.log`.
- Note stores and both ledger backends: 44 passed,
  `/private/tmp/vpt-note-profiles-stores-ledger.log`.
- Note index: 8 passed, `/private/tmp/vpt-note-profiles-index.log`.
- Workspace Clippy, all targets, `dev-tools`, warnings denied: passed,
  `/private/tmp/vpt-note-profiles-clippy.log`. Formatting passed in
  `/private/tmp/vpt-note-profiles-format-check.log`; `git diff --check` also passed.

These are 283 passing targeted tests. Changed production modules remain within 150 lines and add
no unsafe code or schema changes. The checklist now has 69 checked and 38 unchecked items.
The doctor/show/list timing failures and shared publication race work remain open.
