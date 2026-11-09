# VPT Stage 4 progress

## Proposal decoding and domain values

The protocol crate now decodes `vpt.proposal/1` into typed wire values. The adapter converts those
values into domain proposals containing author provenance, ordered summary and action claims,
declared source references, tag proposals and their reasons, and related-note proposals and their
reasons. Application code can consume these domain values without depending on the wire schema.
No synthesis command is available yet, and this work does not mark the complete proposal checklist
item finished.

The reader enforces the specification's 1,048,576-byte proposal limit, depth eight, and 65,536
characters per text value. Only top-level summary and action arrays use the 65,536-entry limit;
source references, tags, relations, unknown arrays, and similarly named nested fields retain the
256-entry limit. The existing engine and general incoming-document profiles retain their limits.
Checks happen through the bounded JSON reader before conversion to typed values.

Unknown major versions are refused with their version number. Unknown additive fields are accepted
and named in sorted diagnostics without including their values. Malformed shapes, missing required
fields, blank authors/claims/targets, invalid control characters, and non-`related` relation proposals
are refused with fixed diagnostics. Agents cannot use proposals to create `supersedes` links.

Declared citations are preserved even when absent, malformed, inverted, or outside a recording's
bounds. These cases belong to the grounding findings required by the specification. Likewise, tag
syntax is left for the tag rules to classify and reject. Proposal text containing Markdown, managed
markers, or annotation-like strings remains literal data for the later renderer. Neither protocol
nor domain debug output includes claim text, source-reference text, author names, targets, or reasons.

Behavioral red evidence is in `/private/tmp/vpt-proposal-red.log` and
`/private/tmp/vpt-proposal-adapter-red.log`. Focused protocol and conversion checks passed in
`/private/tmp/vpt-proposal-protocol-final.log` and `/private/tmp/vpt-proposal-adapter-green.log`.

### Verification

- All 1,510 workspace unit tests passed serially: CLI 69, adapters 1,126, application 61,
  domain 184, protocol 70: `/private/tmp/vpt-proposal-units-final.log`.
- Workspace Clippy across all targets with `dev-tools` and warnings denied passed:
  `/private/tmp/vpt-proposal-clippy-final.log`. Formatting and `git diff --check` passed.
- New protocol tests cover ordered round trips, private debug output, unknown versions and fields,
  malformed shapes, forbidden relation kinds, grounding inputs, and exact resource boundaries.
  Adapter tests verify complete conversion and refusal of invalid input.
- This change adds no command producer, schema migration, unsafe code, or deadline changes. The
  synthesis command, accepted proposal storage, and analysis rendering remain unfinished.

## Synthesis settings and command runner

Typed settings now retain the configured synthesis argv, deadline (default 600 seconds), and grounding
window (default 15 seconds). An empty command remains disabled. Invalid executable/argument syntax
and oversized argv are refused without echoing their values. The generated configuration comment
states that the command receives the full transcript.

The `AgentCommand` application port exchanges domain proposals. Its process adapter sends exactly
the supplied content bytes on stdin and returns their SHA-256 digest with the decoded proposal and
additive-field diagnostics. The transcript path is passed as metadata; the adapter does not reread
it. Argument replacement preserves boundaries, substitutes each token once, and leaves inserted values
and unknown tokens literal. The bounded replacement implementation is shared with command engines.

Synthesis accepts at most 256 arguments totaling 1 MiB after substitution, input within the existing
64 MiB rendered-note ceiling, and proposal stdout within 1 MiB. It uses the existing owned-process
runner, nonblocking stdin/stdout/stderr handling, command deadline, interruption handling, and cleanup
grace. Captured child output does not enter errors. Disabled commands, invalid input, missing programs,
exit/signal failures, deadlines, output overflow, bad documents, and unsupported versions have typed
failures. A child that closes stdin before all content has been sent cannot return an accepted proposal.

Behavioral tests failed first on ignored runtime settings, the absent runner, and accepted invalid
command arguments: `/private/tmp/vpt-synthesis-settings-red.log`,
`/private/tmp/vpt-synthesis-command-red.log`, and `/private/tmp/vpt-synthesis-boundaries-red.log`.
Focused checks pass in `/private/tmp/vpt-synthesis-boundaries-green.log`, including exact stdin bytes
and digest, literal path/identity/language arguments, private diagnostics, the output-size boundary,
disabled/invalid requests without process execution, blocked stdin, and early input closure.

The `synthesize` CLI, ledger acceptance, grounding, and analysis publication remain unfinished.

### Verification of the command runner

- All 1,521 workspace unit tests passed serially: CLI 69, adapters 1,137, application 61,
  domain 184, protocol 70: `/private/tmp/vpt-synthesis-runner-units-final.log`. This includes the
  existing process interruption, cleanup, and engine argument tests.
- All 33 selected command tests passed with `dev-tools`: pipeline 14 and transcription 19:
  `/private/tmp/vpt-synthesis-runner-commands-final.log`.
- Workspace Clippy across all targets with `dev-tools` and warnings denied passed:
  `/private/tmp/vpt-synthesis-runner-clippy-final.log`. Formatting and `git diff --check` passed.
- Changed production files remain below 150 lines. No unsafe code, database migration, command-test
  watchdog changes, commit, push, or merge was introduced. Real synthesis providers, native Apple
  Speech, and real Whisply were not exercised.

## Claim grounding and analysis content

The domain checker now resolves bounded minute/second citations against the recording duration and
classifies claims in the specified priority order. It compares whole capitalized tokens, digit runs,
punctuated numeric/currency expressions, and English month/date and currency phrases against cited
words plus the configured context window. Compatibility normalization and full case folding preserve
whole-token matching; meaningful signs, decimal/grouping separators, currency symbols, percentages,
fractions, and numeric date separators remain part of the evidence key. English spelled-out money
amounts retain their complete phrase rather than matching their component words independently.
This is a lexical check, not a semantic
paraphrase check.

Evidence windows that touch or overlap are combined. Disjoint windows are checked separately, so
they cannot invent an adjacent date or money phrase. The context distance includes its exact boundary.
Uncertainty uses the actual half-open cited intervals and the accepted word offsets, including anchors
for insertion flags. Only open surfaced lexical flags are inherited. Their identifiers remain attached
even when an unsupported token is the higher-priority finding, so later redaction can inspect them.

Confirmed corrections replace the corresponding evidence spans without mutating accepted words.
Repeated corrections and insertion corrections keep their own time anchors. Empty, contradictory,
overlapping, or otherwise invalid correction inputs are refused. Evidence projection borrows text and
has bounded aggregate size. The checker also bounds transcript words, flag ranges, citations, text,
normalization/token expansion, and cumulative work across summary and action claims.

Checking a proposal retains its author, tags, relations, claim order, raw declarations, resolved ranges,
findings, and inherited flag identifiers. The content renderer writes fixed Summary and Actions
sections with one bullet per claim, citations in declared order, and the selected annotation. It uses
the existing bounded escaping writer for all external text, including malformed citations and finding
payloads. Claim text cannot inject headings, links, managed markers, or additional annotations. Empty
sections remain present. Neither checked-proposal nor finding debug output includes private text.

The initial grounding and analysis-content tests failed against their stubs:
`/private/tmp/vpt-grounding-red.log` and `/private/tmp/vpt-analysis-content-red.log`. Additional
behavioral failures covered compound facts, token limits, budget exhaustion, adjacent windows, and
currency suffixes. A deliberately omitted correction projection failed all three correction tests:
`/private/tmp/vpt-grounding-corrections-mutation.log`; the projection was restored afterward.

This adds domain components. It does not yet publish an owned analysis document, persist accepted
proposals/findings, extract Markdown claims, or expose `synthesize`/`verify-note` commands. Resolution
preservation and complete note metadata/link updates remain part of those integrations.

### Verification of grounding and content rendering

- All 1,546 workspace unit tests passed serially: CLI 69, adapters 1,137, application 61,
  domain 209, protocol 70: `/private/tmp/vpt-grounding-units-final.log`.
- After the final currency-spacing, Unicode-number, and spelled-out-money refinements, all 209 domain
  tests passed again: `/private/tmp/vpt-grounding-domain-final.log`. The added behavioral assertions
  first failed in `/private/tmp/vpt-grounding-currency-separation-red.log`,
  `/private/tmp/vpt-grounding-unicode-numeric-red.log`, and
  `/private/tmp/vpt-grounding-spelled-money-red.log`.
- Workspace Clippy across all targets with `dev-tools` and warnings denied passed:
  `/private/tmp/vpt-grounding-clippy-final.log`. Formatting and `git diff --check` passed.
- New production modules remain below 150 lines. There is no new unsafe code, migration, command
  producer, subprocess change, deadline change, commit, push, or merge. The existing command suites
  were not rerun for these domain-only additions; real synthesis providers were not exercised.

## Durable accepted proposals

Both ledger backends now implement `ProposalLedger` and accept proposal changes through the existing
atomic history transaction. Accepted state preserves agent provenance, ordered raw claims and source
declarations, resolved source ranges, selected findings, inherited lexical flag identifiers, proposed
tags and related notes, the exact transcript-content input digest, the grounding window, and the
acceptance time. A separate grounding basis fingerprints accepted transcript inputs and lexical flag
state, including confidence, resolutions, and correction text. Lexical flags are fingerprinted in
identity order. Diagnostic changes and an unchanged retranscription's acceptance time do not alter
that basis. Inherited flag identifiers also use a stable identity order, so changing the ledger's read
order does not change a checked claim or cause a spurious acceptance conflict.

Acceptance requires a published current transcript, its matching content digest and grounding basis,
and an analysis note whose content matches the checked proposal. The ledger recomputes grounding
against current accepted words and flags before accepting the supplied results. It refuses pending
source publication or retention, concurrent transcript/resolution changes in the same batch, changed
source content, missing analysis publication records, and missing, duplicated, contradictory, or
mistimed work history. Note ownership, expected digests, pinned paths, and publication reservations
remain enforced by the existing note transaction checks.

Proposal state, analysis bytes, publication intent, stage state, and history commit together. Synthesis
stays pending until analysis publication is acknowledged. A failed attempt preserves the accepted
proposal and analysis, records failure, and returns no completed recording. A later valid acceptance
replaces prior accepted state and reports the recording once. A failure while appending later history
rolls back the proposal, note, stage, and journal changes together.

SQLite uses the existing `proposals` table; schema version remains 8. Its saved body has an explicit
version, a 16 MiB ceiling, bounded nesting/arrays, and the original 1 MiB proposal contract inside it.
Grounding metadata has aggregate item and text limits checked before constructing the saved document.
Reads validate declared citations against stored resolved ranges and refuse malformed versions,
digests, finding shapes, or mismatched claim counts with fixed private diagnostics. Oversized columns
are rejected before copying their values into Rust. Reopened and read-only ledger reads preserve
accepted state and the current grounding basis; read-only access does not change the database files.

Behavioral failures preceded the storage implementation, citation consistency check, and aggregate
finding-text bound: `/private/tmp/vpt-proposal-storage-red.log`,
`/private/tmp/vpt-proposal-codec-red.log`, and `/private/tmp/vpt-proposal-storage-limits-red.log`.
The command runner is still not connected to this transaction path. Artifact-specific verification
identities and resolutions are implemented below. Owned analysis document construction, tag/relation
proposal selection, and CLI/pipeline orchestration remain unfinished.

### Verification of accepted-proposal storage

- All 1,575 workspace unit tests passed serially: CLI 69, adapters 1,165, application 61,
  domain 210, protocol 70: `/private/tmp/vpt-proposal-storage-units-final.log`.
- The 134 existing confirmation, note, pipeline, retention, review, and transcription command cases
  were exercised with `dev-tools`. The initial run passed 133; the two-engine provenance case hit its
  unchanged five-second watchdog during a 4.827-second polling gap. Clippy had mistakenly been started
  before the command suites finished. The isolated serial rerun passed in 1.18 seconds:
  `/private/tmp/vpt-proposal-storage-commands-final.log` and
  `/private/tmp/vpt-proposal-storage-transcribe-recheck.log`. No deadline was increased.
- Workspace Clippy across all targets with `dev-tools` and warnings denied passed:
  `/private/tmp/vpt-proposal-storage-clippy-final.log`. Formatting and `git diff --check` passed.
- New production modules stay below 150 lines. No unsafe code, database migration, command producer,
  subprocess change, commit, push, or merge was introduced. Real synthesis providers, native Apple
  Speech, and real Whisply were not exercised.

## Remaining synthesis integration

- Connect synthesis command output to the accepted-proposal transaction and artifact-specific
  verification finding storage.
- Complete owned analysis-note metadata, resolution-aware rendering, and reciprocal
  transcript/analysis links. The `verify-note` command is implemented below.
- Connect the settings and command runner to owned-note inputs, `vpt synthesize`, and `run`.
- Apply tag and relation proposal rules, publication recovery, input-change invalidation, expiration,
  and command output/reporting behavior.
- Continue the remaining briefs/context, redaction, handoff, and full-store retention scope in
  `docs/vpt-remaining-work.md`.


## Artifact-specific verification findings

Added the third review-flag shape for `unsourced`, `bad-reference`, `unsupported`, and
`built-on-flagged-text`. A finding records its owned artifact stage, zero-based claim position,
claim digest, private claim text, and optional source time ranges. Unsourced and bad-reference
findings have null stored source ranges. Unsupported tokens remain private in debug output.

Finding identity includes the recording, artifact stage, claim position, and SHA-256 claim digest.
Artifact identity survives a path rename. Changed text or position gets a new identity; a changed
finding class on the same claim retains its existing resolution. Claim text supplied to this boundary
must already have verification annotations stripped; the Markdown adapter below now supplies that text.

Both ledger backends replace only the selected artifact's findings and preserve unchanged confirmed,
corrected, and dismissed resolutions. The transaction checks the owned content digest and current
transcript/lexical-flag basis, requires matching completion history, and refuses publication or retention
reservations. A batch may update verification annotations while preserving the exact unannotated
content; changing claim content under an older checked digest is refused. Claim digests, positions, sources, text sizes, aggregate storage, and counts are bounded
and validated. Later history failure rolls back replacement. Existing schema-8 flag columns suffice;
no migration was added.

Review snapshots and note inputs include verification findings, while transcript-only flag queries and
classification exclude them. Replacing transcript flags preserves owned-artifact findings. SQLite
recounts open verification and surfaced lexical flags together. Review JSON and human output show
claim identities and source ranges; transcript frontmatter includes the total open count. Corrections
to analysis claims never replace transcript evidence or enter the known-term selection path.

The Markdown increment below adds annotation removal/application. `verify-note`, resolution-aware
analysis rendering, invalidation after input changes, synthesis orchestration, and verification reporting
hooks still need integration.


### Verification of finding storage and review

- Initial behavioral failure: `/private/tmp/vpt-verification-red.log` showed a committed finding
  missing from the review snapshot.
- A second failing regression, `/private/tmp/vpt-verification-stale-batch-red.log`, showed both
  backends accepting findings followed by a conflicting content replacement in the same transaction.
  The shared validator now refuses that batch.
- `/private/tmp/vpt-verification-units-final.log`: 1,595 workspace unit tests passed before the final
  two stale-batch regression cases were added. The final targeted run,
  `/private/tmp/vpt-verification-contracts-final.log`, passed all 19 identity/storage cases,
  including those two new regressions.
- `/private/tmp/vpt-verification-commands.log`: all 134 command cases passed across confirmation,
  notes, pipeline, retention, review, and transcription, with `dev-tools` enabled.
- `/private/tmp/vpt-verification-clippy.log`: workspace/all-targets Clippy with `dev-tools` and
  warnings denied passed. Formatting and diff whitespace checks passed. All eight new production
  modules are at most 150 lines. Builds and timing-sensitive test suites ran sequentially.


## Markdown claims and annotation edits

Added a bounded Markdown adapter using pinned `pulldown-cmark` 0.13.4 with default features disabled.
It uses the parser's [source-offset events](https://docs.rs/pulldown-cmark/0.13.4/pulldown_cmark/struct.Parser.html#method.into_offset_iter)
to retain the original Markdown while extracting text for grounding.

Each list item produces one claim, including loose item paragraphs; nested items have their own claims.
Paragraphs split at sentence punctuation with decimal, common abbreviation, closing-quote, and trailing
citation handling. Extraction omits frontmatter, headings, fenced and indented code, and managed links.
Link labels remain claim text while destinations are excluded. Inline code and escaped brackets cannot
supply citations or verification annotations. Owned content has a separate entry point so its opening
horizontal rules are not mistaken for document frontmatter.

The adapter separates grounding text, exact citation strings, stable claim text, and source positions.
Citation syntax stays in the identity text, so an encoded malformed citation and a valid citation that
display alike still have different identities. Removing citations preserves boundaries between adjacent
number or name tokens. Transcript timestamps such as `[00:03]` remain distinct
from source ranges. Empty, inverted, and malformed time-range candidates reach the grounding checker.

