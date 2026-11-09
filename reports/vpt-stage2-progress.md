# Transcription implementation progress

## Bounded engine documents

`vpt_protocol::engine::decode` now reads `vpt.engine/1` and validates the document against the configured engine name, family, locality, and recording duration.

It preserves optional confidence, checks finite and ordered timing ranges, rejects empty transcripts and malformed metadata, names unsupported major versions, and returns named additive-field diagnostics without copying field values into them. Explicit confidence must be numeric and within [0, 1].

The reader enforces a 16 MiB byte limit, depth eight, 65,536 characters per text value, 65,536 entries for the exact segment/word array locations, and 256 for other arrays. It refuses an excess array element before decoding it. General incoming documents retain their existing limits.

TDD evidence: valid-document decoding, metadata/range/version rejection, additive diagnostics, engine-specific byte/array limits, and explicit-null confidence were each observed failing before implementation. All 47 protocol tests and protocol Clippy pass after implementation. Production modules are below 150 lines; tests are separate.

## Engine settings and orchestration

Typed settings now carry engine definitions, per-language pairs, checker policy, locality, family, and per-engine deadlines. Selection preserves the requested regional tag and refuses same-family pairs before execution unless explicitly allowed. Debug formatting omits configured argv.

The application engine port and transcript model now support sequential main/checker execution. A successful main remains authoritative; a successful checker can replace a failed main for this attempt. Single-engine results, typed failures, and interruption are explicit. Generic outputs remain available as accepted text and raw documents but cannot drive lexical comparison or classification.

The generic command adapter uses the existing owned-process runner, direct argv, single-pass placeholder substitution, and bounded document decoding. Apple adapter tests cover helper version checks, exact audio/locale arguments, configured aliases, safe errors, and confidence. Selection, command execution, sequential fallback, helper argument parsing, and speech-document mapping had observed failing tests before implementation.

## Native speech helper

The Swift helper now implements `transcribe <audio-path> --locale <tag>` through `SpeechAnalyzer` and `SpeechTranscriber`. It checks OS/device/locale/assets, reads the audio through `AVAudioFile`, requests time/confidence attributes, consumes final results, validates the document, and cancels analysis on failure. It does not install speech assets or use the legacy recognition API.