Annotation edits remove earlier verification markers and insert safely escaped findings at claim
endings. They preserve surrounding Markdown and line endings. Repeated checks preserve identity and
produce the same bytes. A generated-analysis test covers inherited lexical uncertainty, unsupported
facts, and unsourced claims, and checks that extraction reproduces the original grounding and exact
analysis annotations.

Input bytes, Markdown syntax, parser events/depth, mapped text pieces, claims, citations, text sizes,
annotation payloads, and output bytes are bounded. Limits are checked before growing output collections.
Parsed claims are exposed immutably, and invalid byte offsets are refused. Sentence splitting avoids repeatedly scanning the
prefixes of long dotted words.

Both ledgers now accept an annotation-only note change atomically with its findings, publication
journal, and history. The validator compares the exact content with verification annotations removed.
The earlier regression refusing stale findings combined with actual claim edits remains in place.
Filesystem ownership checks, the verify-note command, resolution-aware analysis rendering, and
synthesis orchestration remain unfinished.

### Source locations and resolved annotations

Claims now expose their original UTF-8 byte ranges and one-based line numbers, including CRLF input,
paragraph sentences on the same line, and nested list items. A bounded newline index avoids scanning
the document again for each claim.

Resolved annotation rendering removes confirmed/dismissed markers and emits safely escaped
`[corrected: ...]` markers for corrections. Re-extraction excludes those markers from claim text,
citations, and identity. Corrections share the existing aggregate annotation budget. New tests caught
escaped bracket delimiters being treated as annotations; the scanner now checks the preceding escape
characters as well as the parser's source mapping.

`Document::replace_content` provides the owned-note update needed by verification. It preserves
frontmatter bytes, managed links, operator prose, and existing CRLF delimiters, and refuses unsupported
schemas and injected managed markers. These operations are ready for command integration; they do
not yet add the `verify-note` CLI or resolution-aware analysis document rendering.

### Validation and timeout investigation

The earlier command run passed confirmation, note, pipeline, and retention suites, but one review
case exceeded its existing 500 ms watchdog. Temporary phase traces showed about 2 ms for note/mention
preparation and varying stopping points during database operations and log export. They did not
establish a reproducible logic defect. All temporary instrumentation was restored from exact file
backups; no watchdog, database deadline, or durability step was changed.

The restored build passed all 17 review tests (`/private/tmp/vpt-markdown-review-restored.log`) and
all 19 transcription tests (`/private/tmp/vpt-markdown-transcribe.log`). The initial Markdown increment
also passed workspace/all-targets Clippy with `dev-tools` and warnings denied
(`/private/tmp/vpt-markdown-clippy-restored.log`).

Behavioral red tests for this increment are recorded in `/private/tmp/vpt-claim-locations-red.log`,
`/private/tmp/vpt-claim-correction-red.log`, `/private/tmp/vpt-finding-resolution-red.log`,
`/private/tmp/vpt-claim-resolution-red.log`, and `/private/tmp/vpt-note-content-red.log`.

The first workspace rerun passed 1,216 adapter cases but timed out the combined verification-corruption
scenario (`/private/tmp/vpt-verification-rendering-units.log`). That scenario mutated and restored 12
independent columns under one 500 ms watchdog. Those checks now run as 12 named scenarios, each with
the unchanged 500 ms watchdog and a fresh valid fixture. All 12 passed in the targeted run
(`/private/tmp/vpt-verification-corruption-cases.log`); the tested corrupt values are unchanged.

Final checks for this increment:

- `/private/tmp/vpt-verification-rendering-units-final.log`: all 1,643 workspace unit tests passed.
- `/private/tmp/vpt-verification-rendering-commands.log`: all 48 note and 17 review command tests
  passed with `dev-tools`, including the earlier review timeout case.
- `/private/tmp/vpt-verification-rendering-clippy.log`: workspace/all-targets Clippy passed with
  `dev-tools` and warnings denied. Formatting and diff whitespace checks passed.
- Every file used for temporary timing instrumentation matches its original backup exactly. No
  instrumentation remains. Builds and timing-sensitive suites ran sequentially.

## Verify-note command

Added `vpt verify-note <path> --recording <id>` with human and JSON results. It uses accepted transcript
words, lexical resolutions, and the configured grounding window without running an engine or agent.
Findings report their class, one-based line, and cited time ranges; findings alone return success.

External notes are observational, including unregistered copies, symlinked paths, malformed owned
markers, and renamed files whose new pin has not been committed. A different declared recording is
an ownership refusal. Annotation requires the matching ledger pin inside its configured stage store,
without symlink traversal, plus matching recording/stage metadata and valid managed regions. A new
regression demonstrated that a ledger pin outside the configured store initially authorized writes;
the ownership check now makes that case report-only.

Owned verification reads the current content, including edited claims. `VerificationChange` can carry
a checked disk snapshot: both ledgers validate its full/content digests, identity, expected saved note,
publication preimage, and annotation-stripped replacement content. A mismatched snapshot, publication
preimage, or unchecked claim replacement refuses the whole transaction. Grounding basis is computed
from the same accepted-transcript/flag snapshot used by the checker, then revalidated at commit.

The command saves artifact-specific findings, preserved resolutions, updated note bytes/counts,
publication intent, and history together. It retains operator prose, tags, links, and other metadata.
Repeated verification is byte-idempotent; corrected claims retain escaped correction markers and
changed claims start open. Both transcript and analysis findings contribute to the recording count.
Header count edits support block/flow YAML and CRLF. Reported lines account for a restored count field.

Mutating startup recovers history/publication/retention under the write lock. A `verify-note` run kind
is represented in ledger and exported history. Late transaction failure rolls back findings and notes;
publication acknowledgement failure reports the committed recording and can be repaired on retry.
Log export failure preserves successful verification with a safe warning. No private claim or correction
text is emitted in verification summaries or history.

Behavioral failures are recorded in `/private/tmp/vpt-verify-command-red.log`,
`/private/tmp/vpt-verify-observed-red.log`, `/private/tmp/vpt-note-count-red.log`,
`/private/tmp/vpt-verify-authority-red.log`, and `/private/tmp/vpt-verify-lines-red.log`.
The final focused command run, `/private/tmp/vpt-verify-command-final.log`, passed all 13 cases.
`/private/tmp/vpt-verify-units.log` passed all 1,647 workspace unit cases, and
`/private/tmp/vpt-verify-clippy-final.log` passed workspace/all-targets Clippy with `dev-tools` and
warnings denied.

Automatic invalidation when transcript/lexical inputs change, synthesis command integration, the owned
analysis renderer, and the remaining roadmap remain unfinished. The shared publication race and final
command-output acceptance audit remain tracked in the broader goal.

The final command regression run, `/private/tmp/vpt-verify-commands.log`, passed all 147 cases:
confirmation 18, notes 48, pipeline 14, retention 18, review 17, transcription 19, and verification 13.
Formatting and diff whitespace checks passed. All new production modules are at most 150 lines;
builds and timing-sensitive suites ran sequentially, and existing watchdog deadlines were unchanged.

## Analysis document rendering and proposal snapshots

The shared domain document renderer now accepts a typed transcript/analysis stage instead of
hard-coding transcript metadata. Both profiles retain the same provenance and bounded escaping rules;
analysis content uses the existing fixed Summary/Actions renderer. Proposal-ledger fixtures now save
real rendered analysis documents instead of changing a transcript fixture's stage string.

A generated-document integration test covers portable and Obsidian analysis ownership, escaped hostile
claims, replacement links, open counts, tag merging, unchanged operator prose and editable fields,
CRLF preservation outside managed regions, rewrite idempotence, and refusal of a different stage.
Reciprocal-link construction and the application/command integration are still unfinished.

`NoteInputs` now includes the accepted proposal. Memory reads it under the existing state lock;
SQLite decodes it within the same snapshot as transcript, flags, notes, tags, and relations. Shared
ledger tests assert proposal/note visibility after atomic acceptance. Reopen and read-only tests
check the snapshot field, and corrupt proposal state refuses the whole render snapshot without
exposing stored text. The read-only runtime now delegates the proposal and grounding-basis ports.

Behavioral red runs:
- `/private/tmp/vpt-analysis-document-red.log`: analysis had transcript stage metadata.
- `/private/tmp/vpt-analysis-snapshot-red.log`: both backends omitted the accepted proposal.

Focused checks passed: 22 proposal ledger tests and the generated analysis rewrite test. Existing
watchdog deadlines remain unchanged. Full regression results follow below.

Final validation passed:
- 1,649 workspace unit tests (`/private/tmp/vpt-analysis-doc-units.log`).
- 110 affected command cases: confirm 18, note 48, pipeline 14, review 17, verify-note 13
  (`/private/tmp/vpt-analysis-doc-commands.log`).
- Workspace/all-targets/dev-tools Clippy with warnings denied
  (`/private/tmp/vpt-analysis-doc-clippy.log`).
- Formatting and diff whitespace checks.

The full goal remains active. The next application work is rendering both notes from saved proposal
state with reciprocal links and resolution-aware annotations, then synthesis orchestration and stale
finding/inherited-uncertainty invalidation. No remaining feature checkbox was marked complete merely
from these foundational changes.

## Existing analysis notes in shared note updates

Shared note preparation now locates transcript and analysis targets before constructing either
publication. It uses each stage's configured store and pinned path, preserves first-write metadata,
and handles unique rename recovery for both. It prepares both note changes, publication preimages,
relation changes, and history for one ledger transaction. Invalid analysis ownership/markers or a late
analysis-row failure prevents the transcript/tag change from committing.

`note write`, review resolution, and tag/relation confirmation now update existing analysis notes.
Analysis uses its saved content, including claims saved by verification. Matching verification
resolutions are rendered by stage, claim position, and annotation-stripped digest; corrections are
escaped, confirmed/dismissed markers disappear, and lexical uncertainty markers remain intact.
Both notes get aggregate counts and reciprocal Markdown/wiki links to the final recovered pins.
Operator content outside managed regions and operator metadata/tags remain preserved.

Automatic/review/confirmation updates respect each artifact's expiration independently. A live
analysis note can still receive tag changes when its transcript has expired. Explicit `note write`
regenerates expired notes from saved state. The pipeline checks every prepared note/publication, so
an analysis-only change is visible in both the dry-run plan and the actual update. Dry runs preserve
filesystem state.

Analysis publication now checks the accepted proposal against the current transcript content digest
and grounding basis before marking synthesis succeeded. Metadata publication keeps stale synthesis
pending and preserves a failed/disabled synthesis state. New accepted synthesis can still complete
through its pending publication. Both ledger backends share the freshness comparison.

Behavioral red logs:
- `/private/tmp/vpt-dual-note-red.log`: the existing analysis note caused `analysis_unavailable`.
- `/private/tmp/vpt-analysis-republication-red.log`: both backends incorrectly completed stale synthesis.
- `/private/tmp/vpt-analysis-failed-stage-red.log`: metadata publication incorrectly cleared failure.
- `/private/tmp/vpt-analysis-pipeline-red.log`: an analysis-only change was absent from the plan.
- `/private/tmp/vpt-analysis-expired-source-red.log`: an expired transcript prevented live analysis updates.

Focused checks passed for two-note updates, resolution rendering, and proposal publication state.
Initial synthesis creation, proposal tag/relation selection, and automatic invalidation of stale
verification findings/inherited uncertainty remain unfinished. The shared publication race and final
command-output acceptance audit remain open. Production modules touched here stay within 150 lines.


The mixed-expiration audit also found and fixed review output's assumption that every resolution
publishes a note. Resolving a finding with both notes expired now saves the resolution without
regenerating either artifact; the result has a null publication path. When a note is updated, the
reported path belongs to the finding's artifact stage. The regression red is
`/private/tmp/vpt-expired-review-red.log`.

Validation encountered the existing 500 ms review-recovery command watchdog. It repeated in isolation;
temporary traces showed about 4 ms in note preparation and later variable stops during publication,
history export, and database close. This does not establish a cause. All seven temporarily instrumented
files were restored and byte-checked before the final output fix; no timing instrumentation remains.
The restored review suite subsequently passed all 17 cases with the unchanged deadline, followed by
19 transcription and 13 verification cases (`/private/tmp/vpt-dual-note-review-final.log`).

All 10 new analysis command cases passed (`/private/tmp/vpt-dual-note-final-focused.log`). The earlier
full command run passed confirmation 18, notes 57, pipeline 14, and retention 18 before the review
timeout; the later runs cover the added tenth note case and the remaining suites. The intermittent
watchdog observation is retained here rather than treated as a proven performance fix.

The checklist now marks existing-state `note write` and manual tag confirmation/rejection complete.
Synthesis creation, stale-finding invalidation, and the broader remaining scope stay open.

Final gates: 1,657 workspace unit cases passed; the CLI's 72 unit cases were rerun after the
expired-review output fix and passed. Workspace/all-targets/dev-tools Clippy with warnings denied,
formatting, and diff whitespace checks passed. Final logs are `/private/tmp/vpt-dual-note-final-units.log`,
`/private/tmp/vpt-dual-note-final-cli-units.log`, and `/private/tmp/vpt-dual-note-final-clippy.log`.
All build/test processes are finished, and no deadline or durability requirement was weakened.


## Initial synthesis findings and first analysis preparation

Proposal acceptance now checks the rendered analysis Markdown with the same bounded checker used by
`verify-note`. Both ledger transactions save the accepted proposal, analysis note/publication, and
replacement analysis findings together. Finding validation, stable identities, resolution preservation,
and aggregate storage budgets are shared with explicit verification. SQLite recounts recording flags
inside that transaction; memory snapshots derive the same counts from saved flags. A competing explicit
verification for the same recording is refused rather than ambiguously replacing the generated set.

Unchanged claims preserve confirmed, corrected, and dismissed decisions, and the candidate analysis
must render those decisions. Edited claims receive fresh open findings. Transcript findings remain
intact. Late history failures roll back the entire change. Typed proposal citations and grounding stay
saved separately from the final Markdown findings: malformed strings that do not constitute Markdown
ranges are classified from the actual rendered text, just as a later `verify-note` invocation does.
Repeated checking of the accepted content preserves its annotations and finding identities.

Shared note preparation accepts checked incoming analysis content. It can create the first analysis
artifact and update its transcript counterpart in the same delta, with final paths, reciprocal links,
first-write metadata, aggregate counts, and preserved operator prose. Existing ordinary note/review/tag
callers continue to use saved analysis content. Preparation itself performs no publication.

Behavioral reds:
- `/private/tmp/vpt-synthesis-findings-red.log`: both ledgers saved zero initial analysis findings.
- `/private/tmp/vpt-synthesis-first-note-red.log`: first synthesis prepared only the transcript note.

Focused coverage includes both ledger backends, all three saved resolution kinds, changed claims,
other-artifact findings, late rollback, competing verification, malformed citations and escaped content,
and initial notes for both profiles and both link styles. The first resolution test exposed an ordering
assumption in its assertion; findings are now compared by stable identifier. The first-note count fixture
was updated to carry the existing transcript count field; added YAML keys are validly quoted by the
rewriter. Neither issue required weakening production behavior.

The `synthesize` CLI orchestration, proposal tags/relations, stale-finding invalidation, and the broader
roadmap remain unfinished. No additional feature checkbox is marked complete from this groundwork.

Final validation passed:
- 1,666 workspace unit cases (`/private/tmp/vpt-synthesis-findings-units.log`).
- 157 command cases: confirm 18, note 58, pipeline 14, retention 18, review 17, transcribe 19,
  verify-note 13 (`/private/tmp/vpt-synthesis-findings-commands.log`).
- Workspace/all-targets/dev-tools Clippy with warnings denied
  (`/private/tmp/vpt-synthesis-findings-clippy.log`).
- Formatting and diff whitespace checks. Changed production modules remain at or below 150 lines.

The review-recovery watchdog passed in this run with its existing deadline unchanged. All build and
test processes are terminal. The full goal remains active; no commit, push, or publication was made.

## Standalone synthesis command

`vpt synthesize <id> [--dry-run] [--json]` now connects configured command execution to accepted
proposal state and owned analysis publication. An empty command refuses with configuration exit 2.
Dry runs return the planned input digest and operations without creating notes, history, directories,
or child processes, including when publication is pending. Mutating execution takes the write lock,
recovers history/publication/retention, publishes the current transcript, and sends its exact content
bytes and configured argument substitutions to the bounded agent adapter.

The command rechecks source ownership and content before and after execution. Agent failure, malformed
output, source edits, or a late transaction failure preserve prior accepted proposal/findings/analysis.
Synthesis failures save the failed stage without replacing accepted state. Successful execution grounds
and checks the generated Markdown, then saves proposal, finding replacement, tag/relation state, known-tag
publication, and both note publications in one transaction. Output includes the record and finding class
counts without claim text or proposal provenance.

Tag vocabulary and relation-target resolution are now shared with manual confirmation. Synthesis uses
one per-run index, applies hold/write, deduplication and caps, preserves manual decisions and vocabulary
spelling, and records private proposal provenance. A bounded batch known-tag update journals multiple
selected additions together. Related targets may be indexed names/aliases or recording identities;
external indexed notes remain unchanged. Invalid/capped tags create typed `tag_rejected` history facts
with original proposal indexes and rejection reasons, without tag text and without notification hooks.
The SQL and exported-log codecs support these facts and the new `synthesize` run kind.

Retries recognize pending owned-note publications associated with saved proposal state. After repair,
a still-current input digest, grounding basis, configured window, and published source allow returning
the saved analysis without rerunning the agent. This covers interrupted acknowledgement of either the
analysis or its transcript counterpart. Source changes require fresh synthesis instead of reusing stale
accepted input. Explicit repeated synthesis otherwise runs the configured command and retains matching
review resolutions, pinned paths, first-write metadata, and operator prose/tags.

Behavioral reds:
- `/private/tmp/vpt-synthesize-command-red.log`: synthesize was an unknown verb.
- `/private/tmp/vpt-synthesize-metadata-red.log`: proposal tags were not saved.
- `/private/tmp/vpt-synthesize-workflow-red.log`: analysis recovery incorrectly reran the agent.
- `/private/tmp/vpt-synthesize-tag-history-red.log`: tag rejection reasons were absent from history.
- `/private/tmp/vpt-synthesize-transcript-recovery-red.log`: later transcript acknowledgement failure
  also incorrectly reran the agent after analysis publication had already succeeded.

Final validation passed:
- 1,667 workspace unit cases (`/private/tmp/vpt-synthesize-units.log`).
- 169 command cases in the broad run: confirmation 18, notes 58, pipeline 14, retention 18, review 17,
  synthesis 12, transcription 19, verification 13 (`/private/tmp/vpt-synthesize-commands.log`).
- All 13 synthesis cases after the additional transcript-recovery regression and fix, bringing distinct
  command coverage to 170 (`/private/tmp/vpt-synthesize-final-focused.log`).
- Workspace/all-targets/dev-tools Clippy with warnings denied (`/private/tmp/vpt-synthesize-clippy.log`).
  Its first run caught one unused import after splitting history generation; that import was removed.
- Formatting and diff whitespace checks. Changed production modules stay within 150 lines.

The roadmap marks standalone command/configuration, exact agent input/substitution, and accepted proposal
execution/storage complete. Automatic synthesis in `run`, stale verification/inherited-uncertainty
invalidation, synthesis review notifications, and broader later-stage work remain open. Vocabulary-only
publication recovery with custom store ordering still needs an acceptance case.