The permission distinction is verified in [Apple's documentation](https://developer.apple.com/documentation/speech/asking-permission-to-use-speech-recognition). API signatures were also checked against the installed Speech framework SDK. Tests inject speech results and failures and preserve the existing Trash and version behavior.

A native smoke test used generated speech saying "The quick brown fox jumps over the lazy dog" and no personal recordings. Outside the sandbox, the helper returned `Apple speech assets are not installed for the requested locale`. Successful real-framework transcription remains unverified on this host. The roadmap helper item remains unchecked pending that evidence.

## Exact duration

Container inspection preserves original duration ticks and timescale alongside existing whole-second metadata. `precise_duration_secs()` is available for engine timestamp validation, including the final fractional second. Version-zero and version-one fractional-duration tests failed before the change and pass now. Container parsing was split into two production modules below 150 lines.

## Verification

- 43 application and 47 protocol unit tests pass.
- All 816 adapter unit tests pass, including the 13 engine/settings tests with actual fixture processes, deadline failure, and oversized output.
- All 25 container tests pass after the duration refactor.
- Swift's full suite passes: 20 XCTest cases and 10 Swift Testing cases.
- Swift formatting and strict lint pass.
- Swift release build passes. The sandbox denied debug-symbol generation; the approved build outside the sandbox succeeded.
- Full-workspace Rust Clippy passes across all targets with `dev-tools` after the final container refactor. Formatting checks pass for the new engine/transcription modules, and `git diff --check` reports no whitespace errors.

The earlier integration failures were rechecked. The subsequent full Rust workspace run passed all 1374 tests, including ingestion partial-result and deadline checks (`/private/tmp/vpt-whisply-full-workspace.log`). That run predates the accepted-transcript persistence work below.

## Remaining integration

Connecting transcription execution to persistence and the CLI, startup availability checks, uncertainty/review integration, notes, and the later roadmap stages remain unfinished. The full roadmap remains the acceptance authority. The adapter implementations do not establish completion of the user-facing transcription command.

Inspection of the installed Whisply 0.14.2 source found that it writes `<input-stem>_converted.wav` beside the input, and exports `<out-dir>/<input-stem>/<input-stem>.json`. Its JSON uses `transcription.<base-language>.chunks`, with each chunk's `timestamp`, `text`, and `words`; words have `word`, `start`, `end`, and optional CPU `score`. MLX scores are absent. Its language key echoes the provided option. No real Whisply model was run during this work.

## Whisply adapter and temporary files

The Whisply adapter now makes a private byte-preserving working copy, invokes the configured argv with the specified run options, and reads its generated JSON through retained directory descriptors. It rejects redirected or multiply linked output, caps output before reading, and normalizes it to `vpt.engine/1`. Missing confidence remains absent during serialization. Tool version is explicitly `unreported`; no version is fabricated from the configured model.

The backend reader enforces the 16 MiB input limit, depth/text limits, and 65,536-entry limits at the actual chunks/words locations. Other arrays retain the smaller limit. It validates timing, confidence, device resolution, selected language, and text. Additive diagnostics contain field names without their values. Device normalization follows the installed backend's `auto` and `gpu` behavior; GPU-to-CPU output is reported without guessing the cause.

Language mapping preserves the requested regional tag in accepted values and passes the supported base code to Whisply. `LanguageSource` distinguishes requested/echoed language from an engine report. The domain mismatch check does not infer regional disagreement from a base-only report.

Temporary directories are created exclusively with mode 0700, and the audio copy is mode 0600. Cleanup uses retained descriptors, removes temporary links without following them, has depth/entry budgets, and refuses a replaced directory. Explicit cleanup failure names the retained workspace. Crash recovery for abandoned engine working directories still needs integration with the transcription journal.

Fixture processes verify archive preservation even when a backend deletes its working input, sidecar cleanup, malformed and oversized output, timeouts, input/output links, replacement refusal, and permissions. `EngineAdapters` selects Apple, Whisply, or generic command execution; external engines run without probing an absent Apple helper.

The full Rust workspace suite subsequently passed, including the malformed Whisply output and revised cleanup-depth tests. Real-model Whisply transcription remains unverified.


## Accepted transcript persistence

`TranscriptLedger` reads the accepted transcript, requested language, reported/echoed language provenance, exact duration, engine metadata, acceptance time, lexical-classification eligibility, and main/checker provenance. SQLite uses the existing `transcripts` table with a versioned private body; no database schema change is needed for this step. Domain values remain independent of serialization.

`WorkDelta` can accept a transcript or mark a failed transcription. Acceptance saves the text/timings and successful stage together with matching history. Failure marks the stage failed while preserving the prior accepted state. Unrelated, contradictory, or missing history is refused. Both repositories roll back the state change when appending history fails. Completed receipts include accepted recordings without duplicating an identity already created in the same transaction.

The storage codec limits text, arrays, serialized bytes, and decoded body size, and validates saved documents against the same engine contract. Corrupt bodies return a fixed diagnostic without their text. SQLite bounds body and engine-column reads before moving their contents into Rust. Accepted state survives reopening and is available through an observational connection.

The shared contract tests were observed failing before acceptance was implemented. Separate failing tests exposed unrelated-history acceptance and duplicate completed identities; both were corrected. Tests cover replacement by later success, preservation after failure, unknown recordings, rollback, privacy, and reopening. Oversized memory repository and history modules were split by responsibility, with tests in separate files.

Diagnostic/flag state and transcription command orchestration still need integration. Raw-output storage and startup repair were added in the following step. Roadmap transcription items remain unchecked.

Verification for this step: the workspace unit suite passed 1,102 tests before the final receipt/refactor changes. After those changes, all 295 ledger tests passed, including 20 transcription persistence cases; workspace Clippy passed across all targets with `dev-tools` and warnings denied. `git diff --check` passed. Logs are `/private/tmp/vpt-transcript-state-unit.log`, `/private/tmp/vpt-transcript-ledger-final.log`, and `/private/tmp/vpt-transcript-state-clippy-final.log`.


## Durable raw engine outputs and recovery

Schema version 3 adds `engine_outputs`, with the original document bytes, target, digest, recording identity, engine name, duration, creation time, and reported/echoed language provenance. The record and comparison outputs are saved together with the accepted transcript, matching history, and one publication entry per output. Byte digests, document identity/timing, accepted text, safe filenames, and publication preconditions are checked before committing. An unfinished publication prevents replacement of its saved bytes; a later attempt requires the previously saved digest.

Saved documents are bounded before allocation into Rust and revalidated when read. Corruption produces a fixed diagnostic without document content. Unknown additive fields and original formatting remain in the saved raw bytes. Accepted render inputs remain in their separate repository.

Publication recovery is shared by ingestion and retention startup. It can publish committed engine bytes without rerunning an engine, sync the containing directory, and clear the journal with confirmed history. Modified targets are preserved and remain pending. Engine-output files and temporary files are created with mode 0600; existing shared directory permissions are preserved. Dry runs perform no publication or journal repair.

The migration preserves version-2 transcripts and history, including original record identifiers. An injected precommit failure rolls back the new table and version; a later successful upgrade preserves the existing rows. Older observational connections report no saved engine outputs without migrating.

The tests first failed for missing publication validation, replacement of pending bytes, missing startup render state, non-private publication mode, and lost language provenance. Those cases now pass. Shared contracts cover byte-for-byte record/checker storage, invalid batches, expected previous digests, and preservation on refusal. Command tests cover recovery through both ingestion and retention, dry-run snapshots, modified targets, permissions, and absence of transcript text in events.

Verification: the workspace unit run passed 1,118 tests before the final corruption/provenance checks. All 32 ingestion and retention command tests passed; the four raw-output startup cases passed again after provenance persistence was added. Workspace Clippy passes with warnings denied. All 309 ledger tests passed after the final changes (`/private/tmp/vpt-output-ledger-verified.log`); other logs are `/private/tmp/vpt-output-workspace-unit.log`, `/private/tmp/vpt-output-commands.log`, `/private/tmp/vpt-output-commands-final.log`, and `/private/tmp/vpt-output-clippy-final.log`.

The core command now prepares and commits these records as described below. Engine working-directory recovery, availability checks, diagnostic/flag persistence, note generation, and the later roadmap stages remain unfinished.


## Core transcription command

`vpt transcribe <id> [--language <tag>] [--dry-run] [--json]` now selects configured engines, checks same-family refusal before writes or execution, and reports engine locality in its dry-run plan. The command validates the archived file against its saved digest, reads its exact media duration, takes the shared mutation lock, recovers logging/publications/retention, and saves a run start before preparing new work.

Every engine attempt receives a separate private copy and output directory. The copy is bounded by the configured audio-size ceiling and checked against the archive digest before execution. The archive is never passed to a configured engine. This also gives the checker fresh input when a failed main engine deletes its working file. Whisply currently makes its own additional working copy inside the guarded directory; both copies are cleaned when the attempt returns. The audio remains byte-for-byte unchanged, without a VPT transcoding step.

Successful execution prepares accepted state and raw publication rows through the application layer, commits matching history, publishes from saved bytes, and finishes the run. Both-engine failure marks transcription failed, preserves the earlier accepted text, and emits a privacy-filtered `transcribe_failed` event. Modified raw-output targets are preserved and refuse replacement. Confirmed accepted identities remain in errors if later publication fails. Log-export warnings preserve the operation result.

The success result includes whole-recording diagnostics and safe warnings from successful and failed engines, including retained temporary paths after cleanup failure. These diagnostics are not yet the persistent review-flag model. Note publication and uncertainty classification remain unfinished; no roadmap completion is claimed for those parts.

Command tests were observed failing before the implementation. Seven command cases now cover observational dry runs, successful private publication, a failed retry, same-family refusal without writes, changed archives, checker fallback after main-input deletion, and changed output files. The fallback test also failed before safe engine warnings were included in success output.

Verification: all 1,120 workspace unit tests passed before the final output-warning change; all 39 transcription, ingestion, and retention command tests passed afterward. Workspace Clippy passes across all targets with warnings denied. Logs: `/private/tmp/vpt-transcribe-unit.log`, `/private/tmp/vpt-transcribe-integration.log`, and `/private/tmp/vpt-transcribe-clippy-final.log`. The existing no-speech-assets limitation still prevents a successful native Apple smoke test on this host.

## Uncertainty classification and durable flags

Transcription now classifies built-in engine output before committing accepted state. The pure domain classifier uses the existing normalized Myers alignment and applies the specified class order: formatting, numeric, proper-noun, low-confidence, agreed-unverified, and other. Formatting is retained without surfacing. Divergence stops disagreement spans while preserving record confidence/risk classification. Generic record engines produce no lexical flags; generic checkers cannot supply lexical comparisons. Whole-record diagnostics remain independent of those exemptions.

Flag ranges use global accepted-word offsets, with an empty range representing checker-only insertion at a word boundary. Repeated agreed-unverified surface strings retain every occurrence in one flag. Known terms use intact, case-folded whole tokens and suppress only agreed-unverified candidates. A confirmed term inside a larger capitalized span is excluded without hiding an adjacent unknown term. Input sizes and matching work are bounded; exhaustion returns an error rather than inventing divergence.

Risk detection currently uses capitalization outside sentence-initial positions, Unicode numeric characters, and an English vocabulary for spelled quantities, calendar words, times, and currencies. Adjacent words of the same risk kind form a span; sentence-ending punctuation ends it. This is heuristic detection: sentence-initial and lowercase names are not inferred, and spelled risk vocabulary is English-only. Ambiguous words such as month names may prompt review. Broader risk detection remains work to assess; the complete classifier roadmap item remains unchecked.

The typed configuration now carries confidence/divergence thresholds, the known-terms path, and the readability setting. The known-terms loader performs bounded descriptor reads, rejects redirects and invalid text, checks file/root stability, and treats missing files as an empty list without creating them. Readability transformation and known-term writes are still pending.

Lexical identifiers are SHA-256 digests of versioned, length-framed recording identity, class, occurrence ranges, record text, and optional alternative text. A fixed expected digest protects the encoding contract. Confidence changes do not change identity. Diagnostic identifiers use a separate versioned encoding. Debug output omits flag text, alternatives, and correction text.

SQLite and the memory ledger now save lexical/diagnostic flags with accepted transcripts and matching history. Invalid ranges, text, confidence, duplicate diagnostics, and command-engine lexical flags refuse the transaction. A failed history append rolls back flags and accepted state together. Failed transcription preserves prior flags. Re-transcription removes obsolete transcript flags, retains unchanged resolutions, and recomputes persisted open/diagnostic counts. SQLite reads validate identifiers, range/text correspondence, and bounded fields; malformed rows produce fixed errors without transcript content. Existing verification rows are retained; ownership validation for verification findings remains part of the later artifact work.

The command calls this path, so flags survive reopening and raw-output publication recovery. A fake Apple helper command test verifies repeated-name occurrences, confidence overrides, whole-token known terms, immutable accepted words, and privacy in events. Another verifies that an invalid terms file preserves accepted flags. These tests exercise the adapter/CLI contract without running native speech recognition. Review commands, correction/confirmation writes, readability, note rendering, and post-publication review alerts remain unfinished. No review alert is emitted before note publication.

Verification: all 1,162 workspace unit tests passed with one test thread (`/private/tmp/vpt-uncertainty-unit-serial.log`); all 41 ingestion, retention, and transcription command tests passed (`/private/tmp/vpt-uncertainty-commands.log`). Workspace Clippy across all targets with `dev-tools` and warnings denied passed (`/private/tmp/vpt-uncertainty-clippy-final.log`). `git diff --check` passed. The earlier parallel unit run hit five 500 ms database watchdog timeouts and one Whisply fixture deadline; deadlines were not changed, and the serial run passed those cases. The complete workspace integration suite was not rerun in this step.

## Read-only review command and consistent snapshots

`vpt review <id> [--json]` now displays open surfaced lexical flags in class order, then accepted-word order. Each row includes its stable identifier, record text, alternative, confidence, occurrence count, first `[mm:ss]` timecode, and every occurrence's word range and precise start/end seconds. Formatting and resolved flags are omitted. Diagnostics have their own list/count and no invented source ranges. Human text is quoted so embedded control characters are escaped. Existing recordings without accepted text return an empty review with `transcribed: false`; unknown or malformed identities return usage errors.

The application reads a single `ReviewSnapshot` through a dedicated observational port. SQLite holds a deferred read transaction across recording existence, accepted text, and flag reads. The memory implementation clones the same state under its mutex. Accepted-transcript and flag getters also use this shared snapshot boundary. This closes a race where a concurrent transcription could otherwise pair one generation's words with another generation's flags. No mutating-command lock, migration, log recovery, hook, or engine is involved.

A two-process regression test keeps a reader open while a native SQLite writer commits, verifies the reader retains the original value, then verifies a later read sees the new value. Temporarily removing the read transaction made the test fail with the newer value appearing during the first snapshot; restoring it passed. The normal 500 ms watchdog was retained. Evidence is in `/private/tmp/vpt-review-snapshot-mutation.log` and the passing workspace unit run.

The command tests were first observed failing for the missing verb. The application tests first failed against an empty implementation. Passing cases now cover ranked human/JSON output, repeated occurrences, diagnostic separation, hidden formatting, resolved flags, insertion timing at the beginning/middle/end, invalid timings/ranges, missing/untranscribed recordings, global options, and unchanged filesystem snapshots. The read-only review roadmap item and the known-term/aggregation/stable-identifier item are now checked. Resolution writes, automatic known-term additions, readability, note publication, review alerts, and artifact verification remain pending.

Verification: all 1,168 workspace unit tests passed serially (`/private/tmp/vpt-review-unit.log`). All 44 selected command tests passed: review 4, show/list/storage 31, transcription 9 (`/private/tmp/vpt-review-commands.log`). Workspace Clippy across all targets with `dev-tools` and warnings denied passed (`/private/tmp/vpt-review-clippy.log`), and `git diff --check` passed. The full integration suite and native speech recognition were not rerun in this step.

## Readability and transcript-note rendering

The domain readability pass now removes the closed filler list, capitalizes sentence starts, and adds missing segment-ending punctuation. Its input includes protected byte ranges for every lexical flag, including formatting and resolved flags. It returns the ranges after surrounding deletions and Unicode case expansion. Tests enumerate every character-boundary substring in representative inputs and verify that protected bytes survive unchanged. The disabled pass preserves the exact text and ranges. Accepted transcripts remain immutable.

`note::transcript_content` renders one timecoded paragraph per segment, repeated flag occurrences, disagreement alternatives, and correction markers. Confirmed/dismissed flags render their accepted text; diagnostics do not change the body. Flags crossing segments are split at segment boundaries so every timecode survives. An open disagreement shows its complete alternative on the first fragment. A correction spanning segments replaces its words once at the first fragment; each repeated occurrence receives its own replacement.

The renderer maps accepted words back into segment text while preserving its punctuation and spacing. When segment text cannot represent the accepted word stream, it reconstructs that segment from the accepted words so word-based flags remain visible. Flag ranges and record-text correspondence are validated before expansion. Source text, counts, and output growth are bounded; output has a 64 MiB ceiling. External strings are escaped for Markdown and Obsidian syntax, including managed delimiters, annotations, links, comments, math, and block identifiers. Embedded newlines remain data within their paragraph.

`note::filename` derives names from capture date, the normalized title, and the identity's hash prefix. Slugs use the shared Unicode normalization/case-folding policy, collapse unsafe characters to hyphens, and use `untitled` for empty results. Truncation uses a preceding hyphen boundary when available and a character boundary for a single oversized word. Pinning and filesystem authority checks belong to the pending publication integration.

`note::document::create` produces fresh portable and Obsidian transcript notes with typed provenance, language, uncertainty counts, quoted YAML values, a filename-matching heading, an empty links region, and the rendered content region. Obsidian metadata precedes VPT metadata and derives `startDate` from the local first-write instant. `note::managed::Regions` extracts exact content bytes and replaces the two region bodies while preserving all surrounding bytes. It rejects missing, duplicate, non-standalone, reversed, and overlapping markers. Link-only changes preserve the exact content bytes.

Behavioral tests failed against empty implementations before each component was added. Additional failing cases drove fixes for bare web-address escaping and Obsidian-specific markup. All new production modules are below 150 lines and use safe Rust. Remaining integration work includes note settings, atomic render-state persistence, ownership/schema checks, frontmatter rewrites that preserve operator fields/tags, durable publication and recovery, resolution commands, and post-publication review alerts. The corresponding roadmap items remain unchecked.

Verification: all 1,207 workspace unit tests passed serially (`/private/tmp/vpt-note-unit-final.log`), including 151 domain tests. Workspace Clippy across all targets with `dev-tools` and warnings denied passed (`/private/tmp/vpt-note-clippy.log`). Rustfmt checks on the new rendering/readability modules and `git diff --check` passed. Command integration tests and native speech recognition were not rerun in this step.

## Existing-note documents and stable reads

The adapter now parses bounded YAML frontmatter and validates note identity, stage, schema, and body markers. It uses the pinned `saphyr-parser` 0.0.11 event parser with bounded nodes, nesting, scalar text, and alias expansion. Header offsets account for UTF-8, and container spans account for trailing flow separators and comments. Future schemas can be inspected but refuse rewriting. Marker-looking text in frontmatter does not count as a body marker.

Rewriting takes a complete freshly rendered note, updates VPT properties and both managed regions, merges generated and manual tags, and retains other frontmatter and surrounding prose. It validates the complete result before returning it. Tests cover block and flow YAML, Unicode, comments, anchors, explicit scalar types, CRLF, foreign identities, malformed markers, and expansion limits. A failing Unicode-whitespace test exposed that Rust's general whitespace trimming left a nonbreaking space outside a replacement scalar; replacement spans now preserve only YAML's ASCII separator whitespace.

`NoteFiles` provides descriptor-based UTF-8 reads and a digest of the bytes read. Filesystem reads enforce the note size limit, refuse redirected paths, compare descriptor and named-file metadata, and revalidate the retained store root. Race tests cover file replacement, modification, deletion, and directory redirection. Debug output omits note text. These checks establish stable input bytes and document identity; the pending publication workflow must still prove ledger ownership and enforce pinned paths.

## Runtime note settings and vault diagnostics

Typed settings now retain the note profile, independently selected link style, slug limit, three Obsidian hubs, and expanded optional vault path. Portable mode has no vault requirement. Obsidian mode rejects empty or relative vault paths during settings conversion and requires an existing directory during root resolution. Existing shared-vault contents and permissions remain unchanged. Retention settings conversion was split into its own module to keep the settings modules below 150 lines.

`doctor` now includes a named `vault` check in both human and JSON output, including unavailable-configuration reports. It reports the resolved directory or the reason it cannot be used, while continuing independent checks. Configuration and command tests were observed failing before these behaviors were implemented.

Atomic note render-state persistence, pinned-path ownership, publication/recovery, correction commands, post-publication review alerts, and the later roadmap stages remain unfinished. No corresponding feature checkbox has been marked complete.

Verification: 1,235 workspace unit tests passed serially (`/private/tmp/vpt-note-settings-unit-final.log`), including 918 adapter tests, 17 note-document tests, and five stable note-file read tests. All 33 doctor command tests passed (`/private/tmp/vpt-doctor-vault-final.log`). Workspace Clippy across all targets with `dev-tools` and warnings denied passed (`/private/tmp/vpt-note-settings-clippy.log`); formatting checks and `git diff --check` passed. Earlier runs hit two existing doctor command deadlines and one database watchdog assertion; final serial reruns passed without changing those tests or their deadlines. Native speech and the full command integration suite were not rerun in this step.

## Durable owned-note state and startup publication recovery

Schema version 4 adds `owned_notes`, with one transcript and one analysis artifact per recording. Each record retains its pinned target, exact publication text and digest, content-region digest, managed tags, first-write instant and offset, and update time. Note changes commit with their dirty publication entries and matching history. Both repositories reject foreign target claims, changed pins or first-write metadata, stale saved-state digests, pending publications, and paths reserved by pending retention. Rewrites may expect the digest of current operator-edited bytes rather than the previous generated file; the saved-state digest separately guards against stale ledger input.

The SQLite transaction also updates the existing recording path column. An injected history failure rolls back the note bytes, pin, and publication entry together. Transcript and analysis state can commit together with one completed recording identifier. Note text, operator prose, and managed tags are excluded from debug/history output. The storage operation does not by itself mark a note stage published; command orchestration and stage transitions remain part of the pending note-writing workflow.

Reads validate saved document identity, stage, schema, markers, full digest, content digest, and managed tags. SQLite column lengths are checked before allocating owned values, and saved tag JSON has byte, depth, text, and array limits. The bounded JSON writer is shared with accepted-transcript serialization. Corruption tests cover altered text, content digests, foreign tags, and oversized body/tag columns. Oversized cases run separately within the existing watchdog rather than extending its deadline.

The version-3 migration test verifies observational access before migration, precommit rollback, and preservation of engine bytes, publication entries, and history after upgrade. Earlier schema migration tests now include version 4 and continue to verify rollback and preservation. Reopened and observational ledgers return the saved notes without a migration or write.

Startup publication recovery now loads committed note bytes as well as engine outputs. Command tests cover publishing a saved note, acknowledging bytes already published, preserving an operator replacement with the journal entry pending, and leaving pending work untouched during a dry run. A failing startup test reported missing render state before the note lookup was added. No engine is needed to recover the note.

The note-writing command, consistent collection of rendering inputs, rename recovery, note stage transitions, correction commands, and post-publication review alerts still need integration. This work leaves those roadmap items unchecked.

Verification: all 1,257 workspace unit tests passed serially (`/private/tmp/vpt-owned-notes-unit.log`), including 940 adapter tests. All 113 selected command tests passed across ingestion, retention, transcription, review, doctor, and show/list/storage (`/private/tmp/vpt-owned-notes-commands.log`). Workspace Clippy across all targets with `dev-tools` and warnings denied passed (`/private/tmp/vpt-owned-notes-clippy.log`); formatting checks and `git diff --check` passed. No native speech run or Swift changes were needed for this step.

## Transcript-note command and durable checker provenance

`vpt note write <id> [--json]` now publishes a transcript note from a consistent ledger snapshot of the recording, accepted words, review flags, and saved notes. Unknown recordings and recordings without accepted text return usage errors before starting mutating work. The command shares the write lock and startup logging/publication/retention recovery, saves a note run, and commits rendered bytes with publication intent and matching history. It preserves committed recording identifiers and logging warnings on later failures. Transcription now uses the same logging-recovery helper.

Portable and Obsidian notes use the configured profile, link style, slug, hubs, and readability setting. Existing owned documents preserve operator prose, manual tags, editable frontmatter, pinned filenames, and first-write metadata. Invalid markers, foreign occupants, and future schemas refuse rewriting. Explicit regeneration after expiration uses the previous saved document as the rewrite base, preserving saved operator edits while refreshing managed content. Its command test first failed because regeneration discarded custom frontmatter; the corrected implementation passes. Missing unexpired notes still refuse until rename recovery is implemented.

Successful publication now commits the note stage transition with publication-journal removal and observation history. A shared SQLite/memory contract injects a late history failure and verifies rollback of both the stage and journal removal, then confirms the successful transaction. Saving note bytes alone leaves the stage pending.

Accepted transcripts retain optional checker-engine metadata independently of raw output files. Serialization checks metadata bounds before cloning and still reads older accepted bodies without the new field. Engine-output transactions require exact agreement between saved checker provenance and the accompanying raw document. New shared contract tests first demonstrated that both repositories accepted a mismatched model; they now refuse wrong models, names, missing output, and missing metadata without saving state or history. The existing checker-output contract verifies the matching metadata round trip. This does not reconstruct checker provenance for older accepted bodies that lack the field.

Eight note command tests cover publication without audio/engines, preservation of operator changes and pins, malformed/foreign/future-schema refusals, Obsidian Markdown references, no-write input errors, and regeneration. A tenth transcription command test runs main and checker successfully, removes audio and raw-output files, and verifies note publication with both saved engine identities. These use fixture engines; native Apple Speech and real Whisply were not run.

Verification: 1,263 workspace unit tests passed serially, including 946 adapter tests (`/private/tmp/vpt-note-command-unit.log`). All 122 selected command tests passed across note, transcribe, ingestion, retention, review, doctor, and show/list/storage (`/private/tmp/vpt-note-command-integrations.log`). Workspace Clippy across all targets with `dev-tools` and warnings denied passed (`/private/tmp/vpt-note-command-clippy.log`). Workspace formatting and `git diff --check` passed. The full integration suite was not rerun.

Analysis-note rendering, rename recovery, indexing, tags/relations, correction writes, and post-publication review alerts remain unfinished. The filesystem publisher still checks the expected digest before a separate replacement operation; closing that concurrent-edit window remains required. No complete note-workflow roadmap checkbox is claimed by this step.

## Pinned-path lookup and transcript rename recovery

`vpt path <id> --stage transcript|analysis [--json]` now returns the ledger's pinned absolute path. The query loads only the bounded path column, validates its shape, and does not inspect artifact bytes. Missing pins and invalid identities/stages return usage errors. The command does not take the write lock, recover publications, create state, or migrate an old schema. Command tests verify human/JSON output, changed and missing files, a held application write lock, an unusable newly configured store, absent state, and pending analysis bytes. Corruption tests verify that a valid pin remains readable when saved note body/tag data is invalid, while relative and oversized pins refuse. The Stage 3 path-command checkbox is now complete; occasion paths remain part of briefs.

When a transcript's pinned file is missing, `note write` now scans the flat store for its frontmatter recording identity. A unique claimant is validated and rewritten with its operator content intact; no claimant allows regeneration at the old pin. Multiple claimants produce a named refusal listing their paths. Markdown symlinks are refused without following them. A foreign occupant at the current pin still refuses, even if another file claims the recording. Matching notes with invalid markers or unsupported metadata fail normal document validation before any pin change.

Discovery uses bounded directory enumeration (65,536 entries), a 256 MiB aggregate declared-size budget, bounded stable note reads, and refusal when read size differs from the inventory. It compares the inventory again and rereads the selected claimant before returning render inputs. A deterministic test adds a second claimant or edits the selected note during identity inspection and verifies refusal. These checks detect changes during discovery; they do not close the separate digest-check-to-publication replacement window previously recorded.

A relocation carries the expected previous pin and saved digest. Both repositories require same-store movement, exactly one matching `note_moved` history fact at the note update time, and no pending publication or retention at either old or new path. The new pin, saved note bytes, dirty publication, and move history commit together. SQLite also updates the recording's path column. Shared contract tests verify rollback after a late history failure, stale/missing/duplicate relocation evidence, changed saved state, escaped destinations, and reservations at both paths. Repeated command execution logs the move once and retains first-write metadata. The history codec and log protocol cover the new fact; logging recovery treats it as work activity without sending a hook.

Six rename command cases cover unique moves, no-match regeneration, ambiguity, malformed claimants, symlink refusal, and foreign occupants. Rename discovery currently applies to transcript-note writes. Analysis rendering and automatic pipeline integration remain unfinished, so the broader rename and note-writing roadmap items remain unchecked.

Verification: all 1,272 workspace unit tests passed serially, including 953 adapter tests (`/private/tmp/vpt-note-path-rename-unit-final.log`). All 131 selected command tests passed across note/path, transcribe, ingestion, retention, review, doctor, and show/list/storage (`/private/tmp/vpt-note-path-rename-commands-final.log`). Workspace Clippy across all targets with `dev-tools` and warnings denied passed (`/private/tmp/vpt-note-path-rename-clippy-final.log`); workspace formatting and `git diff --check` passed. Earlier runs hit the existing 500 ms watchdog in oversized-corruption setup and one doctor command. Large corruption fixtures now prepare their oversized rows before the supervised observational read, which retains the same 500 ms deadline. The final doctor suite passed without changing its deadline. The full integration suite and native speech engines were not rerun.

## Review resolution state and synthesis invalidation

The domain and application now prepare confirmation, correction, and dismissal of surfaced flags. Repeating an identical action preserves its original resolution time; an explicit different action can replace a previous resolution. Hidden formatting flags cannot be resolved, and whole-recording diagnostics cannot be corrected. Corrections remain separate from accepted words. Preparation identifies known-term additions from corrected text and from proper-noun/agreed-unverified confirmations; it does not yet write the terms file.

Correction text is bounded and rejects unsafe control characters while retaining tabs and line breaks. A failing behavioral test showed that the initial validator rejected embedded newlines required by the rendering specification. The corrected policy preserves the exact resolution text, renders newline and Markdown syntax as entities, and prepares one whitespace-normalized known term. Invalid actions leave the input snapshot unchanged. Debug output omits correction and term text.

Both ledger implementations now commit a resolution with a matching completed-work fact and a private-safe `flag_resolved` history fact. They reject stale expected state, unknown flags, reopening, invalid correction text, and missing, duplicated, or mismatched history. SQL recomputes open lexical/verification and diagnostic counts. Reopening the observational ledger retains the resolution and accepted transcript. Unchanged flags preserve resolutions after re-transcription. Shared contracts verify that a resolution, rendered note, and publication intent commit together or all roll back after a late history failure.

Changing the saved transcript content digest now makes succeeded synthesis pending in the same transaction. Operator prose changes outside the content region preserve synthesis status, and expired/disabled synthesis remains unchanged. Shared SQLite/memory contracts observed the missing invalidation before implementation and verify both success and rollback behavior. The synthesis command and analysis renderer remain unfinished.

The resolution CLI, durable known-term updates, and their publication/recovery orchestration remain unfinished. The corresponding roadmap items remain unchecked. The previously recorded filesystem replacement race also remains unresolved.

Verification: all 1,297 workspace unit tests passed serially, including 969 adapter tests (`/private/tmp/vpt-review-resolution-unit-final.log`). All 131 selected command tests passed across note/path, transcribe, ingestion, retention, review, doctor, and show/list/storage (`/private/tmp/vpt-review-resolution-commands-final.log`). Workspace Clippy across all targets with `dev-tools` and warnings denied passed (`/private/tmp/vpt-review-resolution-clippy-final.log`); workspace formatting and `git diff --check` passed. No test deadlines changed in this step. The full integration suite and native speech engines were not rerun.

## Known-term file updates and publication inputs

The known-terms adapter now prepares additions without writing, preserving the existing file's bytes, spellings, punctuation, and line endings. A new term becomes one whitespace-normalized line. Whole-term duplicate detection uses the same Unicode compatibility normalization and case folding as known-term matching; punctuation remains significant. Duplicate confirmation leaves the file bytes and modification time unchanged. The existing 16 MiB, 65,536-line, and 65,536-character bounds apply before writing, including the output size after an addition.

Publication writes a private temporary file, synchronizes it, verifies the owned temporary and the expected destination, renames, synchronizes the directory, and checks the final content. An absent destination uses exclusive rename. Created files use mode 0600; missing parent directories use 0700, while existing shared directory modes remain unchanged. Symlinks, nonregular targets, multiply linked lists, changed bytes, and redirected parents refuse. Ordinary failures remove only the owned temporary through the retained directory descriptor, including after the parent is renamed; foreign replacement files remain untouched.

The update exposes its intended bytes and a `DirtyPublication` containing the previous/intended digests. `Update::restore` validates those saved inputs and can resume before replacement or acknowledge the already-published bytes without adding a duplicate. The caller must still save those inputs under the write lock before invoking publication. Ledger storage, startup replay, safe configured-path authority, and the confirmation/resolution commands are not wired up yet, so the known-term and review-resolution roadmap items remain unchecked.

Behavioral tests first failed for the missing writer, restore operation, acknowledgement after a concurrent edit, cleanup after directory redirection, and tampered staging bytes. They now cover those cases along with exact capacity limits and idempotence. The checks detect changes at the inspected boundaries; existing-file replacement still has a separate check-to-rename interval, as recorded for general artifact publication. This step does not establish race-free replacement against an uncooperative concurrent editor.

Verification: all 1,315 workspace unit tests passed serially, including 987 adapter tests and 22 known-term tests (`/private/tmp/vpt-known-terms-unit-final.log`). All ten transcription command tests passed, including known-term classification and private-safe invalid-input refusal (`/private/tmp/vpt-known-terms-command-final.log`). Workspace Clippy across all targets with `dev-tools` and warnings denied passed (`/private/tmp/vpt-known-terms-clippy-final.log`); formatting and `git diff --check` passed. The wider command integration suite and native speech engines were not rerun for this adapter change.

## Pending known-term publications in the ledger

Schema version 5 adds bounded private bytes for a pending known-term publication, linked to its existing publication-journal entry. Both repositories now commit those bytes, the previous/intended digests, and a matching publication-intent history fact together. They reject invalid documents or paths, missing or mismatched history, duplicate pending work, and conflicting artifact targets. A pending known-term publication also prevents a generic journal upsert from replacing its saved intent.

Clearing the publication removes the saved bytes in the same transaction; SQLite uses a foreign key with cascading deletion. A late history failure rolls back both completion and saved-byte removal. Shared contracts now verify that a flag correction, rendered note, and known-term publication commit or roll back together while accepted words remain unchanged. Debug and history output omit the terms file's content.

The SQL reader checks column types, lengths, document limits, and the intended digest before returning saved bytes. It refuses malformed bodies, oversized bodies, corrupted digests, and invalid times with fixed private-safe errors. Read-only reopening preserves pending work. Reading an older schema returns no known-term publication without migrating it. The version-four migration test preserves existing notes, publication intents, and history, including rollback after a migration failure; the earlier migration fixtures now exercise upgrades through version 5.

The adapter and ledger are ready for command orchestration, but startup replay, configured-path authority checks, and the confirmation/resolution CLI remain unfinished. The roadmap items remain unchecked, and the existing-file replacement race remains unresolved.

Verification: all 1,335 workspace unit tests passed serially, including 1,007 adapter tests (`/private/tmp/vpt-known-terms-ledger-unit-final.log`). All 131 selected command tests passed across note/path, transcribe, ingestion, retention, review, doctor, and show/list/storage (`/private/tmp/vpt-known-terms-ledger-commands-final.log`). Workspace Clippy across all targets with `dev-tools` and warnings denied passed (`/private/tmp/vpt-known-terms-ledger-clippy.log`); formatting and `git diff --check` passed. An earlier run exceeded the 500 ms watchdog while one fixture exercised eleven independent resolution-refusal variants; those variants now run in two supervised tests with the same assertions and unchanged deadline. The full integration suite and native speech engines were not rerun.

## Explicit term confirmation and startup recovery

`vpt confirm --term <text> [--json]` now confirms a term without a recording, archived audio, or engine. Input validation happens before mutating work. Under the write lock, the command recovers logging, pending publications, and retention before preparing an addition. It saves a confirmation run and the known-term bytes with publication intent before writing. Completion is logged after durable publication. Duplicate terms preserve file bytes and modification time; human and JSON results report confirmation, whether an addition was needed, and the destination without echoing private term text.

The shared publication workflow now reads known-term render state and uses the known-term writer. It resumes an unpublished update or acknowledges already-published bytes, clears saved bytes and the journal together, and refuses operator changes or a pending path that no longer matches `reconcile.known_terms_path`. Ingestion, transcription, note writing, retention, and explicit confirmation use that workflow. Dry runs remain observational. Confirmation uses existing log-warning behavior; an unavailable event log preserves the operation's successful result and adds a safe warning. Note writing and transcription now share the same output-warning helper.

Known-term destinations cannot overlap state, Voice Memos, or the released store. Configuration-file aliases are detected by file identity as well as path, so a hard-linked alias refuses before state creation. A behavioral test first showed that this alias reached the later writer refusal after beginning a run; the earlier configuration check now preserves an observational failure. Symlinks and nonregular lists remain writer refusals. Newly created list parents are private, and shared parent permissions remain unchanged.

The argument parser extracts term text before global options, allowing option-like literal text without interpreting it as configuration or output flags. Debug formatting hides the value. Thirteen command tests cover confirmation/idempotence, private permissions and output, invalid input, pending and already-published recovery, operator changes, configured-path changes, protected destinations, configuration aliases, redirected lists, logging failure, custom parents, and retention startup versus dry run. Parser tests cover literal values and malformed combinations.

Automatic additions from review confirmations/corrections still require the resolution CLI, so the combined roadmap checkbox remains unchecked. The previously recorded existing-file replacement race also remains unresolved.

Verification: all 1,337 workspace unit tests passed serially, including 1,007 adapter tests (`/private/tmp/vpt-confirm-unit-final.log`). All 144 selected command tests passed across confirmation, note/path, transcription, ingestion, retention, review, doctor, and show/list/storage (`/private/tmp/vpt-confirm-integrations-final.log`). Workspace Clippy across all targets with `dev-tools` and warnings denied passed (`/private/tmp/vpt-confirm-clippy-final.log`); formatting and `git diff --check` passed. No test deadlines changed. The full integration suite and native speech engines were not rerun.

## Review resolution command

`vpt review <id> --resolve <flag> --confirm|--correct <text>|--dismiss [--json]` now resolves surfaced lexical flags and whole-recording diagnostics. The parser requires exactly one action and treats correction text as literal data, including option-like values. Debug formatting hides correction text. Invalid identities, unknown/hidden flags, malformed corrections, and diagnostic corrections refuse before mutation. Read-only review retains its observational behavior.

Under the write lock, the command recovers pending logging, publications, and retention, then reads a fresh note/review snapshot. It prepares the resolution and rendered transcript, including rename recovery and operator-content preservation. The resolution, note bytes, any known-term addition, and corresponding history commit in one transaction before publication. Confirmations of proper-noun/agreed-unverified flags learn their record text; every lexical correction learns its normalized text. Dismissals and diagnostic resolutions do not learn terms. Accepted words remain unchanged.

Repeated identical actions retain the original resolution time, avoid duplicate terms, and rerender the current note. A different explicit action can replace the previous resolution. Changed transcript content invalidates succeeded synthesis through the existing ledger rule. Results include the resolved identifier/state, whether it changed, note path, and remaining open flags. Logs omit correction and transcript text. Safe logging warnings preserve a completed result; later publication failures report the committed recording and keep recoverable intent.

Initial behavioral tests failed because the CLI rejected `--resolve`. Expanded command tests cover transaction rollback, acknowledgement failure and restart, immutable accepted words, duplicate actions, diagnostic handling, invalid-input observation, malformed notes, renamed notes with operator edits, synthesis invalidation, redirected term lists, literal correction values, and log-export warnings. The known-term checklist item is complete. The resolution item remains unchecked because updating existing analysis notes still requires the Stage 4 renderer; the current command refuses that case. Downstream verification invalidation also remains tracked in Stage 4. The existing-file publication race remains unresolved.

Verification: all 1,339 workspace unit tests passed serially, including 1,007 adapter tests (`/private/tmp/vpt-review-cli-unit-final.log`). All 155 selected command tests passed across confirmation, note/path, transcription, ingestion, retention, review, doctor, and show/list/storage (`/private/tmp/vpt-review-cli-integrations-final.log`), including 15 review command tests. Workspace Clippy across all targets with `dev-tools` and warnings denied passed (`/private/tmp/vpt-review-cli-clippy-final.log`); formatting and `git diff --check` passed. New production modules are below 150 lines. No test deadlines changed. The full integration suite and native Apple Speech/real Whisply were not rerun.

## Transcription note publication and review alerts

`vpt transcribe` now publishes its transcript note after saving the accepted transcript and publishing raw engine outputs. It uses the same owned-note writer as `note write`, including profile/readability settings, pinned paths, rename recovery, operator-content preservation, and durable publication acknowledgement. Results include the published note path and open uncertainty counts. Newly accepted words remain saved if later note rendering or publication fails, and the error lists the committed recording.

Acceptance now marks the note pending in the same transaction while preserving expiration. A behavioral command test exposed the previous stale-success state when a malformed note refused rendering after a new transcription. Shared SQLite/memory contracts cover invalidation, expiration preservation, and rollback after a late history failure. Retranscription leaves an expired note absent and sends no review alert for it; explicit note writing remains the regeneration path.

After note publication and acknowledgement, the command records `review_needed` with safe identity/path metadata and counts of open lexical occurrences and diagnostics, then invokes only the optional alert hook. Repeated occurrences count individually; resolved flags and unchanged corrections remain closed after retranscription. Failed publication emits no alert. Hook failures preserve success with a warning and no retry/fallback. An unavailable event log still permits the original alert. Recovery repairs saved note bytes without replaying hooks.

A shared aggregation planner emits individual events through the configured threshold and one aggregate event above it. The aggregate has a recording count and combined class counts, without individual paths/identities. The planner bounds batches and retained path bytes, rejects duplicate recordings and count overflow, and is covered by protocol and history-codec tests. Multi-record command integration still requires `vpt run`, so the broader alert checklist item remains unchecked. The first-transcript-note checklist item is complete; analysis notes and the previously recorded concurrent-edit publication race remain unfinished.

Verification: all 1,350 workspace unit tests passed serially, including 1,013 adapter tests (`/private/tmp/vpt-transcribe-note-unit-final.log`). All 164 selected command tests passed: 145 across confirmation, note/path, ingestion, retention, review, doctor, and show/list/storage (`/private/tmp/vpt-transcribe-note-integrations-final.log`), plus 19 transcription tests on the final run (`/private/tmp/vpt-transcribe-note-command-final.log`). The first integration run's new Obsidian assertion used `begin` instead of the specified `start` marker; correcting the assertion and rerunning the transcription suite passed. The stage contracts now use separate fresh ledgers for pending and expired cases, preserving the existing duplicate-recording guard. Workspace Clippy across all targets with `dev-tools` and warnings denied passed (`/private/tmp/vpt-transcribe-note-clippy-final.log`); formatting and `git diff --check` passed. No test deadlines changed. All new production modules are below 150 lines. The full integration suite and native Apple Speech/real Whisply were not rerun.

## Initial pipeline and repeat-run stability

`vpt run [--dry-run] [--json]` now composes ingestion, transcription, and transcript-note publication. One write lock covers startup recovery and all mutating stages. Records resume in capture order from their earliest incomplete stage; pending, failed, and previously disabled work can proceed while successful transcription and expired notes remain preserved. A failed stage retains earlier committed work and reports its recording identities. Pipeline history uses the `run` kind, with existing ingestion/transcription history retained.

The dry run reads existing state and reports ingestion candidates and selected stages without migrations, publication repair, engine execution, hooks, or durable writes. A pending note can finish with both audio and the engine absent. Successful notes are prepared against current settings so readability/profile changes refresh them from accepted words. Unchanged runs skip note publication and repeat alerts.

Review notices are queued only after acknowledged note publication. A bounded queue emits individual events through the configured threshold or one combined event above it. A four-record behavioral test verifies aggregation, privacy, and that all note files exist before the hook runs. A later engine failure still reports the notes already published. Startup retention-hook warnings survive a later source failure; a regression test first observed the missing warning before propagation was fixed.

Repeat-run tests exposed a YAML formatting change from block to flow lists even when the managed value was unchanged. The formatter now retains the existing representation of equal values. Aliases still become independent authoritative values, with operator anchors preserved. Separate adapter tests cover both cases. A pipeline refusal test also exposed named note refusals being converted to exit 1; the pipeline now preserves the stage's error kind and exit code, including exit 3, without changing operator bytes.

The exact-content audit found no missing-heading defect: `transcript_content` includes `## Transcript` and the terminating newlines, and ledger validation hashes the parsed content region before accepting note state. Existing SQLite/memory contracts cover content-only synthesis invalidation and preservation after outside-region edits; review/readability command tests exercise the changing render inputs. The initial pipeline, aggregated review alert, and saved content-input roadmap items are now checked. Synthesis execution, analysis notes, automatic redaction, new retention sweeps, all-store retention, and concurrent-edit publication safety remain unfinished.

Verification: all 1,355 workspace unit tests passed serially (`/private/tmp/vpt-pipeline-units.log`). Eight command suites passed all 160 tests: confirmation 13, doctor 33, ingestion 18, note/path 17, retention 18 (`/private/tmp/vpt-pipeline-commands.log`), show/list/storage 31 and transcription 19 (`/private/tmp/vpt-pipeline-observe-transcribe.log`), and the final pipeline suite 11 (`/private/tmp/vpt-pipeline-command-final.log`). Workspace Clippy with warnings denied passed (`/private/tmp/vpt-pipeline-clippy-final.log`); formatting and diff checks passed. New pipeline production files remain below 150 lines.

The broader command gate is not green. Five review tests initially exceeded their existing 500 ms child deadline; an isolated suite rerun passed 12/15, leaving repeated confirmation, renamed-note correction, and replacing a correction with dismissal over the deadline (`/private/tmp/vpt-pipeline-remaining-commands.log`). Temporary timing instrumentation on repeated confirmation showed progress through recovery and the resolution commit, followed by the deadline during event-log export (`/private/tmp/vpt-review-timing-expanded.log`). This narrows the location but does not establish the cause or prove a timing fix. All temporary instrumentation was restored byte-for-byte, no deadlines or durability checks changed, and the review timing failures remain open. The full integration suite and native Apple Speech/real Whisply were not rerun.


## Transcription engine and persistence acceptance

Closed seven engine/transcription checklist rows against specification sections 6.1–6.3 and 6.6.
The new same-family acceptance case exposed a missing error detail: refusal omitted the configured
engine names and shared family. The error now includes all three, with escaped string formatting,
while retaining exit 3 and refusal before any engine invocation or durable mutation.

| Requirement | Evidence |
| --- | --- |
| Archived audio, language arguments, and observational dry run | Existing command tests and the new pair fixture verify unchanged archive bytes, byte-identical private working inputs, regional language arguments, cleaned temporary inputs, unchanged dry-run snapshots, and preserved accepted state after engine failure. |
| Main/checker policies, overrides, and defaults | `transcribe/pairs.rs` exercises all eight combinations of policy and engine success/failure. Recorded start/end order proves sequential invocation and checker skipping. A command-level language override selects its configured pair and reports declared locality. Settings/application tests cover local Apple/no checker, `en-US`, the 1,800-second deadline, override propagation, and typed deadline failures. |
| Same-family guard and opt-in | The new command case tests both ordinary and dry-run refusal with unchanged snapshots, named engines/family, and successful explicit opt-in. |
| Whisply adapter | `transcribe/whisply.rs` exercises default MLX/large-v3-turbo and configured CPU/medium through a fixture executable with a spaced command prefix. It checks exact argv, generated JSON rather than progress stdout, unchanged audio, regional-to-base language mapping, requested-language provenance, absent MLX confidence, supplied CPU confidence, typed saved output, and private file mode. Adapter/protocol tests cover malformed, oversized, linked, unsupported-language, deadline, and cleanup cases. |
| Generic command contract and lexical exemption | Adapter tests cover single-pass placeholder substitution and argument boundaries, declared identity/locality validation, bounded stdout, and typed process failures. Command matrix cases prove a generic record generates no lexical flags despite low confidence and risky text. A new builtin-main/generic-checker case preserves builtin lexical flags and records the checker's independent language mismatch. |
| Accepted engine and diagnostic selection | The matrix checks main preference, checker fallback, `single-engine`, saved checker metadata, and both-engine failure retaining the prior accepted transcript and emitting a private-safe failure event. Domain language tests distinguish requested language from independently reported base/regional results. |
| Durable accepted text and raw documents | `transcribe/recovery.rs` forces an engine-output acknowledgement failure, removes the published output and original audio, disables the engine, and recovers through `note write`. Exact saved output and accepted state survive, publication completes, and the engine invocation log remains unchanged. Both ledger contract suites cover atomic history/state coupling, rollback, raw/accepted consistency, prior-digest checks, and checker output retention. |

The Stage 2 first-note checklist wording now says a separate managed link region: the completed
Stage 3 renderer adds its audio reference, so the final product's region is no longer empty.

Validation ran sequentially with `TMPDIR=/private/tmp`, serial CLI/SQLite tests, and unchanged
command and subprocess deadlines:

- Behavioral red: `/private/tmp/vpt-engine-command-acceptance.log` (23 passed; the new same-family
  case failed because the refusal omitted the required names/family).
- Full transcription commands after the fix: 25 passed,
  `/private/tmp/vpt-engine-commands-final.log`. The added Whisply command case passed separately in
  `/private/tmp/vpt-engine-whisply-command.log`, for 26 distinct command cases.
- Engine selection and transcription application tests: 5 and 16 passed (20 distinct),
  `/private/tmp/vpt-engine-selection.log` and `/private/tmp/vpt-engine-application.log`.
- Engine adapters/settings: 26 passed, `/private/tmp/vpt-engine-adapters.log`.
- Engine and Whisply protocol tests: 14 and 11 passed (23 distinct),
  `/private/tmp/vpt-engine-protocol.log` and `/private/tmp/vpt-engine-whisply-protocol.log`.
- Engine-output and transcript ledger contracts: 16 and 28 passed,
  `/private/tmp/vpt-engine-outputs.log` and `/private/tmp/vpt-engine-saved-transcripts.log`.
- Domain language provenance: 2 passed, `/private/tmp/vpt-engine-language.log`.
- Workspace Clippy, all targets, `dev-tools`, warnings denied: passed,
  `/private/tmp/vpt-engine-clippy.log`. Formatting passed in
  `/private/tmp/vpt-engine-format-check.log`; `git diff --check` also passed.

These are 141 distinct passing targeted tests. Engine process tests use controlled fixtures;
native Apple Speech and a real Whisply installation were not exercised in this pass. The Apple
helper, lexical-classification, and readability acceptance rows remain open, as do earlier command
timing failures and shared publication race work. The changed production module is below 150 lines;
there are no new unsafe blocks or schema changes. The checklist now has 76 checked and 31 unchecked
items.


## Lexical classification and readability acceptance

The English risk vocabulary omitted ordinals after `twelfth`. A new behavioral test first failed
for `thirteenth`, then passed after adding the remaining teen/tens ordinals, scale ordinals through
`trillionth`, and `trillion`. It checks both single-engine and agreeing-engine records, compound
numbers, exact occurrence ranges, and known-term suppression. A command test proves repeated
`thirteenth` occurrences are saved and marked inline until the whole term is known. The production
risk module remains 135 lines, with no new unsafe code, dependencies, or schema changes.

The following acceptance checks now cover the two remaining pure transcription features:

| Requirement | Current evidence |
| --- | --- |
| Classification and precedence | A fixture Apple/Whisply pair produces all six classes in order. Numeric, proper-noun, and other disagreements win over low confidence on those same words. Formatting remains stored and hidden. |
| Ranked review and aggregation | Command output exposes five surfaced rows, two occurrences of the repeated risk term, original text, alternatives, first timecode, and exact occurrence timing. |
| Risk categories and bounds | Domain tests cover names, numeric and spelled quantities, dates, times, money, strict confidence thresholds, whole-token known terms, divergence, sentence-initial capitals, and bounded inputs. |
| Generic-engine exemption | The complete transcription command suite retains the generic-record exemption and generic-checker behavior with independent language diagnostics. |
| Readability defaults and transformation | A command test starts with the default disabled setting, enables sentence capitalization, punctuation, and the closed filler list, and verifies ordinary `like` remains. Both segment boundaries are checked. |
| Protected text and immutable sources | Every lexical class's marker survives the toggle, formatting stays unmarked, both raw engine documents remain byte-identical, and accepted transcript/review flags remain equal. Disabling readability restores the original content digest. Domain tests cover protected Unicode ranges, formatting, diagnostics, and resolutions. |
| Saved-state refresh | Note rewriting works after both engine fixture commands are removed. Pipeline acceptance refreshes successful notes without invoking engines again. |
| Documentation | README documents the readability setting and the current capitalization/numeric/English-vocabulary detection heuristics. |

Risk detection remains heuristic: it does not infer lowercase or sentence-initial names, and spelled
quantity vocabulary is English-specific. Native Apple Speech and a real Whisply installation were
not invoked; the command tests exercise controlled process fixtures.

Validation used `TMPDIR=/private/tmp`, sequential build/test/lint commands, serial CLI tests, and
unchanged deadlines:

- Regression before the fix: `/private/tmp/vpt-ordinal-risk-red.log` (failed for `thirteenth`).
- Domain suite: 261 passed, `/private/tmp/vpt-lexical-domain.log`.
- Note and transcription command suites: 62 and 29 passed,
  `/private/tmp/vpt-lexical-commands.log`.
- Final readability checks, including stronger exact-content assertions: two transcription cases
  and one pipeline case passed, `/private/tmp/vpt-readability-refresh.log`.

These runs cover 353 distinct passing tests. The lexical classification and readability roadmap rows
are checked, leaving the native Apple helper acceptance row open in Stage 2. The overall checklist
is now 78 checked and 29 unchecked. Earlier command timing failures and shared publication races
remain open and are not resolved by these checks.

Workspace Clippy across all targets with `dev-tools` and warnings denied passed
(`/private/tmp/vpt-lexical-clippy.log`). Formatting passed
(`/private/tmp/vpt-lexical-format.log`), and `git diff --check` passed.


## Native helper punctuation fix and host availability

The helper previously rejected every nonempty attributed-string run without an audio time range,
including punctuation-only runs. Apple's [attributed audio-range documentation](https://developer.apple.com/documentation/foundation/attributedstring/rangeofaudiotimerangeattributes%28intersecting%3A%29)
allows untimed runs. `appleSegment` now keeps untimed punctuation in the original segment text while
omitting it from timed words. It still rejects untimed words, numbers, mixed punctuation/text, and
non-Latin text; it does not invent timestamps or confidence. A regression test failed with
`invalidOutput` before the fix and passed afterward for quotes and Western/CJK sentence punctuation.

The current permission check agrees with [Apple's documentation](https://developer.apple.com/documentation/speech/asking-permission-to-use-speech-recognition):
the legacy authorization request concerns `SFSpeechRecognizer`; this helper uses local
`SpeechAnalyzer` modules and reads an existing file without recording microphone input.

Validation:

- Regression failure: `/private/tmp/vpt-native-punctuation-red.log`.
- Full Swift tests: 21 XCTest and 10 Swift Testing cases passed,
  `/private/tmp/vpt-native-helper-final-tests.log`.
- SwiftFormat and strict SwiftLint passed, `/private/tmp/vpt-native-helper-lint.log`.
- Release build passed, `/private/tmp/vpt-native-helper-release.log`. SwiftPM's nested sandbox was
  disabled within the existing tool sandbox for testing; release debug-symbol generation required
  the approved execution outside the tool sandbox after `dsymutil` was denied.
- A generated one-second silent WAV was used only to probe availability. Inside the sandbox the
  helper reported an unsupported locale. The approved host probe returned exit 1, empty stdout,
  and `Apple speech assets are not installed for the requested locale`, recorded in
  `/private/tmp/vpt-native-helper-host-smoke.log`.

The user approved installing the en-US speech model outside the repository. The first installer
attempt returned no installation request; a subsequent status query reported the model installed
(`/private/tmp/vpt-speech-asset-install.log`). That evidence establishes availability, but does not
establish whether this attempt downloaded new model bytes or reused existing system assets.
No personal audio was accessed.

## Native helper locale reservation and successful acceptance

A real release-command test still failed with missing assets after installation was confirmed
(`/private/tmp/vpt-native-spoken-smoke.log`). A separate API diagnostic found an empty reservation
list and module status `supported`; reserving en-US changed that status to `installed` immediately
(`/private/tmp/vpt-speech-availability.log`). Apple's
[asset inventory documentation](https://developer.apple.com/documentation/speech/assetinventory)
describes the locale reservation step.

The helper now checks that the requested supported locale is installed, reserves it, and then checks
the module status. Asset preparation lives in `AppleSpeechAssets.swift`. No download API is called
by the helper. Untimed punctuation remains preserved without fabricated word timing.

Validation after the fix:

- Generated a 2.67075-second mono 16 kHz WAV with macOS `say`, containing “The quick brown fox jumps
  over the lazy dog.” The sandboxed synthesis attempt produced an empty file; the host attempt
  produced valid audio. No microphone or user recordings were used.
- All 32 Swift tests passed (22 XCTest plus 10 Swift Testing), including the new opt-in native test:
  `/private/tmp/vpt-native-reservation-tests.log`.
- SwiftFormat and strict SwiftLint passed: `/private/tmp/vpt-native-reservation-lint.log`.
- Release build passed: `/private/tmp/vpt-native-reservation-release.log`.
- The rebuilt release command returned exit 0, empty stderr, and one `vpt.engine/1` document. It
  transcribed the expected sentence into nine words with finite confidence in [0, 1] and increasing
  ranges bounded by the audio duration. This measured invocation took 1.36 seconds; that is one
  short-file observation, not a general performance guarantee.
- The release executable also refused an unsupported locale and a missing file with exit 1, empty
  stdout, and safe diagnostics; its version command returned `vpt.helper/1`.
  Evidence: `/private/tmp/vpt-native-spoken-final.log`.

The opt-in XCTest uses `VPT_TEST_APPLE_SPEECH_AUDIO` and skips when no generated fixture is provided.
Existing tests cover unsupported platform/framework/assets and safe error mapping; those conditions
were not all reproduced on the current host. The native helper acceptance row is now checked, so
Stage 2 has no remaining rows. With the three ingestion rows accepted in this continuation, the
roadmap now has **82 checked and 25 unchecked items**. Doctor timing and shared publication race
conditions remain open.

Final Rust formatting check passed (`/private/tmp/vpt-ingest-native-format.log`), and
`git diff --check` passed.