The link audit identified a separate concrete gap: first transcript and analysis filenames normally
match, while current wiki counterpart links use only the basename. This can make the intended target
ambiguous. Obsidian documents vault-relative folder-qualified wikilinks in its
[internal links guide](https://obsidian.md/help/links); portable wiki roots and notes outside the configured vault need a
consistent policy before that audit item can close. Markdown counterpart links already include relative
store paths. No profile/link checkbox was marked complete from the current implementation.

All build/test processes are terminal. Watchdog deadlines and durability requirements remain unchanged.
The full goal stays active. No commit, push, or external publication was made.

## Verification freshness (2026-09-29)

Both ledgers now retain the exact checked input digest, transcript digest, lexical-state digest,
artifact stage, and check time for verification, including successful checks with zero findings.
Proposal acceptance saves the analysis check's actual input digest alongside its findings in the same
transaction. SQLite schema 9 adds `verified_artifacts`; older findings remain stored but require a
fresh check because their original grounding basis cannot be reconstructed reliably.

Review and note-input snapshots exclude stale findings from active flags and retain them separately
for rechecking. Transcript or lexical changes invalidate both artifacts' findings, and a previously
successful synthesis becomes pending when its proposal basis no longer matches. Failed, disabled,
and expired synthesis states are preserved. SQLite refreshes counts after all changes in the work
transaction, before history is committed. Direct attempts to resolve a stale finding refuse until it
has been checked again.

Lexical resolution invalidates the application snapshot before preparing note publications. Both notes
therefore receive the new aggregate count in the same transaction. Analysis rewrites remove stale
verification annotations; transcript rendering preserves its freshly rendered lexical corrections.
The command regression suite caught an initial mistake that stripped those corrections, and the
rendering path was corrected. `verify-note` and standalone synthesis consult retained decisions when
rechecking, so unchanged claims regain their prior confirmation, dismissal, or correction. Rechecking
one artifact does not revive the other's stale findings.

Behavioral failures before fixes:
- `/private/tmp/vpt-freshness-red.log`: both backends kept findings current after lexical and
  transcript changes.
- `/private/tmp/vpt-freshness-cli-red.log`: note rewriting retained a stale correction annotation.
- `/private/tmp/vpt-freshness-resolution-red.log`: both ledgers accepted resolutions of stale findings.
- `/private/tmp/vpt-freshness-commands.log`: three existing review cases detected stripped lexical
  corrections; those cases pass after the rendering fix.

Validation:
- Workspace unit suite: 1,675 cases passed (`/private/tmp/vpt-freshness-units.log`).
- Final focused verification suite: 51 cases passed, including five additional migration/corruption
  cases and the strengthened empty-result rollback case (`/private/tmp/vpt-freshness-final-focused.log`).
  This brings distinct unit coverage to 1,680 cases. Read-only schema-8 observation leaves files
  unchanged; failed migration rolls back its table/version; successful migration preserves findings
  and history while updating current counts. Corrupt digest/time fields refuse safely through both
  snapshot readers.
- 172 distinct command cases passed across the broad and follow-up runs: confirmation 18, notes 58,
  pipeline 14, retention 18, review 17, synthesis 15, transcription 19, verification 13.
  Logs: `/private/tmp/vpt-freshness-commands.log`, `/private/tmp/vpt-freshness-command-fix.log`, and
  `/private/tmp/vpt-freshness-commands-final.log`. The first two runs' failures are superseded by
  successful follow-ups. The synthesis fixture also needed its confidence value aligned with its
  accepted word before exercising the new behavior.
- Workspace/all-targets/dev-tools Clippy with warnings denied passed after removing a redundant trait
  bound and collapsing one conditional (`/private/tmp/vpt-freshness-clippy.log`).
- Formatting and diff whitespace checks passed. Changed production modules remain within 150 lines.

The roadmap remains at 28 checked and 79 unchecked items. A final code inspection found a remaining
review/verification interaction: the shared Markdown token parser treats every literal `[corrected: ...]`
as a verification annotation, including lexical corrections in transcript content. The new stale-note
path preserves freshly rendered lexical corrections, but direct verification of an already corrected
transcript still needs a regression test and a fix. The review-resolution checkbox therefore remains
open despite the completed freshness work. Automatic rechecking and
synthesis in `run`, rendered-input comparison before pipeline selection, synthesis notifications, wiki
counterpart disambiguation, and the broader remaining product scope stay open. This work establishes
freshness for transcript and lexical changes; a broader audit of claim-content changes during ordinary
note rewrites remains required when integrating automatic verification.

All compiler/test processes are terminal. Watchdog limits are unchanged. The full goal remains active;
no commit, push, or external publication was made.

## Distinguishing lexical and finding corrections (2026-09-30)

Fixed direct verification and subsequent note rewrites removing lexical correction markers from owned
transcripts. Owned transcript extraction now keeps unmarked lexical corrections in claim text and
identity, including punctuation and escaped range-like text inside a correction. A saved legacy finding
correction is recognized only when its terminal marker, claim index, original claim identity, and saved
resolution match. This allows existing unchanged finding corrections to be read and rewritten safely.

New finding corrections keep the visible `[corrected: ...]` marker and append the invisible
`<!-- vpt:verification-correction -->` role marker. A test showed that saved identity alone cannot
disambiguate identical lexical and finding corrections after a claim edit. The explicit marker removes
that ambiguity for newly written annotations. Marked finding corrections are excluded from extraction
and removed/replaced even after edits; an unrecognized unmarked correction remains claim text. The
product specification's note-grounding section and README now document these rules.

The shared checker, owned-note finding renderer, and ledger's checked-content comparison all use the
same owned-content extraction. This also keeps a lexical correction from being silently removed by the
ledger's annotation-only comparison. Paragraph/list positions account for the invisible marker's byte
range, so repeated annotation updates do not overlap edits or accumulate markers.

Behavioral reds:
- `/private/tmp/vpt-correction-identity-red.log`: `verify-note` removed a lexical correction.
- `/private/tmp/vpt-correction-edits-red.log`: editing a claim with identical lexical and finding
  correction text retained the old finding correction as part of the claim.

Validation passed:
- 1,684 workspace unit cases (`/private/tmp/vpt-correction-units.log`). Four new parser cases cover
  edited claims with identical correction text, punctuation/range-like correction payloads, legacy
  correction recognition/upgrade, and formatted or adjacent claims with repeated updates.
- 172 command cases (`/private/tmp/vpt-correction-commands.log`): confirmation 18, notes 58,
  pipeline 14, retention 18, review 17, synthesis 15, transcription 19, verification 13. The expanded
  synthesis/review case verifies twice, resolves a finding with text identical to its lexical
  correction, then repeatedly verifies and rewrites while checking both markers and saved decisions.
- Workspace/all-targets/dev-tools Clippy with warnings denied (`/private/tmp/vpt-correction-clippy.log`).
- Formatting and diff whitespace checks. Changed production modules remain within 150 lines.

Review resolution is now checked off: 29 checked and 78 unchecked roadmap entries. The next freshness
acceptance check concerns claim-content changes during ordinary note rewrites, where the saved
verification basis currently compares transcript and lexical state. Automatic rechecking, synthesis in
`run`, review notifications, wiki disambiguation, and the broader product scope remain open. No goal
completion claim is made. All test/compiler processes are terminal; no deadlines were widened and no
commit, push, or external publication was performed.

## Claim freshness during ordinary note rewrites (2026-09-30)

Verification now records a fingerprint of the ordered, annotation-stripped claims in addition to its
exact checked-content digest and transcript/lexical basis. The fingerprint includes claim position,
text, and citation syntax; verification annotations and excluded structure such as headings do not
change it. Both ledgers compare it with the saved owned artifact when reading review or note inputs.
Changing claims invalidates only that artifact, including a previously checked empty finding set.
Stored decisions remain available for a later recheck; new claim identities start open.

Shared note preparation computes the prospective content of both targets before rendering metadata.
If a target's claims differ from its current verification fingerprint, a temporary review snapshot
excludes its stale findings before either note's counts or annotations are rendered. The original
snapshot is borrowed unless invalidation needs a copy. SQL recounting also runs after ordinary note
changes in the same work transaction, so ledger counts and published frontmatter agree. Standalone
synthesis replaces its analysis verification input explicitly, preserving its freshly checked findings
while other artifacts remain independently current or stale.

SQLite schema 10 adds the nullable `claims_digest` column to `verified_artifacts`. Schema-9 records
without this evidence require rechecking; migration preserves their input/basis digests, finding rows,
resolutions, and history. Read-only observation of the older schema remains observational. Failed
migration rolls back the column and version; successful migration updates current counts without
inventing a historical claim fingerprint.

Behavioral reds:
- `/private/tmp/vpt-claim-freshness-red.log`: both ledgers retained current findings after an ordinary
  owned-note content change.
- `/private/tmp/vpt-claim-freshness-cli-red.log`: changing readability removed the annotation but
  published the old open-finding count.

Validation passed:
- 1,691 distinct workspace unit cases. The broad run passed the 73 CLI-library cases and 1,272 of
  1,273 adapter cases (`/private/tmp/vpt-claim-freshness-units.log`). A mechanical schema-version test
  edit had changed the future-version expectation from 99 to 109; that expectation was corrected,
  and the focused future-schema test passed (`/private/tmp/vpt-claim-freshness-schema-fix.log`).
  The remaining application 61, domain 214, and protocol 70 cases passed in
  `/private/tmp/vpt-claim-freshness-other-units.log`.
- New coverage includes both backends' artifact-specific and empty-result invalidation, schema-9
  upgrade/rollback/read-only observation, malformed claim fingerprints, and fingerprint stability
  across annotation/headings changes while lexical correction/citation/claim changes alter it.
- 173 command cases (`/private/tmp/vpt-claim-freshness-commands.log`): confirmation 18, notes 58,
  pipeline 14, retention 18, review 17, synthesis 15, transcription 19, verification 14. The new
  readability case verifies stale flags, published zero counts, retained prior decisions, and a fresh
  identity after rechecking the changed text.
- Workspace/all-targets/dev-tools Clippy with warnings denied passed
  (`/private/tmp/vpt-claim-freshness-clippy.log`), along with formatting and diff whitespace checks.
  Changed production modules remain within 150 lines.

The roadmap stays at 29 checked and 78 unchecked entries. The next integration work is automatic
rechecking and synthesis in `vpt run`: select work from current inputs, preserve expired stages, avoid
rerunning unchanged synthesis, and finish notifications and partial-progress output. The full goal
remains active. All compiler/test processes are terminal. No watchdog limits were changed, and no
commit, push, or external publication was performed.

## 2026-09-30: automatic synthesis, stale checks, and review alerts

`run` now continues through configured synthesis after transcription/note publication. It reuses the
per-run note index, compares the rendered input digest and grounding inputs, skips unchanged agent
work, and retries failed synthesis without repeating successful transcription. Results list synthesized
recordings; child work retains committed identifiers and its run history on partial failure.

Previously verified transcript claims are rechecked after input changes and before synthesis. Stale
analysis claims are rechecked when synthesis does not replace them, including with synthesis disabled
or the transcript expired. Both notes receive updated open counts, and automatic work preserves
expired artifacts. Dry-run selection reports the earliest required step without repair or processes.
Accepted pending analysis publication repairs without another agent call or repeated notification.

Review alerts now count unsourced, bad-reference, unsupported, and built-on-flagged-text findings.
The new count fields are additive in hook/history output; old saved events default absent fields to
zero, and malformed counts refuse without private values. Pipeline notices replace earlier counts
with the latest successfully published counts, aggregate once per recording, and survive a later
synthesis failure. Standalone synthesis reports review only after analysis publication.

Behavioral red evidence:
- `/private/tmp/vpt-pipeline-synthesis-red.log`: pipeline skipped configured synthesis and its failures.
- `/private/tmp/vpt-pipeline-verify-red.log`: changed transcript claims remained stale after `run`.
- `/private/tmp/vpt-synthesis-alert-red.log`: standalone synthesis emitted no alert; pipeline alerts
  omitted synthesis findings.
- `/private/tmp/vpt-pipeline-later-recovery-red.log`: dry-run selection missed live analysis work when
  its transcript had expired. The corrected plan first refreshes analysis metadata, then verifies it.

Remaining integration work includes durable configuration-driven disabled/pending stage transitions,
interrupted run outcomes, verification freshness when grounding-window settings change, and the
standalone vocabulary-only publication recovery ordering case. Wiki counterpart disambiguation,
briefs/context, redaction/releases/handoff, full-store retention, installation, and the roadmap's
remaining acceptance audits are still open. No roadmap checkbox is closed by this partial audit.

Validation passed for this increment:
- 1,695 workspace unit cases: CLI library 73, adapters 1,274, application 63, domain 214,
  protocol 71 (`/private/tmp/vpt-pipeline-later-units.log`).
- 183 command cases: confirmation 18, notes 58, pipeline 24, retention 18, review 17,
  synthesis 15, transcription 19, verification 14
  (`/private/tmp/vpt-pipeline-later-all-commands.log`).
- Workspace/all-targets/dev-tools Clippy with warnings denied
  (`/private/tmp/vpt-pipeline-later-clippy.log`), formatting, and diff whitespace checks.
- All 23 production modules inspected for this increment remain within 150 lines.

The roadmap remains 29 checked and 78 unchecked entries. All compiler/test processes are terminal.
The goal remains active; next work is durable stage eligibility and the remaining freshness/recovery
boundaries above. No watchdog limits changed. No commit, push, or external publication was performed.

## 2026-09-30: durable synthesis eligibility and interrupted run outcomes

Synthesis eligibility now commits through a typed state change and matching
`synthesis_state_changed` history fact. Disabled configuration moves pending/failed work to disabled;
enabling it moves disabled work to pending. Successful and expired outputs remain intact. Changed
rendered input or grounding settings mark enabled successful work pending before artifact selection,
including when a subsequent note ownership/marker refusal prevents agent execution. Each attempted
synthesis also becomes pending after source preparation and before its agent starts.

Both ledger backends require the expected previous state, a permitted transition, one matching
history fact at the same time, and a state-only transaction. Stale/unknown recordings, fabricated or
duplicate facts, mixed mutations, attempts to revive expiration, and missing history refuse. A later
history failure rolls back the state; scheduling itself does not appear as completed synthesis.
The existing StageState value now lives in the domain and remains re-exported through the application
ports. No schema migration or state-value change was needed.

Interrupted engine and agent errors now retain an internal interruption marker through command
composition. Child synthesis runs and parent pipeline runs use it to record `interrupted` rather than
`failed`. The public error JSON and operational-failure exit code are unchanged. Failed stage state
still permits an explicit later retry. Protocol error tests were moved to a separate test module.

Behavioral red evidence:
- `/private/tmp/vpt-eligibility-red.log`: configuration-off left incomplete work pending/failed.
- `/private/tmp/vpt-eligibility-ledger-red.log`: both ledgers ignored the requested state change and
  admitted inconsistent history before the new transaction handling was connected.
- `/private/tmp/vpt-run-interrupt-red.log`: actual SIGINT during agent/engine execution recorded
  failed child/parent outcomes instead of interrupted outcomes.

The next freshness boundary is verification after grounding-window configuration changes. Accepted
proposals retain their window, but verified_artifacts and VerificationChange do not yet retain the
window used by the checker. That needs durable metadata, legacy/read-only migration behavior, and
pipeline comparison without losing unchanged review decisions. The standalone vocabulary-only
publication-recovery ordering case, wiki counterpart disambiguation, briefs/context, sharing/handoff,
full-store retention, installation, and the roadmap acceptance audits also remain open.

Validation passed for eligibility and interruption handling:
- 1,706 workspace unit cases: CLI library 73, adapters 1,283, application 63, domain 214,
  protocol 73 (`/private/tmp/vpt-eligibility-interrupt-units.log`).
- 189 command cases: confirmation 18, notes 58, pipeline 30, retention 18, review 17,
  synthesis 15, transcription 19, verification 14
  (`/private/tmp/vpt-eligibility-interrupt-commands.log`).
- Workspace/all-targets/dev-tools Clippy with warnings denied
  (`/private/tmp/vpt-eligibility-interrupt-clippy.log`), formatting, and diff whitespace checks.
- New production modules are within 150 lines. The existing recovery planner is 240 lines after
  adding the new fact to its work-bearing cases; it remains below the 250-line ceiling.

The roadmap remains 29 checked and 78 unchecked entries pending the complete acceptance audit.
All compiler/test processes are terminal. No watchdog limits changed; no commit, push, or external
publication was performed. The goal remains active.

## Grounding-window freshness and vocabulary publication recovery

Saved checks now retain their grounding window, including checks that found no issues. `run` compares
the saved value with the configured window and rechecks live transcript/analysis artifacts when it
changes. Rechecking works with synthesis disabled, retains decisions for unchanged claims, updates
both notes' counts, and leaves expired analysis expired. The pipeline regression narrows and widens
the window around actual timed evidence, verifies the corresponding annotation/count changes, and
proves that neither the agent nor transcription engine reruns.

SQLite schema 11 adds nullable `grounding_window_secs` to `verified_artifacts`. Legacy checks keep
their findings and decisions but become stale; the migration does not invent the window that was
used. The migration test checks read-only byte preservation, rollback of an interrupted upgrade,
retained confirmed findings/history, and a subsequent recheck. Both ledger contracts cover empty
sets, artifact replacement, zero and maximum u32 windows. Corrupt text, negative, and overflowing
window values refuse without exposing the stored value.

The standalone synthesis recovery test exposed a separate bug: sorted publication targets can put
the vocabulary after both notes. If its acknowledgement fails, the saved proposal is complete but
the retry previously reran the agent because it only recognized pending notes. Accepted proposals
now retain the exact vocabulary publication from the same transaction. The shared ledger validator
requires a matching tag publication; missing, mismatched, or differently timed dependencies refuse.
The version-2 saved proposal codec persists the dependency and still reads version-1 proposals
without inventing one. The SQLite schema remains 11.

Recovery recognizes the exact saved dependency, repairs it, and returns the accepted result when
source and grounding inputs remain current. Tests also prove that changing the grounding window
still runs synthesis, an unrelated pending manual tag update does not suppress an explicit request,
dry runs perform no recovery, and recovery produces no synthesis alert. Both backend contracts
cover dependency persistence after note acknowledgement, transaction consistency, and rollback on
late history failure. Codec tests cover legacy reads, exact round trips, bounded paths, malformed
digests, unsafe paths, unknown fields, and safe errors.

Behavioral failure evidence:

- `/private/tmp/vpt-grounding-window-red.log`: changing the grounding window did not schedule a
  verification pass.
- `/private/tmp/vpt-vocabulary-recovery-red.log`: a retry with only vocabulary publication pending
  invoked the deliberately failing replacement agent instead of returning the accepted proposal.

### Grounding persistence acceptance audit

The final Note grounding checklist item is complete. The evidence covers its stated requirements:

| Requirement | Implementation and behavioral evidence |
| --- | --- |
| Identity includes artifact, claim position, and annotation-stripped digest | `flag_identity.rs`, `verification.rs`, Markdown extraction; identity tests change each field independently and preserve identity across reclassification. |
| Replace only the selected artifact; preserve unchanged decisions | Shared verification preparation and backend replacement; both ledger contracts exercise confirmed, corrected, and dismissed findings, changed claims, and the other artifact's retained set. |
| Recompute open counts | SQLite verification refresh and command rendering; `verify_note/analysis.rs` checks both finding sets and the stored count, while pipeline window tests check both note headers. |
| Annotate claim endings and retain corrections | `markdown_claims/render.rs`; correction and resolution tests verify exact placement, escaping, stable extraction, and separate lexical/finding markers. |
| Ground analysis before synthesis success | `synthesize/prepare.rs` checks rendered analysis; proposal validation repeats the grounding checks before atomic acceptance. Synthesis command tests inspect saved findings, annotated output, counts, and accepted input digests. |
| Preserve state across failure and recovery | `verify_note/workflow.rs`, synthesis workflow/recovery tests, and backend contracts cover late rollback, failed acknowledgement, committed IDs, publication repair, and log-export warnings. |
| Recheck stale claims, inputs, and settings | Verification freshness contracts and pipeline/readability/window tests cover transcript, lexical, claim, and window changes, empty results, retained review decisions, and expiration. |

The audit closes this grounding item only. Wiki counterpart disambiguation, the broader synthesis and
profile audits, stdout-failure reporting of committed work, publication race protection, briefs and
context, sharing and handoff, full-store retention, installation, and the other roadmap items remain.
The checklist now contains 30 checked and 77 unchecked entries.

Validation for grounding windows and vocabulary recovery:

- 1,720 workspace unit tests passed: CLI library 73, adapters 1,297, application 63, domain 214,
  protocol 73 (`/private/tmp/vpt-window-recovery-units.log`).
- Before the vocabulary recovery change, all 191 command tests passed
  (`/private/tmp/vpt-grounding-window-commands.log`). The three added recovery boundary tests passed
  (`/private/tmp/vpt-vocabulary-recovery-boundaries.log`).
- After the recovery change, confirmation 18, notes 58, pipeline 32, and retention 18 passed in
  `/private/tmp/vpt-window-recovery-commands.log`. That run stopped at review: two cases exceeded
  the unchanged 500 ms command deadline. In `/private/tmp/vpt-window-recovery-review-recheck.log`,
  both passed, but the absent/invalid-recording test exceeded that deadline. Its isolated rerun
  passed (`/private/tmp/vpt-window-recovery-review-absent.log`). The timeout cause is unproven.
- The remaining synthesis 18, transcription 19, and verification 14 cases passed in
  `/private/tmp/vpt-window-recovery-remaining-commands.log`. All 194 distinct command cases have
  passing evidence across these runs; the combined command invocation was not a clean pass.
- Workspace/all-targets/dev-tools Clippy passed with warnings denied
  (`/private/tmp/vpt-window-recovery-clippy.log`). Formatting and diff whitespace checks passed.
- The changed production modules remain within 150 lines. No watchdog or command deadline changed.

All compiler/test processes are terminal. No commit, push, or external publication was performed.
The complete VPT goal remains active.

## Exact wiki targets and deterministic managed links

Wiki links now retain the destination path. The previous renderer used only the display label,
which made the transcript and analysis counterparts ambiguous when their filenames matched.
Obsidian-profile links use paths from the configured vault root. Portable-profile links use the
common parent of the transcript, analysis, and brief stores, as selected by the user. Display labels
remain the filenames without their Markdown extension. Non-Markdown extensions, including audio,
remain in the destination.

The shared renderer applies this rule to reciprocal links, saved relations, audio references, and
hubs. Targets outside the chosen root and names that cannot safely appear in wiki syntax use encoded
relative Markdown links. Paths and labels are bounded before allocation, invalid UTF-8 refuses, and
reserved wiki characters or line separators cannot introduce a second link. Markdown style retains
its relative destination behavior. The implementation uses the resolved note-store roots already
passed into note preparation; it performs no new filesystem traversal or writes to indexed notes.

The syntax follows the [Obsidian internal links guide](https://obsidian.md/help/links): wiki folder
paths begin at the vault root, display text can use an alias, and Markdown destinations are encoded.
The specification and README now state the portable root and fallback rules explicitly.

Behavioral evidence:

- `/private/tmp/vpt-wiki-links-red.log` reproduces the ambiguous basename-only reciprocal link in
  a generated pair of same-named notes under a nested vault directory.
- `synthesize/links.rs` verifies the exact reciprocal destinations, first publication, stable note
  bytes on rewriting, both profiles and both link styles, rename recovery with `#` in the filename,
  preserved operator prose, and audio references outside the vault.
- Its portable relation case places the brief store outside the two other stores' parent and creates
  duplicate indexed filenames. Both notes use the root determined by all three stores and link to
  the selected external file. Both external files retain their original bytes.
- Application tests cover root-level and nested targets, Unicode/space handling, non-Markdown
  extensions, component-wise root membership, encoded fallback, invalid bytes, bounds, and syntax
  injection attempts.
- Existing relation command cases verify escaped provenance, suggestion caps, rejected-link
  omission, stable repeated rendering, pinned rename updates, transactional rollback, and unchanged
  content/operator prose/external notes. Reciprocal links are emitted before the relations, which
  remain sorted by relation kind and saved target.

The deterministic managed-links checklist item is complete. The roadmap now has 31 checked and
76 unchecked items. The broader analysis/profile acceptance audits and the other product stages
remain open.

Validation:

- 1,723 workspace unit tests passed: CLI library 73, adapters 1,297, application 66, domain 214,
  protocol 73 (`/private/tmp/vpt-wiki-links-units.log`).
- 198 distinct command tests passed. Notes 58, pipeline 32, and synthesis 21 passed in
  `/private/tmp/vpt-wiki-links-commands.log`; the additional synthesis relation/root case passed in
  `/private/tmp/vpt-wiki-links-related.log`. Confirmation 18, retention 18, review 17, transcription
  19, and verification 14 passed in `/private/tmp/vpt-wiki-links-other-commands.log`.
- Workspace/all-targets/dev-tools Clippy passed with warnings denied
  (`/private/tmp/vpt-wiki-links-clippy.log`). Formatting and diff whitespace checks passed.
- Changed production modules remain within 150 lines. Deadlines and watchdog limits are unchanged.

Native verification limitation: the [Obsidian Markdown skill](/Users/stephen/.agents/skills/obsidian-markdown/SKILL.md)
asks to "Verify the note renders correctly in Obsidian's reading view." The Obsidian CLI reported
that it could not find a running instance, and the desktop tool returned `noWindowsAvailable` after
a long delay. Reading-view rendering and link clicks were therefore not verified in Obsidian itself;
the passing evidence above is from source inspection, documented syntax, and automated tests.

All compiler/test processes are terminal. No commit, push, or external publication was performed.
The full VPT goal remains active. The next identified implementation gap is preserving committed
recording IDs when writing a successful command's final stdout result fails.


## Committed work survives final-output failure

A successful command now carries explicit internal completed-recording IDs and safe diagnostics.
If writing its final stdout result fails, the error on stderr retains those IDs and diagnostics.
The emitter does not infer completed work from public JSON fields or copy private result text into
the error. This applies even when the output sink accepts a prefix before failing. Bytes already
accepted by an output sink cannot be retracted; the error remains a single final stderr document.

Ingestion includes newly ingested and recovered archives, retention includes completed moves and
reconciliation, and the pipeline deduplicates recording IDs across completed stages. The shared
note mutation finish path covers note writes, synthesis, owned-note verification, review resolution,
and tag/relation confirmation. Transcription retains its accepted recording ID. Startup retention
recovery merges its IDs without losing or repeating IDs from the command itself. Dry runs, external
note verification, and unchanged ingest/pipeline retries do not claim completed recordings.

Human failures now list completed IDs centrally, including normal operational failures and partial
reports. Ingest and retention partial reports use that shared list without duplicating IDs. Safe
log-export warnings survive stdout failure, along with safe engine/proposal/helper diagnostics that
were already available to the result. Ingestion result serialization failures also retain recovered
recording IDs and diagnostics. Output unit tests now live separately from the production module.

Behavioral evidence:

- `/private/tmp/vpt-completed-output-red.log` records three failing note command cases: saved notes
  survived, but human/JSON errors omitted the committed ID and lost logging warnings.
- Those three cases pass in `/private/tmp/vpt-completed-output-green.log`.
- `/private/tmp/vpt-completed-output-focused.log` passes 22 CLI/output-related unit tests and nine
  focused command cases across ingestion, notes, pipeline, retention, and transcription. Assertions
  cover durable artifacts/state, recovered IDs, deduplication, unchanged retries, observational dry
  runs, retained logging warnings, and absence of private text from output errors.

Final validation for the output change:

- All 391 integration cases across the 18 CLI test targets have passing evidence. The initial
  invocation (`/private/tmp/vpt-completed-output-commands.log`) passed through review and stopped at
  six setup terminal fixtures because the sandbox refused `/dev/tty`. All 18 setup cases passed
  when rerun with terminal access (`/private/tmp/vpt-completed-output-setup.log`). The seven remaining
  targets passed in `/private/tmp/vpt-completed-output-remaining.log`. No fixture deadline changed.
- All 75 CLI library unit tests and the fake-engine binary's one unit test passed in the initial
  invocation. The change adds two output unit tests and 13 integration tests, including synthesis,
  review corrections, confirmed tags/vocabulary, and owned versus external note verification.
- Workspace/all-targets/dev-tools Clippy passed with warnings denied
  (`/private/tmp/vpt-completed-output-clippy.log`). Formatting and diff whitespace checks passed.
- Changed production modules remain within 150 lines; the largest is 125 lines. No schema migration,
  new dependency, unsafe block, commit, push, or external publication was introduced.

All test and compiler processes are terminal. The roadmap remains at 31 checked and 76 unchecked
items; the shared output item includes commands from product stages that are still unfinished.
The full VPT goal remains active. The pending shared-publication check/rename race and remaining
feature acceptance checks still require work.


## Expected-state checks at artifact publication

The application previously checked the target digest, rendered the saved artifact, and then called
an adapter that staged bytes and unconditionally renamed them over the target. A target created or
edited after the application check could therefore be replaced. The behavioral red in
`/private/tmp/vpt-publication-guard-red.log` reproduces this: the renderer creates operator content,
but repair returns `Ok(true)` instead of refusing the changed target.

`Stores::publish` now requires the expected previous digest (or an absent expectation). The filesystem
adapter checks that expectation both before and after staging. First publication uses exclusive
rename, so a file, directory, or symlink that arrives after the final absence check remains in place.
Existing-file replacement checks the expected bytes again after staging. The staged name must still
refer to the retained file, have one link, and contain exactly the intended bytes; validation uses an
8 KiB buffer and reads at most the expected length plus one byte. Name allocation stops after 64
collisions and preserves every existing occupant. Failed staging/publication continues to retain its
temporary file, as the previous adapter did; it does not delete a name that could have been replaced.

The adapter rechecks the published digest after directory synchronization. Application repair also
checks the intended digest after its synchronization step, including the already-published recovery
path, before clearing the journal. Detected changes return a typed refusal and leave the intent
pending. The command mapping uses `publication_target_modified` without exposing private path text.
Store-root acquisition/authority code and composition error tests were separated into cohesive files.

The tests cover edits between the application check and adapter entry, target creation at the last
pre-rename boundary, root replacement, modified/replaced/linked/symlinked temporary files, trailing
bytes, changes after rename, and journal preservation on changes during sync. Existing store cases
still cover successful replacement, directory synchronization, contained paths, private 0600 engine
outputs, shared 0644 note output, and unchanged directory permissions. Directory occupants now refuse
before creating a temporary file.

Remaining limit: replacement of an existing file still uses check-then-rename. A writer replacing the
existing target after the adapter's final check can still lose that replacement. The source temporary
name also remains subject to a last-check/rename race. These changes do not establish atomic
compare-and-replace for existing files. The [Apple rename contract](https://raw.githubusercontent.com/apple-oss-distributions/xnu/main/bsd/man/man2/rename.2)
provides exclusive destination creation and atomic swapping, with mutually exclusive flags; neither
operation accepts an expected digest. This implementation adds no swap/backup recovery protocol.
The complete shared-publication requirement therefore remains unchecked.

Validation:

- The focused 28 store tests passed, followed by all 1,734 workspace unit tests (CLI 76, adapters
  1,304, application 67, domain 214, protocol 73) in `/private/tmp/vpt-publication-guard-units.log`.
- The combined command run passed confirmation 18, ingestion 20, notes 62, pipeline 34, and retention
  19 cases (`/private/tmp/vpt-publication-guard-commands.log`). Seven review cases then exceeded the
  unchanged 500 ms command deadline. All 18 review cases passed on the isolated rerun
  (`/private/tmp/vpt-publication-guard-review-recheck.log`). This is passing case coverage across
  runs, not a clean combined run or a proven explanation of the intermittent timing failures.

- Synthesis 23, transcription 20, and note verification 15 cases passed in
  `/private/tmp/vpt-publication-guard-remaining.log`. All 229 distinct command cases in the nine
  affected targets have passing evidence across the recorded runs.
- Workspace/all-targets/dev-tools Clippy passed with warnings denied
  (`/private/tmp/vpt-publication-guard-clippy.log`). Formatting and diff whitespace checks passed.
- Changed production files remain within 150 lines. No watchdog, fixture deadline, durability sync,
  schema, dependency, or unsafe block changed. No commit, push, or external publication was performed.

All compiler/test processes are terminal. The full goal remains active with 31 checked and 76
unchecked roadmap items. Existing-file replacement and temporary-source last-check races remain
explicitly open; this increment completes no broader roadmap row by itself.

## Occasion identity, persistence, and observational commands

Occasions now have their own identity type and canonical manual, dam, and Google keys. Provider
lookup happens before ID derivation. Changing a provider event's title or start preserves its ID,
saved path, pack, and briefed marker. Manual keys use the complete sanitized title and exact start;
filename truncation does not influence identity. Keys frame their UTF-8 fields by byte length, so
calendar/event boundaries and provider names cannot alias each other.

The start-time parser preserves local dates, offsets, and arbitrary fractional precision within a
65,536-byte input bound. It normalizes case, positive zero offset, and trailing fractional zeros,
retains the unknown-offset spelling, and orders leap-second notation between adjacent seconds.
Its syntax and offset behavior follow [RFC 3339](https://www.rfc-editor.org/rfc/rfc3339#section-5.6).
Provider keys, titles, participant data, and tags are omitted from Debug/history records. Explicit
occasion output includes the requested metadata; provider object keys remain private.

Both ledgers save occasion changes with a distinct `occasion_saved` history fact in the same
transaction. Expected-state checks reject stale changes and altered identities/keys. Saving metadata
cannot fabricate a brief path or completion marker and does not report a completed recording or
emit a notification. Read validation bounds fields and collections; listing has limits of 65,536
rows and 64 MiB of field data. A shared sanitizer continues to serve transcript filenames.

SQLite schema 12 adds the exact RFC 3339 start. Migration and its version change roll back together.
Earlier occasion rows are preserved. Rows without the newly required start are reported as corrupt
when accessed because their current offset and fractional precision cannot be recovered reliably;
no start value is invented. Other ledger data and history stay readable. Fixtures for all prior
migrations were updated to remove the new column when constructing older schemas.

`vpt occasions [--json]` now lists saved occasions, including unbriefed ones, in exact chronological
order. `vpt path --occasion <id> --stage brief [--json]` returns the saved absolute pin, including
when the file is absent. Neither command creates a missing ledger, rewrites files, or starts recovery.
Human list output escapes fetched title controls. Unsupported selectors and missing pins return
usage errors. README and the roadmap describe the new commands and remaining writer work.

This does not implement `brief`, evidence selection/rendering, initial path pinning, pack generation,
context collection, automatic triggering, or brief publication/recovery. Both broader occasion rows
remain unchecked; the full roadmap still has 31 checked and 76 unchecked items.

Validation:

- Red/green tests cover exact timestamps, canonical identities, provider rescheduling, stale
  writes, transaction rollback, manual reproduction, privacy, and invalid/bounded metadata.
- All 1,754 workspace unit tests passed in `/private/tmp/vpt-occasions-units.log` (CLI 76,
  adapters 1,316, application 67, domain 221, protocol 74).
- Added corrupt-row cases exposed noncanonical pinned paths and an unrepresentable briefed timestamp.
  Both were fixed. All 13 occasion ledger cases passed after the final changes in
  `/private/tmp/vpt-occasions-final-focused.log`, including migration rollback, reopen durability,
  preserved pack/pin/marker, and fixed private error messages.
- All 130 command cases passed together in `/private/tmp/vpt-occasions-commands.log`: occasions 3,
  notes 62, pipeline 34, and show/list/storage 31. Snapshots include file bytes, modes, and timestamps.
  The saved-pin fixture uses the shared persistent-WAL helper to retain valid database companions.
- Workspace/all-targets/dev-tools Clippy passed with warnings denied in
  `/private/tmp/vpt-occasions-clippy-final.log` after removing an unnecessary test clone.
- Formatting and diff whitespace checks passed. No dependency or unsafe block was added. Existing
  watchdogs and command deadlines are unchanged; the new CLI cases use five seconds. New production
  files stay below 150 lines. No commit, push, or external publication was performed.

All test/compiler processes are terminal. The full goal remains active.

## Brief evidence selection, quote assembly, and JSON contract

The domain now selects transcript snapshots by confirmed participant, title term, and occasion tag,
then expands recent continuation/shared-tag matches. Participant and title terms require a unique
current indexed note plus a confirmed mentions relation. Unconfirmed, missing, and ambiguous
participant terms remain visible; only unconfirmed terms carry a safely quoted confirmation command.
Conflicting snapshots for the same recording are refused, and exact duplicates count only once.

Recent selection computes the full eligible closure before ranking and applying the note limit.
The lookback uses the generation instant, includes the lower boundary, and excludes future captures.
Continuation chains may bridge old or artifactless recordings without selecting those intermediates.
A cycle without a direct match selects nothing. This uses the expansion assumption stated during
implementation; it was not a user-confirmed decision. The specification now states this rule explicitly.

Quote assembly copies accepted segment text without readability or correction rewriting. Participant
and title selectors quote matching segments; tag/recent selectors quote all nonblank segments.
Deduplication precedes the span cap and combines uncertainty from duplicate occurrences. Identical
text at different times remains distinct. Open flags and original text superseded by a correction
produce unverified quotes. Unreviewed sources and open divergent diagnostics are reflected in the
pack's reviewed state. Each selected source retains its recording, absolute note path, and digest.
Omitted notes/spans and rejected candidates remain counted and explained.

The protocol encoder emits `vpt.brief/1` with exact occasion metadata, participant term state,
certainty and sources for every item, unresolved counts, selection explanations, not-selected data,
independent calendar/task statuses, and generation time. It refuses missing item certainty/sources,
invalid timestamps, forged source paths/digests, duplicate selected entries, and inconsistent counts.
Input fields, aggregate text, graph size, matching operations, and encoded output are bounded.
Debug output omits private quote, term, participant, and context-failure text.

Brief configuration now reaches typed runtime settings: 12 notes, 180 days, five spans, trigger off,
and a 24-hour lead window by default. Configured zero caps and zero lead time are retained. Note and
span caps above 65,536 are refused with the setting name. The trigger itself is not yet implemented.

An existing process test exposed a readiness publication race: the parent observed the marker after
creation but before the participant PID was written. The new empty-marker regression failed first.
Test fixtures now rename a completed marker into place, and readiness requires a nonempty marker.
Production process execution and the existing watchdog/fixture deadlines are unchanged.

Validation:

- Behavioral red/green tests cover direct/recent selection, quote provenance and uncertainty,
  deduplication and limits, invalid inputs, required JSON fields, inconsistent pack counters,
  an open divergent diagnostic, readiness publication, and configured brief settings.
- All 1,776 workspace unit tests passed in `/private/tmp/vpt-brief-workspace-units-final.log`:
  CLI 76, adapters 1,318, application 67, domain 237, protocol 78. This followed the initial
  workspace failure that exposed the readiness race; the final run includes its regression.
- After typed settings were added, all 16 configuration-settings tests passed in
  `/private/tmp/vpt-brief-settings-green.log`, including three new brief tests.
- All 155 command regressions passed in `/private/tmp/vpt-brief-command-regressions.log`:
  confirmation 18, notes 62, occasions 3, pipeline 34, synthesis 23, and verification 15.
- Workspace/all-targets/dev-tools Clippy passed with warnings denied in
  `/private/tmp/vpt-brief-clippy-final.log`. After its two style fixes, all 20 brief domain/protocol
  tests passed again in `/private/tmp/vpt-brief-focused-final.log`.
- Formatting and diff whitespace checks passed. The 23 new production files are at most 134 lines.
  No dependency or unsafe block was added; no watchdog or command deadline was widened.
  No commit, push, or external publication was performed. All test/compiler processes are terminal.

The brief command, current ledger/artifact/index snapshot collection, Markdown rendering, pack
persistence, first-write path pinning, durable publication, context collectors, and automatic trigger
remain unfinished. These supporting libraries do not complete the broader roadmap rows.
The full goal remains active with 31 checked and 76 unchecked items.

## Brief Markdown and owned-document rewrites

Brief document validation now lives in the domain and is shared by Markdown and JSON encoding.
The existing JSON refusal tests still pass after this move. Note links also share one domain renderer,
so briefs use the existing root-relative wiki destinations and encoded relative Markdown fallback.
The application retains its public link-style type through a re-export.

Brief Markdown includes the required occasion/source metadata, selected recording paths and digests,
unresolved counts/reviewed state, independent context statuses, and generation time. All five managed
sections are always present. Empty sections say None. Quotes retain accepted text, with Markdown
escaping, recording identity, precise timecode, source-note link, selector, and inline uncertainty.
Participants, titles, tags, confirmation commands, provider data, and context failure details cannot
introduce raw Markdown structure or managed markers. Filename generation uses the current occasion's
local date, sanitized title, and the stable identity hash; it does not reuse a rescheduled ID's old date.
Obsidian frontmatter uses the local first-write date and the supplied generated hub/reference links.

Typed context task data now supports sourced open-task rendering and an additive JSON tasks array.
Completed tasks are excluded. A document containing tasks requires present task context. Task fields,
count, duplicate source identities, and total document text are validated before rendering. Raw task
content remains separate from accepted transcript quotes. This adds no collector or provider access.

The brief region parser refuses missing, duplicate, reversed, and inline markers. A rewrite preserves
all bytes outside the managed body, including surrounding prose and original outer line endings.
The adapter checks occasion identity, brief kind, and schema before updating. Higher schemas remain
readable and refuse rewrites. Transcript, analysis, and brief formatters now share the frontmatter
update implementation: owned keys update, operator properties and comments survive, and operator tags
are retained while previous managed tags are replaced. Block and flow YAML are covered.

Validation:

- The three initial Markdown behavior tests failed before implementation and then passed.
- Open task rendering/status validation and the additive JSON task field each failed behaviorally
  before being connected and then passed. The first task-test attempt had a fixture borrow error;
  it was corrected before obtaining the behavioral red results.
- Both brief-region tests failed before implementation and then passed.
- All three owned-brief rewrite tests failed against the initial stub. After implementation, all 35
  note-document tests passed together in `/private/tmp/vpt-brief-rewrite-green.log`, including existing
  transcript/analysis YAML preservation cases. Shared-link tests also passed after extraction.
- All 1,790 workspace unit tests passed in `/private/tmp/vpt-brief-render-workspace.log`:
  CLI 76, adapters 1,324, application 67, domain 244, protocol 79.
- All 155 command regressions passed in `/private/tmp/vpt-brief-render-commands.log`:
  confirmation 18, notes 62, occasions 3, pipeline 34, synthesis 23, and verification 15.
- Workspace/all-targets/dev-tools Clippy passed with warnings denied in
  `/private/tmp/vpt-brief-render-clippy-final.log` after removing one unnecessary borrow.
- All 35 note-document tests passed again after that cleanup in
  `/private/tmp/vpt-brief-rewrite-final.log`. Formatting and diff whitespace checks passed.
  New production files are at most 134 lines; changed production files remain within 150 lines.
  No dependency, unsafe block, watchdog, or command deadline was added or changed.
  No commit, push, or external publication was performed. All test/compiler processes are terminal.

The command is not wired yet. Current ledger/artifact/index collection, pack persistence, first-write
pinning, durable publication/recovery, context collectors, and the upcoming trigger remain unfinished.
No broader checklist item is marked complete by these rendering and formatting changes.
The full goal remains active with 31 checked and 76 unchecked items.

## Saved brief packs and publication recovery

Both ledgers now retain an owned brief artifact containing the complete evidence/context snapshot,
exact rendered text, independent content/snapshot digests, managed tags, first-write time/offset,
pinned path, and publication status. The bounded versioned snapshot codec round-trips exact occasion
times, quote provenance and certainty, selection decisions/counters, unresolved state, and tasks.
Malformed, inconsistent, future-schema, or oversized saved data is refused with private errors.

Accepting a brief atomically saves its snapshot, rendered text, path pin, publication intent, and
content-free history. It leaves the occasion unbriefed until publication completes. Rewrites retain
its original path and first-write metadata, require the expected version and occasion, and preserve
the last successful briefed timestamp while a new publication is pending. Selected sources must
refer to ledger-owned transcript paths. Other artifact writers cannot reserve an owned brief target;
generic journal replacement/clearing and standalone brief completion facts are refused.

SQLite schema 13 adds owned brief render state while retaining the pack in the occasion row.
Migration and the version change commit together. Legacy schema-12 packs remain untouched and do not
become fabricated owned artifacts. Read-only access to older schemas remains observational. Saved
packs survive reopening and provider rescheduling with their original content and provenance.

Mutating startup now repairs briefs from their retained bytes. Only after the existing publication
repair verifies durable intended bytes does a transaction clear the journal, record completion, and
mark the occasion briefed. A replacement file is preserved and leaves the saved pack pending. Startup
recovery sends no brief notification. Occasion completion IDs remain distinct from recording IDs and
survive post-commit failure, a stopped event sink, a later publication/command failure, and failed final
output. Startup carries brief completions separately from its recording-based retention report.

Validation:

- Behavioral red/green tests cover saved snapshot round trips, atomic brief acceptance/completion,
  rewrite/version guards, rollback, journal ownership, canonical paths, and completion-history guards
  in the memory and SQLite ledgers. SQLite tests also cover migration rollback, reopening,
  rescheduling, private corruption failures, and observational legacy reads.
- The command recovery test first exposed fixture/module and state-permission issues; after those
  were corrected, it failed because startup had no saved brief renderer, then passed with recovery.
  Replacement preservation and completed-ID behavior are covered separately. The later-command
  fixture was corrected to avoid preflight rejection before startup.
- All 1,810 workspace unit tests pass in `/private/tmp/vpt-brief-persist-workspace-final.log`:
  CLI 77, adapters 1,343, application 67, domain 244, protocol 79. The initial full run exposed
  stale migration table/version expectations and a changed non-UTF-8 error classification; both
  were corrected before the successful full run.
- Workspace/all-targets/dev-tools Clippy passed with warnings denied in
  `/private/tmp/vpt-brief-persist-clippy.log`. Formatting and diff whitespace checks pass.
- The extended command run exposed intermittent 500 ms review-command timeouts. Recovery now
  reads saved engine/note artifacts before attempting brief lookup, avoiding an extra database open
  for those publications. A subsequent run also timed out in read-only review commands, which do not
  use publication recovery; the latest diagnostic review run passed all 18 cases. These observations
  do not establish the full cause of the timeouts. No deadline was changed.
- The final extended command run passed all 237 cases in
  `/private/tmp/vpt-brief-persist-commands-final.log`: confirmation 18, ingestion 20, notes 62,
  occasions/recovery 8, pipeline 34, retention 19, review 18, synthesis 23, transcription 20,
  and verification 15. It includes recovery after rename, preservation of replacement files,
  partial multi-brief recovery, and completion reporting after a later pipeline source failure.
- New brief production files are at most 146 lines. No dependency or unsafe block was added,
  and no watchdog or command deadline was widened. No commit, push, or external publication was
  performed. All test and compiler processes are terminal.

The brief command, current source/index collection, context collectors, and upcoming trigger remain
unfinished. The existing-target and staged-path final-check-to-rename races in shared publication
also remain open. These changes do not close any broader checklist row: 31 checked, 76 unchecked.

## Manual briefs, current evidence, and context documents

Manual and existing-ID `brief` commands now collect current evidence, assemble a pack, save its
rendered bytes and publication intent, and publish through the existing recovery path. Manual
arguments accept duration, repeated participants, and repeated tags without exposing their values in
errors. Repeated requests retain the pinned path, first-write metadata, and operator edits; provider
rescheduling changes the current occasion details while retaining its original file. Unknown IDs
refuse before state creation. Requested briefs emit no automatic notification.

Both ledgers stream brief inputs from one consistent snapshot, with a 65,536-record ceiling and
an aggregate text/item budget before candidate retention. The command combines saved confirmations
and relations with the current term/name index and current owned transcript files. Quotes come from
accepted transcripts, with current file digests and review uncertainty. Missing or expired sources
supply no quotes; altered managed transcript content or ownership refuses privately.

Dry runs and explanations leave stores, state, and history unchanged. JSON returns `vpt.brief/1`;
human explanations include each candidate's selector, matched values, and rejection or limit reason.
Zero note/span caps preserve omission counts and unconfirmed participants retain confirmation guidance.
The portable wiki-root calculation is shared with other note writers.

Context settings now reach runtime as typed values with private debugging. Supported context halves
require explicit selections. A domain `ContextSnapshot` and bounded `vpt.context/1` encoder/decoder
now preserve provider identities, exact occasion/task text, independent statuses, and Google calendar
identity. They reject invalid states, duplicate identities, unsupported major versions, and oversized
documents; additive fields receive diagnostics without copying values. A new failing test exposed
duplicate JSON fields being silently overwritten; the shared bounded reader now refuses those fields.

Validation:

- Behavioral tests were run failing before command parsing, evidence collection, context selection
  validation, explanation, unknown-ID preflight, and context normalization were implemented.
- The brief command increment passed all 1,815 workspace unit tests in
  `/private/tmp/vpt-brief-command-workspace-final.log` and all 258 command tests in
  `/private/tmp/vpt-brief-command-regressions.log`, including 16 brief cases. The command regression
  process is terminal with exit 0; no existing deadline was changed.
- All 86 protocol tests pass in `/private/tmp/vpt-context-protocol-full.log`, including seven new
  context tests covering exact round trips, independent absence, private debugging/errors, limits,
  identity collisions, additive diagnostics, and duplicate JSON fields.
- Workspace/all-targets/dev-tools Clippy passes with warnings denied in
  `/private/tmp/vpt-context-clippy.log`. New context production files are at most 104 lines.
- The initial context workspace run passed CLI 77 and adapter 1,346 cases, but one existing SQLite
  relation boundary test hit its 500 ms child deadline. The same test then passed in isolation in
  0.13 seconds (`/private/tmp/vpt-context-relation-isolated.log`). Application 67, domain 245, and
  protocol 86 subsequently passed in `/private/tmp/vpt-context-remaining-units.log`. No deadline
  changed, and the isolated pass does not establish the timeout's cause.
- A repeat full run (`/private/tmp/vpt-context-workspace-final.log`) passed the relation test but
  hit the same 500 ms deadline in the existing SQLite missing-history transcript rollback test.
  That test also passed alone (`/private/tmp/vpt-context-transcript-isolated.log`). Each full run
  passed 1,346 adapter cases and failed one different case. The full suite is therefore not clean;
  these intermittent timeout causes remain unresolved. All compiler and test processes are terminal.
- Formatting and diff whitespace checks pass.

The roadmap now checks occasion path/rewrite behavior, evidence selection, and explanation/omission
reporting: 34 checked, 73 unchecked. Actual context collectors, their shared deadline, upcoming
discovery and notifications, and the shared publication races remain unfinished. Configured providers
currently report failed context; they do not yet collect external data. No dependency or unsafe code
was added in this increment, and no commit, push, or external publication was performed.

Read-only inspection of the installed dam 0.2.0 CLI and its local source established the next adapter's
wire contract: `ls --json` returns an `objects` array with `oid`, `kind`, `subject`, `path`, and nested
`event`/`task` data. Event attendees carry email, organizer carries optional name, and task fields are
`done`, `priority`, and optional `due`. The query lexer has no quoting mechanism for path predicates,
so calendar/project strings containing query operators or whitespace need rejection before spawning.
No dam store or remote was read or modified during that inspection.

## Dam context collection

Manual and existing-ID briefs now collect dam context through the application `ContextSource` port.
Each direct-argument invocation includes `ls`, an explicit calendar/project path predicate, `--json`,
and `--no-pull`. Missing selections and path strings that could change query syntax refuse before
state creation. Unrequested halves require no selection and perform no queries. The command prefix
remains configurable; no shell interpolation, write command, or remote pull was added.

The collector normalizes dam's actual nested `objects` wire format into `vpt.context/1` before
returning domain values. It preserves provider IDs, organizer names, attendee emails, project paths,
task text/due/priority, and exact event starts. Completed tasks and cancelled events are omitted.
Zoned timestamps and all-day dates use pinned `jiff = 0.2.37`; all-day dates use the event's declared
zone or the system zone. DST transitions affect duration correctly. Jiff's date-to-zone behavior was
checked against its [primary documentation](https://docs.rs/jiff/latest/jiff/civil/struct.Date.html#method.to_zoned).
The dependency and its transitive dependencies were resolved from the local cache with Cargo offline.

Calendar queries include the neighboring UTC boundary days because dam matches each event's local
date. Returned occasions then use the exact inclusive-start/exclusive-end window. Scope checks refuse
unexpected provider paths. Repeated rows from overlapping selections are deduplicated; conflicting
versions fail the half instead of producing mixed evidence.

All queries share one 30-second deadline, including both halves. Calendar and task failure statuses
are independent, and a completed half can survive failure of the other. Missing programs, process
failures, stderr error documents, malformed data, and capacity failures produce fixed private details.
Interruption aborts collection and prevents brief acceptance, including when the interruption arrives
after the deadline. The collector bounds input/output documents, query count, and retained query text;
aggregate overflow still returns a valid bounded failure document.

Brief previews perform read-only collection without changing state, stores, or history. Published
manual briefs save escaped task text and source metadata, and can succeed with failed calendar
context. Google collection and the upcoming trigger remain unfinished.

Validation:

- Initial codec tests failed before decoding existed; initial collection tests failed before scoped
  spawning existed; three CLI tests failed before brief integration. Later failing boundary tests
  exposed aggregate failure-document overflow, excessive query-plan allocation, interruption being
  treated as absence, and deadline masking a late interruption. Each was fixed without widening a
  deadline.
- All 1,837 workspace unit tests passed in `/private/tmp/vpt-dam-workspace.log`: CLI 77,
  adapters 1,362, application 67, domain 245, protocol 86. The earlier intermittent SQLite timeouts
  did not recur in this run; their cause remains unresolved.
- After the final late-interruption fix, all 15 collector tests passed in
  `/private/tmp/vpt-dam-final.log`. They include deterministic clock/runner cases for the shared
  deadline, real fixture subprocesses for direct arguments and stderr, and normalized provider data.
- All 19 brief CLI tests passed in `/private/tmp/vpt-dam-command-final.log`, including byte/metadata
  snapshots proving observational previews, refusal before state creation, failed calendar plus
  successful tasks, escaped managed markers, and saved durable context.
- Workspace/all-targets/dev-tools Clippy passes with warnings denied in
  `/private/tmp/vpt-dam-clippy-final.log`; its initial run found a manual ceiling division in a test,
  which was replaced with `div_ceil`. Formatting and diff whitespace checks pass. All compiler and
  test processes are terminal.

The dam checklist item is now complete: 35 checked, 72 unchecked. No real dam store, calendar, task,
or remote was accessed during implementation tests. Production additions stay below 150 lines per
file; no unsafe block was added and no commit, push, or external publication was performed.

## Native Google Calendar context

Manual briefs and previews now use the native Google Calendar adapter. Configured credentials are
validated before state creation. Token refresh requests only `calendar.events.readonly`; the reply
must explicitly grant that scope alone. All event requests use configured calendar IDs, expanded
recurrences, the requested time window, and bounded partial responses. The collector applies the exact
inclusive-start/exclusive-end predicate because Google's lower bound filters event end times.
Calendar/event identities remain distinct across calendars; cancelled events are omitted. All-day
and zoned events preserve local offsets and daylight-saving duration.

Token refresh and all pages share one 30-second deadline. Each calendar permits at most ten pages;
repeated cursors, conflicting event versions, aggregate overflow, or any later failure discard all
calendar results. Google task context is unsupported. Interruption is checked before and after each
native request, with request timeout bounded by the remaining collection time. Native socket calls
can continue until that timeout; interrupted collection cannot proceed to brief acceptance.

The adapter pins `ureq = 3.4.2` with default features disabled and rustls enabled. HTTPS is mandatory,
redirects and proxies are disabled, server certificate verification stays enabled, and authorization
headers are marked sensitive. Bodies and response headers are bounded. Only fixed token-refresh POST
and event-list GET endpoints are used. Errors contain fixed failure classes, never credentials,
provider bodies, or event text. The dependency graph contains rustls and no OpenSSL/native-tls.

Provider behavior was checked against Google's primary documentation:
[events.list](https://developers.google.com/workspace/calendar/api/v3/reference/events/list),
[event fields](https://developers.google.com/workspace/calendar/api/v3/reference/events), and
[OAuth refresh](https://developers.google.com/identity/protocols/oauth2/web-server).
Transport timeout and body semantics were checked against the installed ureq source and its
[configuration](https://docs.rs/ureq/latest/ureq/config/struct.ConfigBuilder.html) and
[body limit](https://docs.rs/ureq/latest/ureq/struct.BodyWithConfig.html) documentation.

Validation:

- All 32 context adapter tests passed in `/private/tmp/vpt-google-context-final.log`, including 17
  Google cases. Tests use fake replies, deterministic clocks, native request construction, synthetic
  response bodies, and client policy checks. The actual HTTP-scheme refusal assertion also passed in
  the later full unit run. No live Google account, credential, calendar, or task was accessed.
- All 20 brief CLI tests passed in `/private/tmp/vpt-google-command-green.log`, including missing
  credential errors without state creation or secret disclosure.
- The workspace unit run in `/private/tmp/vpt-google-workspace.log` passed CLI 77 and adapters 1,377,
  but two existing SQLite proposal cases reached their unchanged 500 ms child watchdogs:
  `corrupt_saved_proposal_columns_are_refused_without_private_values` and
  `synthesis_findings_follow_rendered_markdown_and_recheck_idempotently`. All 22 proposal tests then
  passed in `/private/tmp/vpt-google-proposals.log`. This is not a clean full-suite result; the
  intermittent watchdog failures remain unresolved.
- The remaining application 67, domain 245, and protocol 86 tests passed in
  `/private/tmp/vpt-google-other-units.log`.
- Workspace/all-targets/dev-tools Clippy passed with warnings denied in
  `/private/tmp/vpt-google-clippy.log`.

The native Google, normalized context/deadline, private read-only collection, and observational brief
preview rows are now checked: 39 complete, 68 remaining. The upcoming trigger and occasion-specific
report events remain unfinished. No unsafe code, commit, push, or external publication was added.

## Upcoming briefs and occasion reports

`brief --upcoming` now uses the opt-in trigger and configured lead time. Disabled triggering names
`brief.trigger.enabled` with exit 2. Discovery happens before opening mutation state; failed or absent
calendar context exits 1 and creates no state. Both provider adapters enforce the exact start window
and refuse incomplete calendar discovery. One collected snapshot supplies the whole invocation,
including task status, without repeating provider queries for every brief.

Under the write lock, provider keys resolve existing occasion identities before preparation. Previously
briefed occasions are skipped; changed provider details are saved while preserving the original ID,
brief path, accepted pack, and completion marker. Each new brief uses current accepted transcript
sources and the same preparation/publication path as requested briefs. The command returns a `written`
array of newly published occasion IDs. Partial failures and broken stdout retain completed IDs.

Dam labels now become occasion tags. The bounded context protocol accepts an additive optional `tags`
array, validates it, and preserves it on encoding; older documents without tags retain empty tags.
The trigger saves these tags and uses them for confirmed-tag evidence selection. New behavioral tests
first demonstrated that the provider and protocol layers dropped labels, then passed after both layers
were connected. Invalid and excessive tag arrays fail without exposing their values.

Automatic publication records one `brief_written` event in the same transaction as the briefed marker,
after durable bytes exist. The event carries a typed occasion identity, a null recording identity,
published path, and counts for source notes, quoted spans, tasks, and unresolved flags. The normal
report channel logs it and invokes the configured report command. Requested briefs emit no report;
repeated trigger invocations and startup recovery do not replay notifications. The saved-history codec
preserves occasion identities and accepts legacy events without that field.

Validation:

- Three initial trigger tests failed against the previous unavailable-context stub. All six trigger
  tests now pass: successful distinct identities/rescheduling, failed discovery, disabled context,
  atomic completion/report rollback with partial success and startup recovery, broken stdout, and
  real fixture report-hook input with evidence selected by a provider label.
- A SQLite trigger forces the second brief's completion transaction to fail. The first completion ID
  and report survive, while the second report and briefed marker roll back together. After removing
  the fixture fault, startup finishes the saved publication without replaying notification hooks.
- All 1,857 workspace unit tests pass in `/private/tmp/vpt-upcoming-workspace.log`: CLI 77,
  adapters 1,381, application 67, domain 245, protocol 87. The earlier SQLite watchdog failures did
  not recur; their cause remains unresolved and the deadlines are unchanged.
- All 26 brief CLI tests pass in `/private/tmp/vpt-upcoming-brief-final.log`.
- The context tag tests passed separately in `/private/tmp/vpt-label-protocol-green.log` and
  `/private/tmp/vpt-label-context-green.log`, and again in the full workspace run. A new history-codec
  test covers occasion/count round trips, old missing fields, and private corruption errors.

Workspace/all-targets/dev-tools Clippy passes with warnings denied in
`/private/tmp/vpt-upcoming-clippy.log`. Formatting and diff whitespace checks pass. All build, test,
and Clippy processes are terminal.

The brief-command, brief document/event, and calendar-trigger rows are now checked: 42 complete,
65 remaining. The shared publication race items remain open. All provider data in these tests was
synthetic; no live Google account, dam store, calendar, task, or remote was accessed. Production modules
added or expanded for this increment remain below 150 lines, and no unsafe block was added. No commit,
push, or external publication was performed.

## Redaction preparation

The pure redaction engine now masks supplied confirmed terms and note names independently of enabled
pattern classes. Matching is Unicode caseless and respects whole tokens and possessives, with no
approximate-name matching. Candidate selection uses longest spans first, then the documented class
precedence, and labels are numbered in source order across blocks of one copy. A fresh redactor starts
at one. Private mappings retain exact removed values; Debug output exposes only class and number.

Email and URL rules follow the specified whitespace-token definitions. Phone runs use the specified
separators and minimum digit count. Number and money recognition uses Unicode decimal-number and
currency-symbol categories from pinned `unicode-general-category = 1.1.0`. Amounts are scanned once,
including long decimal/grouped runs, without repeatedly rescanning their suffixes. The engine bounds
source size, tokens, matches, mappings, output, and template size. Failed application does not advance
mask numbering. Residue checks refuse remaining vocabulary or removed values without copying values
into errors.

The Markdown adapter uses the existing pinned pulldown-cmark parser. It joins visible inline text
before masking, so formatting and character references cannot split a sensitive name. Link/image/wiki
labels remain eligible text while destinations, reference definitions, metadata, comments, and raw
HTML are discarded. Hidden inline HTML contents are omitted as well as their tags. Denied sections
include nested headings and end at the next peer or ancestor heading. Timecodes are removed by
default; retained text is re-escaped so it cannot create links or HTML when rendered again.

Pinned `html-escape = 0.2.15` decodes entities inside code spans and fenced code, where Markdown itself
normally leaves them literal. A failing boundary test exposed that fenced-code gap before the fix.
Both new dependencies were resolved from the local cache with Cargo offline.

Owned-note extraction verifies recording/stage ownership and returns only managed content with its
exact source offset. It excludes original frontmatter, managed links, and operator prose outside the
content region. Release assembly emits only optional title/date and the configured source statement;
an empty source with no other fields emits no frontmatter. The final residue check includes metadata,
so a title or source statement cannot reintroduce private values. Each emitted body line retains its
source byte span; line numbers account for generated frontmatter.

This is preparation code, not a finished redact command. Outstanding work includes live vocabulary
loading, flagged transcript-span and analysis-claim policies, surviving-name candidates, configuration
and CLI dispatch, destination protection, durable private reports, immutable release rows, automatic
pipeline integration, and publication/recovery. No redaction checklist item is marked complete; the
roadmap remains 42 checked and 65 unchecked.

Focused validation:

- All seven initial domain behavioral tests failed against the pass-through stub. The implementation
  now passes eight tests in `/private/tmp/vpt-redact-domain-final.log`, including a long numeric input
  and unchanged numbering after a capacity refusal.
- Five of six initial Markdown tests failed against the pass-through stub; the remaining test already
  used escaped literals. A later fenced-code entity test failed before decoding was connected.
- All three owned-document/frontmatter tests failed against their stubs and then passed. All eleven
  adapter redaction tests pass in `/private/tmp/vpt-redact-document-green.log`.

No release file or private report was written, and no external service was accessed. Production modules
added in this increment remain below 150 lines. No unsafe block, commit, push, or external publication
was added.

Broader validation also passes: all 1,876 workspace unit tests in
`/private/tmp/vpt-redact-workspace.log` (CLI 77, adapters 1,392, application 67, domain 253,
protocol 87), and workspace/all-targets/dev-tools Clippy with warnings denied in
`/private/tmp/vpt-redact-clippy.log`. Formatting and diff whitespace checks pass. No SQLite watchdog
failure recurred in this run; the earlier intermittent failures remain unexplained. All build, test,
and Clippy handles are terminal.

## Redaction flag policies, candidate reports, and share settings

Redaction preparation now binds saved open transcript flags to exact canonical rendered byte spans.
The renderer exposes each original flag occurrence and its accepted text; spans across segments share
one occurrence identity. Redaction accepts either canonical readability mode and accounts for saved
verification annotations. A changed managed transcript refuses before any stale word offset is used.
Omission removes and counts open occurrences, including formatting flags. Marking keeps accepted text,
excludes checker alternatives, and adds a protected `[unverified]` marker. Empty insertions retain only
the warning when marked. Analysis policy instead uses the whole saved claim carrying an open
`built-on-flagged-text` finding, verifies claim text and digest, and ignores transcript word offsets.

Generated warning ranges remain protected through Markdown parsing, timecode removal, and masking.
A vocabulary match that would consume a warning refuses privately. Line source ranges map back through
policy edits and removed verification annotations to the original managed content. No source artifact
is modified by these preparation functions.

Candidate reports now retain every surviving capitalized non-sentence-initial token and its released
line number, including repeated occurrences, Unicode capitals/titlecase letters, and possessives.
Sentence starts follow terminal punctuation, and a new Markdown block starts a fresh sentence.
The scanner uses decoded visible text after removal and masking. Typed generated-mask spans prevent
custom mask labels from appearing as candidates or changing sentence boundaries; identical literal
text remains eligible. Removed HTML, denied sections, and link destinations do not contribute tokens.
Optional title and source values are scanned too; their candidates point to the YAML field line, and
body candidate line numbers shift with generated frontmatter. Candidate counts are bounded across the
whole document, and Debug output omits token values.

All `[share]` and `[share.redact]` settings now populate application settings, with the specified
defaults. Unknown pattern classes, malformed masks, NUL text, and excessive lists/text fail with the
setting name and fixed diagnostic details. Terms and note names remain mandatory masks and cannot be
listed as optional pattern classes. Source statements, section names, and custom masks are excluded
from settings Debug output. Automatic sharing is represented in settings but is not yet connected to
the pipeline.

Focused test evidence:

- The two rendered-span tests failed on the initial stub, then passed with all note-domain tests.
  The protected-marker test also failed before implementation. All 256 domain tests passed in
  `/private/tmp/vpt-redact-domain-flags.log` before candidate reporting was added.
- Three initial flag-policy adapter tests failed before implementation. All 18 adapter redaction tests
  passed in `/private/tmp/vpt-redact-reviewed-boundaries.log`, including cross-segment counts, empty
  insertions, stale source, saved verification annotations/readability, and protected warnings.
- All three initial candidate-domain tests failed, then all 12 redaction-domain tests passed in
  `/private/tmp/vpt-redact-candidates-green.log`. Both adapter candidate tests failed before wiring;
  all 20 adapter redaction tests then passed in `/private/tmp/vpt-redact-candidates-adapter-green.log`.
- All three share-settings tests failed against default-only settings and pass in
  `/private/tmp/vpt-share-settings-green.log`.
- A residue-label test exposed that removed known terms/names lost their mask numbers during final
  checking. Verification now checks recorded removed values first, preserving the original class and
  number without exposing their values. Its initial failure is in
  `/private/tmp/vpt-redact-residue-label-red.log`.

The roadmap remains 42 complete and 65 remaining. Redaction still needs runtime vocabulary/source
loading, command output, destination protection, immutable release persistence, private reports,
publication/recovery, and automatic pipeline integration. No redaction row is checked on the strength
of preparation tests alone. No release, private report, commit, push, or external-service request was
made. Newly added or expanded redaction/note production modules remain within 150 lines, and no unsafe
block was added.

Broader validation passes: all 1,896 workspace unit tests in
`/private/tmp/vpt-redact-flags-workspace.log` (CLI 77, adapters 1,405, application 67, domain 260,
protocol 87). This includes the new cross-block candidate-capacity and residue-label tests.
Workspace/all-targets/dev-tools Clippy passes with warnings denied in
`/private/tmp/vpt-redact-flags-clippy.log` after correcting test-only range/array style findings.
Formatting and diff whitespace checks pass. The earlier intermittent SQLite watchdog failures did not
recur; their cause remains unresolved and deadlines are unchanged. All test and Clippy handles are
terminal.

## Release repository and private report preparation

Both ledgers now implement release reservation and finalization. Reservation allocates a monotonic
creation sequence, derives the private report filename from recording/stage/sequence, and saves the
immutable release metadata plus exact copy/report bytes and two publication-intent history records
in one transaction. Frozen provenance contains capture instant/offset, engine identifiers, open lexical
and total flag counts, diagnostics, and reviewed state. Input validation checks recording identity,
capture provenance, canonical absolute paths, counts, timestamps, payload bounds, and the report's
source/content digest bindings. Reports and copy contents are excluded from release Debug output.

Pending releases expose their exact saved payloads for recovery and are absent from finalized-release
selection. Completion requires the expected content, source, and report digests, validates both saved
publication intents, and atomically records both observed effects, completed work, and the finalization
marker. Pending payloads are discarded only in that transaction. Repeated finalization preserves the
original timestamp and does not repeat history. Release destinations and report paths stay reserved
after completion; ordinary publication-journal writers and clears cannot reuse them.

SQLite schema 14 adds pending payloads, completion markers, report digests, and destination/report-path
uniqueness. Migration preserves earlier release rows and does not invent evidence that their files are
durable. An observational schema-13 open neither migrates nor returns old rows as finalized releases.
The migration rollback test verifies that an interrupted upgrade leaves version 13 and no partial new
tables; a successful retry preserves old rows and continues sequence allocation.

The bounded private-report serializer records whole-source and released-content digests, exact mask
labels/values, omission/mark counts, and every output line. Source spans are translated to original
artifact byte offsets and validated on UTF-8 boundaries. Generated frontmatter/blank lines have an
explicit null source span. Duplicate, invalid, or out-of-range line mappings refuse rather than
produce a misleading report. Neither the report nor its mapping values enter event history. The new
`redact` run kind has typed history encoding, without exposing an unfinished CLI command.

Focused evidence:

- The three initial in-memory repository tests failed against stubs in
  `/private/tmp/vpt-release-memory-red.log`; the corresponding SQLite tests also failed in
  `/private/tmp/vpt-release-sqlite-red.log` before implementation.
- Reservation tests exposed that general publication writers could reuse release paths. The failures
  are in `/private/tmp/vpt-release-reservations-red.log`; both ledgers now refuse those writers.
- Digest-binding tests failed for a report naming a different source digest in
  `/private/tmp/vpt-release-binding-red.log`, then passed with preparation-time binding validation.
- All 15 release repository tests pass in `/private/tmp/vpt-release-durability-green.log`, covering
  exact reopen recovery, payload-insert rollback, completion/history rollback, changed reports,
  malformed provenance, migration rollback, and both preparation/completion acknowledgement loss.
- Both private-report tests failed on an empty-document stub and pass in
  `/private/tmp/vpt-release-report-green.log`.

This increment persists release state; it does not yet publish the two files. The caller contract
requires durable publication and verification of both files before finalization, and runtime code to
provide that evidence remains unfinished. Destination overlap checks, standardized filenames and
duplicate scans, file publication/recovery, live source/vocabulary loading, CLI output, and automatic
pipeline releases remain open. No release file or separate report file was written by the product
command, and no redaction roadmap row is marked complete. The roadmap remains 42 complete / 65 open.
No deadline, unsafe block, external service, commit, or push was added.

Broader validation passes: all 1,914 workspace unit tests in
`/private/tmp/vpt-release-workspace-final.log` (CLI 77, adapters 1,422, application 67, domain 260,
protocol 88), and workspace/all-targets/dev-tools Clippy with warnings denied in
`/private/tmp/vpt-release-clippy.log`. Formatting and diff whitespace checks pass. All test and Clippy
handles are terminal. Every new production module in this increment is within 150 lines.

The first full run found 13 migration-fixture failures: older-schema fixtures left schema-14 release
objects behind, and the generic schema test still expected 22 tables instead of 24. The fixtures now
remove the release extension when reconstructing an older database, and their version/table assertions
match the current schema. All 17 focused migration tests pass in
`/private/tmp/vpt-release-migrations-green.log`, followed by the clean full run above. No SQLite watchdog
timeout occurred in these runs; earlier intermittent watchdog failures remain unexplained, and their
deadlines are unchanged.

## Release destinations, file publication, and startup recovery

Release filesystem preparation now resolves the selected destination against private stores, state,
configuration, and the Voice Memos container. Refusals name the protected root. Per-command destinations
follow section 8.5's inside-root rule, so an ancestor containing a private store remains permitted.
Missing destination directories are created through retained directory handles, with parent directory
synchronization. Existing destination and drafts directory permissions remain unchanged.

Selection hashes every regular file directly in the destination, including unrelated extensions,
and refuses identical bytes with the existing path. Nested files and symlinks are not followed.
The bounded scan admits at most 65,536 entries. Standard filenames use capture date, stage, and hash8;
collisions use the local write timestamp and then the smallest free numeric suffix. Occupied directory
or symlink names also force a suffix. Selection and publication require the caller's VPT write lock.

Publication consumes the exact saved copy/report payload and validates both digests and destination
bindings before writing. It checks both existing targets before creating either missing file,
publishes exclusively, and preserves matching existing files during recovery. Report files are
mode 0600; recovery requires the effective owner, one link, and exact bytes. Existing directory
permissions are not normalized or required to be 0700: the report file carries the specified private
mode. Both files and parent directories are synchronized, then bytes, identities, and report privacy
are checked before the runtime requests ledger finalization. Changed files and replaced retained roots
remain untouched, with pending state preserved.

Mutating startup now repairs pending releases, including saved destinations outside the configured
released store. The standalone retention command also performs this recovery under its write lock.
Recovery uses saved payloads and provenance, emits no notification event, and commits completion only
after both files verify. Completed recording IDs survive later release failures and a sink stop after
completion. Repeated recovery creates no new history once all releases are finalized.

TDD evidence:

- Destination, duplicate, and collision tests failed against stubs in
  `/private/tmp/vpt-release-files-red.log` before implementation.
- File publication/recovery tests failed against the publication stub in
  `/private/tmp/vpt-release-publication-red.log`.
- Runtime recovery tests failed against the empty repair stub in
  `/private/tmp/vpt-release-recovery-red.log`; the startup test then failed until wiring was added,
  recorded in `/private/tmp/vpt-release-recovery-startup-red.log`.
- The first implementation incorrectly required the existing drafts directory to be 0700.
  The specification requires mode 0600 for report files. Corrected behavioral tests failed in
  `/private/tmp/vpt-release-directory-permissions-red.log` before removing the extra directory-mode
  restriction. Tests continue to refuse nonprivate/hardlinked report files and replaced roots.

The redaction command, live source/vocabulary loading, user-facing candidate/collision output, and
independent automatic pipeline releases remain unfinished. Shared publication races between the last
staging-name check and rename remain tracked in the roadmap. No complete redaction capability is
checked off; the roadmap remains 42 complete and 65 open. No unsafe block or deadline was added,
and no commit, push, or external service operation was performed.

Final validation passes:

- All 11 release filesystem tests: `/private/tmp/vpt-release-files-final-focused.log`.
- All 1,929 workspace unit tests: `/private/tmp/vpt-release-files-workspace-final.log`
  (CLI 81, adapters 1,433, application 67, domain 260, protocol 88).
- All 65 affected command tests: `/private/tmp/vpt-release-files-commands-final.log`
  (brief 26, ingestion 20, retention 19).
- Workspace/all-targets/dev-tools Clippy with warnings denied:
  `/private/tmp/vpt-release-files-clippy.log`.
- Formatting and diff whitespace checks pass. Every new production module is within 150 lines.
  All compile/test/Clippy handles are terminal; no watchdog deadline was changed.

## On-demand redaction command

`vpt redact <id> [--stage transcript|analysis] [--to <directory>] [--title <title>]`
now has parser, dispatch, runtime, and human/JSON output. Transcript is the default; audio, briefs,
dry runs, unknown options, and malformed inputs are rejected. Titles use the existing private
argument wrapper and are absent from argument diagnostics. Missing source stages are input errors;
source I/O failures remain operational errors and changed ownership/content is a named refusal.

The command acquires the write lock, repairs prior work, reads current owned source bytes and accepted
review state, and masks live confirmed terms plus indexed names and aliases. Incomplete metadata in the
index refuses before publication. The source's managed content must match saved accepted content;
operator prose outside managed regions is excluded and preserved unchanged. Settings control minimal
frontmatter, timecodes, denied sections, pattern classes, masks, and open-span policy.

Transcript redaction uses accepted word spans and current lexical resolutions. Analysis redaction
rechecks claims against the current accepted transcript, preserving matching prior finding decisions.
This check is observational: it neither rewrites the source nor changes its saved findings. It prevents
stale built-on-flagged-text findings from omitting a claim after its lexical source flag was resolved.
The release freezes the resulting source provenance, digests, counts, and diagnostics.

Before reservation, the command completes masking, residue checks (including optional header fields),
private report preparation, duplicate scanning, and collision-name selection. It rechecks source bytes,
saves exact copy/report payloads and intents, publishes both files, and commits completion only after
both verify. A failed completion leaves recoverable payloads; the next mutating command completes them
without rerendering. No requested-redaction notification event is emitted. Human/JSON output contains
paths, aggregate mask counts, candidate tokens/line numbers, omitted/marked counts, collision status,
and sequence, with no private mask mappings. Broken stdout preserves completed recording IDs.

TDD and focused evidence:

- The first command tests failed with unknown `redact` in
  `/private/tmp/vpt-redact-command-red.log`, before parser/dispatch/runtime implementation.
- The initial mask assertion expected literal/backslash brackets. VPT's Markdown renderer uses
  numeric character references; the assertion now checks that established escaped representation.
- The stale-analysis test failed by omitting one resolved claim in
  `/private/tmp/vpt-redact-analysis-red.log`; runtime analysis rechecking fixes it.
- Applying that grounding check to transcripts introduced an extra verification finding. Transcript
  redaction now uses its existing accepted lexical state, while analysis performs claim rechecking.
- Source error tests failed with refusal status for unreadable/missing input in
  `/private/tmp/vpt-redact-source-errors-red.log`. Typed file errors now retain their required exits.
- All 12 command tests pass in `/private/tmp/vpt-redact-command-final.log`: current terms/names,
  both source stages, omission/marking, stale analysis, frozen provenance after resolution, minimal
  title/date output, duplicates, collisions, protected destinations, residue, unchanged source,
  failed finalization/startup recovery, broken stdout, input validation, and source error classes.

Automatic pipeline releases and unchanged-run suppression remain open. The shared publication races
already recorded in the roadmap also remain open. This increment adds no unsafe block, changes no
watchdog/command deadline, and performs no commit, push, or external service operation.


## Automatic redaction in the pipeline

The pipeline now shares the on-demand release writer under its existing write lock. It releases each
transcript after publication and verification, before synthesis, and releases analysis after that
artifact is published and checked. This works with synthesis disabled, and enabling sharing processes
existing notes. Expired source stages remain skipped. The writer recognizes finalized matching content
for the same recording, source stage, and destination, so later unchanged runs and recovered releases
create no extra sequence. Explicit redaction retains the duplicate refusal.

Run output now includes each release, its report path, mask/policy counts, candidates, collision status,
and sequence. Human output includes candidate tokens and line numbers. Dry-run planning uses current
source bytes and settings without creating directories, reserving a sequence, publishing, or changing
history. Completed releases retain their recording IDs if a later stage fails.

TDD evidence: all four initial automatic-sharing tests failed with empty release lists in
`/private/tmp/vpt-automatic-redaction-red.log`, then passed in
`/private/tmp/vpt-automatic-redaction-green.log`. The additional dry-run test failed with a missing
`redact_transcript` plan entry in `/private/tmp/vpt-automatic-redaction-plan-red.log`; the planner now
reports that work. Recovery coverage also verifies that a failed finalization is completed on the next
run without rewriting the copy or allocating another sequence.

### Review timing investigation

The broader redaction checks exposed a 500 ms review-command timeout. Temporary tracing placed delays
in different database/history phases across attempts. A macOS sample of the uninstrumented child showed
ongoing publication, log export, SQLite commits, and filesystem synchronization. Running outside the
filesystem sandbox also exceeded the unchanged deadline, so sandbox overhead alone is not an established
cause. An attempted completed-history fixture passed the repeated-confirmation test but did not resolve
other timing failures and changed a log-failure fixture's preconditions; that experiment was reverted.
All temporary tracing and fixture changes were removed. No deadline or durability setting was changed.
The timing failure remains unresolved; these results do not establish a clean full-workspace gate.
The separate synthesis integration suite passed all 23 tests in
`/private/tmp/vpt-redact-synthesis-gate.log`.


Final command verification for this increment: all 40 pipeline tests and all 12 redaction command tests
pass in `/private/tmp/vpt-automatic-redaction-commands.log`, including all six automatic-sharing tests.
The production modules added for pipeline sharing, observational planning, unchanged-copy selection,
and the extracted writer are each under 150 lines.

Workspace Clippy with all targets and `dev-tools` passes with warnings denied in
`/private/tmp/vpt-automatic-redaction-clippy.log`. Formatting and `git diff --check` also pass.
The eight redaction/release checklist rows are now checked: 50 complete and 57 still open out of 107.
This is feature-checklist progress, not a claim that the full product or its remaining acceptance gates
are complete. Text handoff, retention for later stores, shared publication race fixes, and other open
rows remain in scope.

The 67 application unit tests also pass in `/private/tmp/vpt-automatic-redaction-application.log`.


## Read-only text handoff

Implemented parser, dispatch, runtime settings, and `vpt handoff <id> --stage
released|transcript|analysis|brief`. The domain kind is a four-variant enum. The formatter emits a
fixed declarative header containing VPT, the identity, unresolved count, review state, and derived-copy
statement. The artifact body follows byte for byte after frontmatter removal. JSON uses
`vpt.handoff/1`; human output is the header and body. Both remain deterministic apart from the JSON
generation time. Artifact digests identify the bytes read before frontmatter removal/header addition.

Released selection uses finalized sequences in descending order, skips absent copies, verifies the
selected digest, and uses only the release's frozen provenance. A changed newest existing copy is
refused rather than falling back. No existing copy is an input error; a pending release alone is an
operational error without recovery or publication. Private stages require the existing default-false
`handoff.allow_private` setting, now connected to runtime Settings. They validate pinned ownership and
read current bytes while preserving operator prose outside managed regions. Briefs resolve an occasion,
its saved source recording IDs, and source engine metadata. Edited or stale private artifacts cannot
be labelled reviewed merely because saved review counts are zero.

The command opens no mutating ledger, takes no write lock, runs no startup repair, and invokes no
engine, agent, hook, provider, or network client. It emits no completed-work claims, including on broken
stdout. Body input is bounded at 16 MiB, serialized JSON at 32 MiB, and retained engine metadata at
1 MiB; brief source identity collections are bounded too. The shared bounded encoder preserves the
existing brief output limit.

TDD evidence:

- `/private/tmp/vpt-handoff-command-red.log`: three behavior tests failed because `handoff` was unknown.
- `/private/tmp/vpt-handoff-command-green.log`: the initial four command tests passed, covering private
  opt-in, exact body and digest, frozen release provenance after review, newest/missing/changed release
  selection, and observational argument failures.
- `/private/tmp/vpt-handoff-boundaries-red.log`: invalid UTF-8 in a changed release returned a store
  error instead of `artifact_changed`. The bounded retained-file reader now checks expected bytes
  before decoding text, retaining the typed changed-artifact refusal.
- `/private/tmp/vpt-handoff-boundaries-green.log`: all eight command tests passed, including brief
  identity/source resolution, preserved operator prose, pending release behavior, human text, and
  broken stdout without completed work.
- `/private/tmp/vpt-handoff-protocol-red.log`: contradictory metadata (`reviewed` with open flags)
  was accepted. Domain validation now rejects that contradiction.
- `/private/tmp/vpt-handoff-protocol-green.log`: all 91 protocol tests pass, including body-byte
  preservation, stable metadata/title/content, identity-kind consistency, and bounds on raw text and
  JSON escape expansion.

Final handoff verification:

- `/private/tmp/vpt-handoff-related-commands.log`: all 26 brief, 10 handoff, and 12 redaction command
  tests passed together after the shared reader and encoder changes.
- `/private/tmp/vpt-handoff-reader-tests.log`: all eight selected adapter tests passed, including
  raw digest checks before UTF-8 decoding and rejection of oversized changed releases.
- `/private/tmp/vpt-handoff-availability-red.log`: the added behavior test caught removed releases
  being advertised as available. Availability now checks existing regular files, and missing private
  files use the same stage listing. All 11 handoff tests pass in
  `/private/tmp/vpt-handoff-availability-green.log`.
- `/private/tmp/vpt-handoff-clippy.log`: workspace Clippy, all targets and `dev-tools`, passed with
  warnings denied. Formatting and `git diff --check` passed. New handoff production modules are
  each below 150 lines.

The four handoff checklist rows are checked: 54 complete and 53 still open out of 107. This does not
resolve the separately recorded review-command timing failure or establish full-roadmap completion.

## Opt-in pipeline retention

`vpt run` now invokes the existing journaled retention sweep after its processing stages when enabled.
Audio still requires its independent opt-in and a nonzero hold. The pipeline result combines moves
recovered at startup with new moves and retains completed recording IDs if a later move, history
commit, or output write fails. Dry runs use the read-only ledger and store handles, report current
retention candidates, and invoke no helper or hook. The pipeline uses the standalone retention
renderer for moved/kept entries, per-store counts, human lines, and diagnostics.

`/private/tmp/vpt-pipeline-retention-red.log` recorded two behavioral failures: no retention plan in
the dry-run result, and successful pipeline exit despite an enabled expired target with no helper.
The initial implementation passed opt-in and observational-plan coverage. The recovery fixture's
expected move count was corrected from one to two: its newly ingested archive retains the old source
mtime and is also eligible, so recovery must combine the pending old move with that new move.

This initial pipeline increment left later-store ownership and expiration open. The completed
later-store extension is described below; full-pipeline acceptance remains open.

Verification:

- `/private/tmp/vpt-pipeline-retention-suite.log`: 43 of 44 pipeline tests passed, including all four
  retention tests, prior publication/recovery checks, and automatic sharing. The existing four-record
  synthesis-alert aggregation test exceeded its unchanged 8,000 ms command deadline.
- `/private/tmp/vpt-pipeline-retention-final.log`: all four retention tests passed after adding human
  dry-run assertions. They cover both opt-ins, byte-preserving observational plans, terminal audio
  expiration, committed processing on helper refusal, combined recovered/new moves, and actual versus
  planned completed IDs on broken stdout.
- `/private/tmp/vpt-pipeline-retention-alert-rerun.log`: the isolated synthesis-alert test again
  exceeded 8,000 ms. No deadline was widened, and no cause has been established. The separate earlier
  500 ms review timing failure also remains unresolved; there is no clean full-workspace test claim.
- `/private/tmp/vpt-pipeline-retention-clippy.log`: workspace Clippy passed for all targets with
  `dev-tools` and warnings denied. Formatting and `git diff --check` passed. The added pipeline
  retention modules are below 150 lines.


## Retention across later stores

The shared retention service now moves and recovers engine outputs, transcript/analysis notes, briefs,
private redaction reports, and releases. Notes/briefs use their ownership markers; other artifacts use
saved digests. The pipeline and startup reports retain occasion identities for brief completions.
The ledger and later-store retention rows are complete, leaving 51 roadmap tasks unchecked.

The detailed implementation and TDD evidence are in `reports/vpt-retention-progress.md`. All four
existing pipeline-retention tests pass in `/private/tmp/vpt-later-retention-pipeline.log`. The three
new brief CLI tests pass in `/private/tmp/vpt-later-retention-brief-final.log`, including a pipeline
case that previews a brief move without writes, reports the actual move by occasion ID, and preserves
expiration on the next run. The full pipeline row remains unchecked while its remaining acceptance
work and previously recorded timing failure are unresolved. No deadlines were changed.


## Pipeline and terminal-expiration acceptance

Both roadmap rows are now complete, leaving 49 tasks. All 49 pipeline tests passed in
`/private/tmp/vpt-terminal-full-pipeline.log`, including the previously failing synthesis-alert timing
case with its unchanged deadline. New acceptance tests cover real later-store expiration, deliberate
note regeneration from saved inputs after audio/engine-output retention, pending-intent recovery and
replacement refusal, immutable expired releases, and expiration through synthesis configuration changes.
Automatic brief discovery also keeps expired briefs absent after a calendar reschedule.

A new end-to-end event test found that all retention moves were mislabeled as audio. The producer now
persists per-store counts and both log/hook encoders use them. Legacy history remains readable.
Detailed TDD evidence is in `reports/vpt-retention-progress.md`. The broader later-stage logging row
remains open, and this does not establish a passing full-workspace test run.

After the per-store count fix, `/private/tmp/vpt-terminal-command-suite.log` passed all 30 brief,
50 pipeline, and 19 retention command tests. The command deadlines remain unchanged.

Final workspace Clippy passed with all targets, `dev-tools`, and warnings denied in
`/private/tmp/vpt-terminal-retention-clippy.log`. Formatting and `git diff --check` passed.


## Later-stage events and combined retention

The later-stage logging/hooks row is complete, leaving 48 roadmap tasks. Pipeline recovery and new
retention moves now produce one final event with accurate per-store counts. Each move still commits its intent and result
and invokes the history exporter before further processing. Partial recovery refusal, later source
failure, and refused final-summary history are covered without duplicate hooks or regenerated files.

`/private/tmp/vpt-combined-retention-command-suite.log` passes all 123 command tests: 30 brief,
54 pipeline, 19 retention, and 20 transcription tests. These include default logged failures,
uncertainty alerts, automatic brief reports, independent channels, and content-free hook diagnostics.
Detailed TDD and failure-boundary evidence is in `reports/vpt-retention-progress.md`.

The 31 retention integration and 67 application tests also pass. Workspace Clippy passed with all
targets, `dev-tools`, and warnings denied in `/private/tmp/vpt-combined-retention-clippy.log`.
Formatting and `git diff --check` passed; no deadlines were changed.


## Analysis profiles and synthesis freshness acceptance

The analysis-rendering and automatic-synthesis-freshness roadmap rows are complete. The roadmap now
has 61 checked and 46 unchecked tasks. This increment adds end-to-end acceptance coverage; it changes
no production behavior or command deadline.

The new analysis-profile test runs portable and Obsidian profiles with both Markdown and wiki links.
It checks the exact Summary/Actions content, citation order, escaped headings and managed-marker
payloads, open-finding count, ownership, profile key order, quoted links, and local first-write date.
It preserves operator tags, arbitrary fields, prose, status, start date, and description through a
rewrite, then deletes the analysis file and regenerates the exact saved document with the agent set
to fail if invoked. Existing link tests cover matching filenames, common-parent/vault roots, encoded
Markdown fallback, and renamed targets; synthesis metadata tests cover tag/relation proposal gates.

The new pipeline test confirms the particular lexical flag inherited by a saved proposal. The review
operation invalidates its derived findings and makes synthesis pending. The next automatic run updates
the proposal and inherited uncertainty once, with the accepted transcript unchanged and no engine call.
An observational plan writes nothing. Subsequent operator prose and unchanged runs do not invoke the
agent. Existing acceptance tests cover raw transcript changes, readability changes, preserved review
decisions, empty verification results, grounding-window changes, configuration transitions, and
expired analysis.

Requirement evidence:

| Requirement | Passing evidence |
| --- | --- |
| Fixed analysis sections, escaping, citations, and inherited findings | Domain note tests and synthesis command tests, including the new four-combination profile test |
| Profile frontmatter and operator preservation | Domain document tests, adapter analysis rewrite test, and CLI rewrite/regeneration assertions |
| Reciprocal links and existing metadata rules | Synthesis `links` and `metadata` command tests plus ten analysis-note command tests |
| Current rendered input drives automatic synthesis | Pipeline freshness, eligibility, and new lexical-confirmation tests inspect input digests and engine/agent call counts |
| Transcript/lexical changes invalidate findings while preserving decisions | Synthesis freshness commands and verification contracts for both memory and SQLite ledgers |
| Empty results, disabled stages, and expired outputs remain correct | Verification window/claim contracts and pipeline grounding-window, later-recovery, and expiration tests |

Verification logs:

- `/private/tmp/vpt-analysis-profile-acceptance.log`: the new profile/escaping/regeneration test passes
  all four profile/link-style combinations.
- `/private/tmp/vpt-lexical-synthesis-acceptance.log`: the new automatic lexical-freshness test passes.
- `/private/tmp/vpt-synthesis-acceptance-command-suite.log`: all 55 pipeline and 24 synthesis tests pass.
- `/private/tmp/vpt-analysis-domain-acceptance.log`: all 41 domain note tests pass.
- `/private/tmp/vpt-synthesis-verification-serial.log`: all 63 adapter verification tests pass with one
  test thread and the original 500 ms child deadlines.
- `/private/tmp/vpt-analysis-document-acceptance.log`: the adapter analysis rewrite test passes for
  both profiles, including operator fields, tags, and prose.
- `/private/tmp/vpt-analysis-note-command-acceptance.log`: all ten analysis-note command tests pass.

The initial default-parallel verification run passed 27 tests and timed out in 36 SQLite checks at the
existing 500 ms child deadline (`/private/tmp/vpt-synthesis-verification-acceptance.log`). The serial
run passed all 63 without changing a deadline or harness. This does not establish the cause of the
parallel timing failures. The earlier review-command timing issue remains unverified, and no
full-workspace test pass is claimed.

Workspace Clippy passed with all targets, `dev-tools`, and warnings denied in
`/private/tmp/vpt-synthesis-acceptance-clippy.log`. Formatting and `git diff --check` passed.

## Brief rendering and saved-pack acceptance

The remaining brief-rendering checklist item is complete. This update adds acceptance tests and
records their connection to design section 8.3 and the existing persistence/publication behavior.

| Requirement | Evidence |
| --- | --- |
| Persist source identities, note paths, digests, and complete assembled pack | New `brief/rendering.rs` command acceptance reopens the ledger for every observation and compares exact accepted quote text, source identity/digest, certainty, and unresolved counts. The saved-brief codec and both ledger contracts also pass. |
| Required occasion frontmatter and all five managed sections | Four portable/Obsidian and Markdown/wiki combinations verify ownership, required fields, source digests, heading order, exactly one section per heading, and per-line recording/timecode/link provenance. The empty-pack command case checks explicit empty sections. |
| Verbatim accepted spans and inline uncertainty | The fixture preserves double spaces in a confirmed quote and supplies a separately flagged quote. JSON and saved snapshots keep both texts exactly; only the flagged Markdown line carries `[unverified]`. |
| Context, sourced open tasks, and untrusted text | Existing domain and CLI context tests pass for task provenance, absent/failed context, completed-task exclusion, and escaped fetched text/managed markers/section injection. Rich saved-pack codec round trips retain these fields. |
| Profile metadata and operator edits survive rewrite | New rewrite acceptance covers all four profile/link combinations, preserved path/first-write instant/offset, manual tags, operator fields/prose, and Obsidian status, startDate, and description. No createdDate/createdTime fields are generated. |
| Accepted pack and journal survive failed publication acknowledgement | New recovery acceptance interrupts the first populated brief at journal deletion, checks that the pack exists while publication/briefed markers remain unset, and restores the exact text/snapshot/version after the file and transcript source are removed. A note-write startup performs recovery without calling the configured context collector. |
| Preserve a foreign replacement | The same recovery case inserts operator replacement bytes and confirms refusal, unchanged bytes, retained pending journal, unset briefed marker, and no source regeneration. Existing formatter tests cover unsupported schemas, foreign identities, duplicate keys, and malformed markers. |

Validation, with build/test/Clippy commands run sequentially and deadlines unchanged:

- Full brief CLI suite: **34 passed**, `/private/tmp/vpt-brief-rendering-command-suite.log`.
- Domain brief selection/quotes/rendering/document tests: **24 passed**,
  `/private/tmp/vpt-brief-rendering-domain.log`.
- Adapter brief tests, including both ledger backends, migration/reopen/rollback/corruption checks,
  saved-pack codec, and brief formatter: **35 passed**, `/private/tmp/vpt-brief-rendering-adapters.log`.
- New focused acceptance runs: `/private/tmp/vpt-brief-rendering-acceptance.log` and
  `/private/tmp/vpt-brief-rendering-durability.log`.

The generic publication race conditions and the previously reported 500 ms observation-command timing
failures remain open under their existing checklist items. No production behavior, deadline, schema,
or unsafe code changed in this acceptance update. The overall goal remains active.

Workspace Clippy across all targets with `dev-tools` and warnings denied passed:
`/private/tmp/vpt-brief-rendering-clippy.log`. Formatting and `git diff --check` passed.
The roadmap now has **62 checked and 45 unchecked items**.
