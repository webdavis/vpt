# VPT remaining features

This is the remaining product scope for the Voice Processing Tool (VPT), including partially
implemented capabilities. The source baseline is `3bac4e5` on `main`. An unchecked item means the
complete user-facing capability remains unfinished; some of its supporting code already exists.

The detailed product specification is
[VPT design](superpowers/specs/2026-09-20-vpt-design.md). Its database protection requirements are
extended by [database containment](superpowers/specs/2026-09-22-vpt-database-containment-design.md).
The feature list below incorporates the later logging, recovery, notification, and standalone-product
decisions.

## Current product boundary

Configuration loading and setup, protected filesystem access, the SQLite ledger, archive operations,
publication recovery, process execution, logged events, report/alert hooks, the Swift Trash helper,
and the `doctor`, `show`, `list`, `storage`, `symlink`, and `--version` commands already have
implementations. Ingestion, transcription, transcript-note writing, review/resolution, known-term
confirmation, audio retention, and the initial `run` pipeline also have command implementations.
Unchecked items below identify remaining acceptance checks and gaps. Standalone analysis now
creates analysis notes and applies proposal tags and related links. Automatic
analysis and stale-finding rechecks now run in the pipeline, and analysis emits review notifications.
Configuration-driven analysis state changes and interrupted run outcomes are also implemented.
Occasion identity persistence, `occasions`, and saved brief-path lookup now have implementations.
On-demand and automatic pipeline redaction now create released copies and private reports with durable
recovery. `vpt show --stage` now returns each stage's text for people and programs, with `--redact`
masking it at call time.
Remaining work covers ingestion/logging acceptance, command timing, and shared boundary guarantees.

All source changes from the implementation worktrees are preserved in `main` and its history.
The main implementation locations are `crates/` and `helper/vpt-macos/`.

## Stage 1: complete extraction and audio retention

Product sources: specification sections 4, 5, 8.7, and 9.

### Ingestion

- [x] Complete `vpt ingest [--dry-run] [--once <path>]`, connecting the existing source, archive,
  ledger, history, and reporting capabilities to the command. A normal sweep considers only
  immediate `.m4a` children of the configured Voice Memos Recordings directory; sidecars and
  nested directories are excluded.
  Command tests cover immediate-file selection, exact source-tree preservation, archive/ledger
  results, logged history, safe export warnings, and independent deferred-alert delivery.
- [x] Complete candidate selection and deferrals: never open an iCloud dataless entry, skip an
  unchanged already-ingested entry, refuse symlink/nonregular source paths, apply the audio-size
  ceiling before reading content, require the complete media container defined in specification
  section 5.2, and enforce the quiet period and unchanged size after an earlier deferral. The defaults
  are 2 GiB and 30 seconds.
  Adapter probes verify no open/read before the relevant dataless/size gates and exact rest
  boundaries. Command cases verify matching source gates in ordinary and observational `--once`.
- [x] Persist each deferral reason and consecutive count, retry on later sweeps, and emit one
  `deferred` alert when the configured threshold is reached, default four. Include changed-during-read
  and staged-container failures, with safe source-path metadata and no audio content in events.
  Separate CLI runs verify default-four/configured-two thresholds, one alert across further retries,
  unchanged dry-run state, and clearing deferrals after a repaired file becomes stable. Adapter
  faults cover changed-during-read/staged failures and atomic seen/history/event persistence.
- [ ] Complete descriptor-based staging and publication: derive capture time, duration, full digest,
  and identity from staged bytes; retain private file permissions; publish without replacing an
  existing archive; and save the recording only after the archive and directory are durable.
  Cross-volume copies preserve the same behavior and report that the archive is not copy-on-write.
  Container inspection now rehashes its retained descriptor and requires the staged size/digest to
  match. Publication now carries that inspected descriptor and the staged size/digest through file
  sync and exclusive rename. It verifies the retained file before and after file sync and verifies
  the final archive after directory sync. Detected changes preserve the files and refuse recording
  rows; file sync uses the inspected descriptor. Replacement between the final pre-rename check and
  rename remains an open boundary, so this row remains unchecked.
- [x] Complete duplicate handling across source renames and time-zone changes. Reuse the identity and
  pinned paths for a known digest, update the source association, and report already-ingested work
  without creating another recording.
  Command acceptance covers UTC-to-+14:00 renames, unchanged archive bytes/path/identity, updated
  source association, redundant-stage Trash, and a later edit producing a distinct recording.
- [x] Recover an existing matching archive when recording or source rows are missing. Validate its
  container and full digest, recover the rows, and move the redundant staged copy to Trash. Different
  content at the expected archive path is `archive_collision` and remains untouched.
  Recovery now validates a stable descriptor and synchronizes against the expected full digest.
  Tests cover source-row recovery, private duplicate cleanup, collision output/history, and changed
  content refusing both recovery and reassociation before commit.
- [x] Recover valid orphaned archives at sweep startup, including when the original source is gone.
  Preserve the capture instant and offset represented by the archived identity; do not invent a new
  identity using the machine's current time zone.
  Command acceptance now covers recovery with and without a surviving source under a different
  time zone. Container inspection and hashing now share one retained descriptor and file-version
  checks; replacement and in-place-edit regressions refuse inconsistent recovery before commit.
- [x] Mark vanished source paths with `source_gone_at`, retain their archive, and expose the condition
  in `doctor`. An edited or trimmed recording becomes a distinct recording; a renamed source with
  identical bytes retains its identity.
  Ingestion/liveness tests preserve archived work, and doctor source-count acceptance now passes in
  both debug and release builds at its unchanged 500 ms deadline. The separate debug doctor
  active-writer timing failure remains under the doctor row below.
- [x] Complete the sweep's abort behavior for unreadable sources, an unexpectedly empty previously
  populated source, no space, and failed synchronization. Preserve and report recordings already
  committed before an abort. Title-copy failure continues with an unavailable title.
  Three-record fault tests preserve the first commit and stop before the third on later source,
  space, or sync failure, including unavailable cleanup helpers. Command tests verify empty-source
  JSON/human errors, one failure event per run, retained archives, and invalid-title-database fallback.
- [x] Complete `--once <path>` with the same safety and eligibility rules as a sweep, and complete
  the observational dry run with no clone, title-copy refresh, history, event, migration, or durable
  write. Known titles come from existing ledger state; otherwise the title is unavailable.
  Selection/refusal and full-snapshot command tests complement adapter checks for known-title
  lookup without refresh. Existing dirty-publication/retention/engine-output dry runs remain covered.
- [x] Complete ingestion startup under the write lock, including unfinished publication repair,
  pending retention reconciliation, and logging recovery before new mutating work. Failed staging
  goes to Trash when possible; if the helper is unavailable, retain the staged file and report it.
  Startup repairs saved publications, pending retention, and logs before new ingestion. A missing
  source no longer prevents recovery; the final source error includes completed recovery identities
  and each safe warning once. Command and adapter acceptance cover interruption, replacement refusal,
  unavailable cleanup helpers, and unchanged dry runs.

### Logged history and command output

- [x] Connect the existing logging implementation to every unfinished mutating command. Save a run
  start before its work, save state changes and corresponding history together, and distinguish an
  external-effect intent from a confirmed result. Preserve known completed work on later failure.
  Manual and upcoming briefs now save and export their run start before context collection or store
  creation; failed and interrupted collection records the corresponding run outcome. Required-history
  refusal and exporter recovery cases are covered.
  All 17 ledger-backed mutating forms pass the run-start guard matrix in both output modes. Shared
  memory/durable ledger contracts cover atomic work/history persistence, planned versus confirmed
  effects, and rollback; the report maps the evidence and records the separate timing failure.
- [x] Export new history to private `<state_dir>/events.jsonl` after each history commit. A log-export
  failure preserves the operation's result and exit status and adds a visible warning to the human
  output or existing JSON (JavaScript Object Notation) result/error. Failure to save required
  state/history, or an unsafe state root, stops new work.
  Exporter tests cover ordering, private output, and typed failures. Command matrices preserve success
  through an unavailable log and replay pending records with their original identities and times.
  Transcription and retention now retain one export warning when startup also refuses a modified
  publication target. Unsafe-state and required-history guards cover every mutating form.
- [x] Complete startup recovery that restores pending log records and appends an evidence-based
  summary of affected runs, observed logging failures, recovered records, completed/failed work,
  and unresolved steps or detectable missing history. Retain original occurrence times separately
  from recovery times, stable record identifiers, and sequence numbers. Resume interrupted recovery;
  never invent an outage cause, repair time, operator action, or an attempt with no surviving evidence.
- [x] Preserve recognizable record identifiers after an append succeeds but its acknowledgement
  fails. Handle incomplete final log lines, and acknowledge only complete durable records. Recovery
  neither repeats the underlying work nor resends hooks, and recovery summaries produce no hook.
- [x] Connect independent `[report] command = [...]` and `[alert] command = [...]` hooks to the
  remaining producers. Both default to empty lists, so the default is logs only. Reports cover routine
  summaries; alerts cover failures or items requiring the user's attention. Each configured command
  receives one `vpt.event/1` JSON document on standard input and owns notification formatting/delivery.
- [x] Preserve operation results when a hook fails; log the bounded failure without retries,
  cross-channel fallback, or native notifications. A log-only export failure still permits the
  original configured hook. Events expose safe identifiers, paths, classes, and counts, never
  transcript text, titles, tags, raw child output, secrets, or redaction mappings.
  Command acceptance covers a blocked exporter, failed alert, persisted replay, duplicate event IDs,
  incomplete-tail repair, and unchanged dry runs. Adapter tests cover interrupted recovery progress,
  missing evidence, durable append acknowledgements, and the real five-second hook deadline.
  Ingestion, pipeline, transcription, notes, analysis, confirmation, review resolution, redaction,
  and both brief-writing paths emit `config_refused` for store-creation failures after safe state
  validation. Refusals before root validation remain output-only.
- [x] Complete ingest/retention JSON envelopes and human summaries, including safe warnings and
  committed recording identifiers on partial failure. Config refusal before a safe state directory
  exists uses command output only. Observational commands and dry runs do not create or repair logs.
  Ingestion and retention command suites cover partial work, missing helpers, unavailable logs,
  failed hooks, broken final output, and human/JSON summaries. The expanded command run also verifies
  every event kind's producer and channel.

### Audio retention

- [x] Complete `vpt retention run [--dry-run]` command behavior over the existing audio-retention
  implementation. Retention defaults off, audio defaults excluded, and each hold time defaults to
  `0` (never expire). Expiration uses the file's modification time.
- [x] Move only ledger-owned, unchanged expired audio through the Swift Trash helper. Preserve
  untracked files, changed content, redirected paths, and files outside the configured authority,
  with an explicit kept reason. A missing helper refuses removal; no artifact is unlinked.
- [x] Save the retention intent and corresponding history before the move, then save the completed
  move and expiration state together. Preserve actual completed moves if a later state write fails.
- [x] Reconcile an interrupted or unknown move: an absent path completes expiration, an unchanged
  original may be moved, and replacement content is refused. Preserve the first expiration time and
  distinguish completed reconciliation from a newly performed move.
- [x] Emit one combined retention report for recovered and newly completed moves, with accurate
  per-store move counts, kept reasons, partial progress, logged history, and only the optional report
  hook. Dry run reports the candidates without taking a mutating lock or changing anything.

## Stage 2: transcription and uncertainty

Product sources: specification sections 3.4, 6, 8.7, 9, and 10.

### Engines and transcription commands

- [x] Add `vpt transcribe <id> [--language <tag>] [--dry-run]`. Use archived audio without transcoding
  it first, respect the selected language, and preserve the audio and prior accepted transcript if
  a later transcription attempt fails.
  Command acceptance verifies byte-identical private working copies, regional language arguments,
  observational dry runs, archive protection, and preservation of accepted state after engine failure.
- [x] Support a configured main engine and optional checker, with `checker_runs = always|on_failure`,
  per-language main/checker overrides, and sequential execution. Defaults are Apple as main, no
  checker, language `en-US`, and a 1,800-second per-engine deadline.
  Tests exercise all eight policy/outcome combinations, exact invocation order, checker skipping,
  language-specific selection, defaults, and deadline propagation and enforcement.
- [x] Reject two engines of the same model family before starting them unless `allow_same_family`
  is enabled. Expose each engine's local/cloud posture; local is the default and sending audio to a
  configured cloud engine remains the user's choice.
  Refusal now names both configured engines and their shared family. Command tests prove refusal
  before mutation/spawning, explicit opt-in, and declared locality in the selected-engine plan.
- [x] Extend `vpt-macos` with `transcribe <audio-path> --locale <tag>` using Apple's
  `SpeechAnalyzer`/`SpeechTranscriber`, with per-word time ranges and confidence. Report unsupported
  macOS/framework/language availability and required speech permissions. The helper remains a Swift
  command-line executable alongside its existing version and Trash commands.
  Native acceptance transcribes generated en-US speech into nine timed words with confidence.
  The helper reserves an already-installed locale before checking module status; missing assets
  still refuse without a download. Swift tests and release-command checks cover structured output,
  unsupported locales, unavailable assets/frameworks, invalid output, and safe failure messages.
- [x] Add the Whisply adapter using its configured executable arguments, device, model, and language.
  Read its generated JSON file and normalize it to VPT's transcript model. The default device is
  `mlx` and model is `large-v3-turbo`; absent MLX confidence remains absent, while reported
  CPU (central processing unit) scores may supply confidence. Do not interpret an echoed language
  option as independent detection.
  Fixture-process command tests cover default MLX/model options, configured CPU/model options,
  generated-file decoding, regional-to-base language mapping, exact audio bytes, and saved confidence.
- [x] Add generic command engines with declared family and locality, direct argument substitution
  for `{audio}`, `{language}`, and `{out_dir}`, and `vpt.engine/1` on stdout. Their text has no
  VPT lexical uncertainty classification, including when such an engine occupies the checker slot.
  Acceptance covers literal argument boundaries, output identity/locality validation, generic record
  exemption, and a generic checker preserving builtin lexical flags while retaining language diagnostics.
- [x] Implement bounded `vpt.engine/1` decoding: engine/tool/version/model/family metadata, language,
  locality, ordered timed segments and words, optional confidence, valid finite time ranges, and
  confidence in `[0, 1]`. Refuse unknown major versions, malformed/empty transcripts, and invalid
  identity or range data; retain diagnostics for unknown additive fields.
- [x] Keep the successful main engine's transcript as the record. If only the checker succeeds, use
  it and add `single-engine`; otherwise diagnose any one-engine-only result as specified. If both
  fail, retain earlier accepted state, mark transcription failed, and emit `transcribe_failed`.
  Independently detected language disagreement adds `language-mismatch`.
  The full command policy/outcome matrix verifies the accepted engine, fallback provenance, diagnostics,
  private failure events, and unchanged earlier state after both engines fail.
- [x] Persist the accepted transcript and timings, engine metadata, raw documents at
  `engine-outputs/<id>.<engine>.json`, flags, language, and stage state. Publish from saved state so
  interrupted artifact publication does not require rerunning an engine.
  An acknowledgement-failure test deletes the pending output and audio, then recovers exact saved
  output and publishes the note without another engine invocation. Both ledger backends cover rollback,
  accepted/raw consistency, private provenance, and guarded replacement of previously published output.

### Alignment, flags, and readable text

- [x] Implement Unicode compatibility normalization and full case folding for alignment, punctuation
  removal, digit-group joining, and ordinal-suffix normalization. Retain a mapping to original word
  positions; known-term matching uses intact whole tokens rather than punctuation-stripped substrings.
- [x] Align main and checker word streams, retaining original text and time ranges. A divergence
  ratio above `reconcile.max_divergence_ratio`, default `0.35`, produces a whole-recording `divergent`
  diagnostic while confidence and risk classification still operate.
- [x] Classify and rank lexical flags: unsurfaced formatting differences, numeric disagreement,
  proper-noun disagreement, low confidence below the default `0.35`, agreed-but-unverified risk
  spans, and other disagreement. Risk spans include proper nouns, numbers, dates, times, and money.
  Acceptance covers all six classes and their precedence, ranked command output, hidden formatting,
  repeated occurrence ranges, confidence thresholds, and the specified risk categories. English risk
  vocabulary now includes later ordinals and trillion; detection heuristics are documented in README.
- [x] Suppress agreed-unverified flags only for exact whole-token entries in `known-terms.txt`.
  Aggregate repeated surface forms for display while preserving each occurrence. Keep whole-record
  diagnostics distinct from lexical spans, with stable identifiers that preserve unchanged resolutions
  on re-transcription.
- [x] Add the optional readability pass, off by default: sentence capitalization, segment-ending
  punctuation, and the specified filler removal. Modify rendered note text only, preserving every
  flagged span and the accepted/raw transcript.
  Command tests toggle the setting from its default, preserve every class's markers and both raw
  engine documents, and restore the original content digest when disabled. Note and pipeline refreshes
  use saved words without engines; domain tests protect formatting/resolved spans and Unicode ranges.
- [x] Produce the first transcript note with the selected portable or Obsidian profile, full required
  frontmatter, filename-matching heading, separate managed links region, and timecoded content region.
  Render uncertainty and correction markers inline without a separate uncertainty-summary block.
- [x] Emit privacy-filtered `review_needed` only after publication, with identity, path, and class
  counts. Apply the configured aggregation threshold, default three recordings, for one aggregate
  alert when the threshold is exceeded; keep transcript text and titles out of events.

### Transcript corrections and known terms

- [x] Add `vpt review <id>` to display ranked open flags, record text, alternatives, and timecodes.
- [x] Add `vpt review <id> --resolve <flag> --confirm|--correct <text>|--dismiss`. Persist the
  resolution, update counts and rendered notes, and keep corrections separate from the immutable
  accepted word stream. Preserve unchanged resolutions when transcription is repeated.
  The command now updates transcript and existing analysis notes, including verification annotations
  and counts. Transcript or lexical changes now invalidate saved findings; stale decisions remain
  available for rechecking unchanged claims. Lexical corrections update both notes and counts in the
  same transaction while preserving the accepted word stream. Owned transcript verification now
  preserves lexical correction markers and distinguishes finding corrections through saved claim
  identity and an invisible role marker. Rechecking and rewriting retain both kinds of corrections,
  including when their visible text is identical.
- [x] Add idempotent `vpt confirm --term <text>` and the specified automatic known-term additions
  from proper-noun/agreed-unverified confirmations and corrections.
- [x] Define the remaining locale-mapping details for backend language tags versus regional tags,
  and the divergence-ratio denominator for unequal word-stream lengths. Specification sections 6.1
  and 6.4 retain regional tags for Apple and commands, map Whisply to supported base codes, and use
  the longer normalized stream as the denominator. Backend implementation remains tracked above.

## Stage 3: notes, tags, relations, and the initial pipeline

Product sources: specification sections 3.3, 4.3-4.5, 7, 9, and 10.

### Owned notes and paths

- [x] Complete both `portable` and `obsidian` note profiles and independently selectable Markdown or
  wiki link style. Include the profile's exact provenance, engine, language, uncertainty, and VPT
  identity/stage fields. Obsidian notes also carry aliases, reference/hub links, status, description,
  and the local first-write start date.
  Both note types now include the audio reference in their managed links, with root-aware wiki paths
  and Markdown fallbacks. Acceptance covers all four profile/style combinations and exact metadata.
- [x] Maintain separate `vpt:links` and `vpt:content` regions. Preserve operator text outside them,
  operator tags, and the specified editable frontmatter fields. Escape external strings for Markdown
  and frontmatter; reject missing/duplicate markers, foreign identities, and unsupported schemas.
  Tests cover both-note regeneration from saved content, operator prose before/between/after regions,
  YAML comments and editable values, escaped external text, and atomic refusal when either note is invalid.
- [x] Pin flat note paths at first publication as `{capture-date}-{slug}-{hash8}.md`. Derive the slug
  only from the Voice Memos title, with Unicode normalization/case folding, safe characters, collapsed
  hyphens, the configured length limit (default 60), and `untitled` fallback. Later title or settings
  changes do not rename an existing note.
  Command acceptance covers nine title/limit cases for both stores and retains both pins after title,
  slug-setting, and tag changes. Domain tests also enforce input bounds and invalid-limit refusal.
- [x] Add `vpt note write <id>` to render existing ledger state into the transcript and, when present,
  analysis note. It also provides deliberate regeneration after expiration; automatic runs leave
  expired artifacts expired and do not regenerate artifacts with pending retention intents.
  Shared preparation now covers existing analysis notes, reciprocal links, saved finding resolutions,
  and atomic two-note updates. Stage checks prevent metadata publication from completing stale analysis.
- [x] Add read-only `vpt path <id> --stage <stage>`, returning the pinned absolute path even if its
  artifact is currently missing.
- [x] Recover a uniquely renamed owned note by its recording identity during a mutating operation.
  Update the pinned path and log the move once. Refuse ambiguous claimants, foreign occupants,
  symlinks, and escaped paths; recreate a missing note only when its state permits regeneration.
  Acceptance covers both note types, reciprocal links, one-time move history, unchanged operator edits,
  ambiguous/redirected claimants, changes during discovery, and expiration-aware regeneration.
- [x] Build a per-run read-only index of Markdown names, aliases, and tag vocabulary from the
  configured Obsidian vault or portable transcript/analysis/brief stores, including untracked notes.
  Do not follow symlinks or treat indexed external notes as VPT-owned.
  The bounded recursive index, profile root selection, manual tag consumer, and automatic mention
  derivation are implemented. Analysis now uses that same index for tag vocabulary and related targets.
  Acceptance covers refreshed names/aliases/tags, nested untracked notes, overlapping roots, safe
  malformed-metadata diagnostics, symlink cycles, resource limits, and unchanged external file bytes.

### Tags and relations

- [x] Implement normalized tag vocabulary and `known-tags.txt`: case/separator variants reuse an
  existing spelling; multi-word tags become kebab-case; accepted syntax contains letters, digits,
  underscores, hyphens, or slashes, has a nonnumeric character, and has no leading `#`.
- [x] Implement `tags.new_tags = hold|write`. The default `hold` keeps unknown tags in
  `vptSuggestedTags`; `write` confirms and records them automatically. Deduplicate in proposal order,
  default to five confirmed tags and ten suggestions, and preserve the operator's tags.
  Typed settings, selection rules, durable tag state, and transcript-note rendering are implemented.
  Analysis now applies proposals in order, preserves manual decisions, journals automatic known-tag
  additions with the notes, and logs syntax/cap rejections without private tag text.
  Acceptance covers default and configured caps, normalized duplicates, first-proposal provenance,
  both profiles and note types, retained operator tags, and recovery of automatic vocabulary writes.
- [x] Add `vpt confirm <id> --tag <tag>` and `--reject-tag <tag>`, with durable tag state, known-tag
  updates where appropriate, and note changes. Manual confirmation works without an analysis command.
  Both commands now update durable vocabulary and available notes atomically. Expiration is checked
  per artifact; a live analysis can update while its transcript remains expired.
- [x] Derive `continues` from the immediate predecessor by capture instant then identity, provided
  its end precedes the current start and the gap is within `relations.session_gap_minutes`, default
  60. An overlapping predecessor does not cause a search for an older substitute.
  Shared note preparation derives, saves, and renders continuation links. The pipeline refreshes
  completed notes when a late-arriving recording changes the predecessor, without retranscription.
- [x] Derive `mentions` from confirmed whole-token terms that uniquely identify an indexed note name
  or alias. Preserve all source ranges; ambiguous matches produce no link.
  Note publication derives links and ambiguity ranges from accepted words and the per-run index.
  Review confirmations include newly learned terms in the same note update. The pipeline refreshes
  changed matches without retranscription; manual relations and transcript content remain intact.
- [x] Represent and render `related` suggestions with provenance and the configured cap, default
  five. Analysis proposals supply these later. Allow `supersedes` only through explicit
  `vpt confirm <id> --relation <kind>:<target>`; support the four specified relation kinds.
  Both ledger backends now save typed relation state, private provenance, derivation rules, and
  mention ranges atomically with history and note publication. Saved related links render escaped
  provenance with the configured suggestion cap. Explicit relation confirmation now supports all four
  kinds, preserves proposal provenance, and commits changes to both existing notes with history.
  Analysis now selects related suggestions through the shared target resolver and configured cap.
  Acceptance covers default, reduced, and zero caps; duplicate, missing, ambiguous, and self targets;
  preserved confirmations; escaped provenance in both notes; and rejection of non-related proposals.
- [x] Render deterministic managed links, including the later transcript/analysis relationship,
  without editing any external note that VPT merely indexed. Filing remains flat per store.
  Both existing notes render saved relations and reciprocal links with either configured link style.
  Rename recovery updates their links together, and initial analysis publishes both notes. Wiki links
  retain exact paths from the vault root or, for portable notes, the common parent of transcript,
  analysis, and brief stores. Outside-root targets and unsafe wiki names use encoded Markdown links.
  Tests cover both profiles/styles, matching filenames, renamed targets, duplicate indexed names,
  deterministic ordering/caps, bounded escaped output, and unchanged external note bytes.

### Pipeline and visibility

- [x] Add `vpt run [--dry-run]` initially composing ingestion, transcription, and note publication.
  Resume each recording from its earliest enabled pending/failed stage. Re-enable previously disabled
  incomplete stages when configuration enables them; preserve earlier success and expiration.
- [x] Keep the exact rendered transcript content-region bytes and digest available as analysis
  input. Corrections and readability changes affect that input; link-only/frontmatter/operator-prose
  changes outside the content region do not.
- [ ] Extend `doctor` for engine availability/locality and the configured Obsidian vault, and expose
  the placement of audio, engine outputs, and private redaction reports inside Git-tracked trees.
  These observations never alter the vault or its ignore files.
  Engine executable checks and declared locality now cover default and language-specific selections;
  unused definitions are informational. Vault/Git checks and filesystem-preservation tests exist.
  All 38 release tests pass. An earlier debug run passed 37 cases and exceeded 500 ms with empty
  output during an active native writer. All 38 debug cases
  pass in the current configuration-privacy run; the intermittent timing cause remains unresolved.
  Later review-command suites also have intermittent 500 ms failures with empty output; focused
  reruns move the failure between cases. One durable verification contract also timed out after
  entering its child scenario, then passed its focused rerun. These timing failures remain unresolved.
  Verification-child diagnostics now time database opens, operations, closes, and file/directory
  synchronization; all 828 database/ledger cases pass with these diagnostics. A combined sandboxed
  command run passes 18 review cases but still times out in the active-writer doctor case. Temporary
  probes observed both a pre-main timeout and a separate timeout after entering the helper version
  check. They do not establish a common cause; the probes were removed and deadlines are unchanged.
  A subsequent host-access run also times out in that doctor case and two review cases (53 passed,
  3 timed out overall), so the managed sandbox alone does not explain these failures.
  A live sample of a later failed review command reached WAL-index recovery during history export.
  Writable ledgers now retain one idle observational SQLite connection so per-operation connections
  reuse the initialized WAL index. Regression tests prove index reuse, released transaction/read
  locks, native checkpoint/write compatibility, and recovery after process termination. Subsequent
  connections also check the main/WAL/shared-memory identities against the retained descriptors,
  refusing regular-file replacement or removal before writes or recreation. All 841 database/ledger
  tests pass, including 13 focused lifetime/acquisition cases. The initial lifetime change passed all
  653 command tests (two installation cases ignored), including 38 doctor and 18 review cases at
  unchanged deadlines. The earlier pre-main/helper delays remain unexplained, so timing acceptance
  stays open. The final identity protection also passes all 70 doctor, review, configuration, and
  configuration-refusal command tests.
  A later database/ledger run passed 838 cases and hit three timing failures during synchronization
  or owned-child cleanup. Already-private files now avoid redundant chmod on each open; the timing
  cause remains unresolved, and all original deadlines remain in force. All 842 database/ledger
  cases pass after the permission change and the missing-file existence-query refusal.
- [x] Extend `show`, `list --stage`, and `storage` to expose the newly implemented stage states,
  engine/language metadata, flags, note paths, and artifacts while remaining observational.
  Show/list now read consistent, bounded metadata snapshots including current flag counts, stale
  verification stages, note pins, and publication/expiration status. Pipeline acceptance covers
  real expiration, current/stale flags, and pending publication without writes. Storage covers all
  seven stores. All 31 command tests now pass in both debug and release builds, including the
  active-writer case, with unchanged deadlines and fixture snapshots.

## Stage 4: analysis, briefs, sharing, and handoff

Product sources: specification sections 3.3, 8, 9, and 10.

### Analysis step and analysis notes

- [x] Add optional `[analysis]` settings and `vpt analyze <id> [--dry-run]`. An empty command
  disables analysis; explicit invocation without one is a configuration error. No model vendor is
  built in. The default command deadline is 600 seconds.
  The CLI supports standalone execution, safe failures, bounded deadlines, and observational dry runs.
  Automatic pipeline execution and analysis review notifications are tracked separately below.
- [x] Run the configured argument list with `{transcript}`, `{id}`, and `{language}` substitution.
  Send the exact transcript `vpt:content` bytes on standard input and retain their digest.
  The command adapter performs bounded, single-pass substitution, streams the supplied content,
  and returns its digest with the decoded proposal. The CLI publishes the current transcript first,
  checks source ownership/content before and after the agent, and refuses edits made during execution.
- [x] Decode bounded `vpt.proposal/1` output containing ordered summary/action claims with source
  ranges and proposed tags/relations. Save accepted proposal state and provenance; invalid output
  or a failed command preserves prior accepted state.
  The bounded wire decoder and conversion to domain proposal values are implemented. They preserve
  claim order and declared citations for grounding, and admit only `related` relation proposals.
  Both ledger backends now save accepted proposals, input and grounding digests, resolved source
  ranges, inherited uncertainty, and the analysis publication record atomically with history.
  Acceptance refuses stale inputs or forged grounding; failed attempts preserve prior accepted
  state. Artifact-specific verification findings and their resolutions are now stored by both ledgers.
  Markdown extraction, annotation editing, and `verify-note` are implemented. Standalone analysis
  commits these outputs together and repairs pending note or vocabulary publication without rerunning a current agent
  result. Bad agent output and late transaction failures preserve prior accepted state.
  Note-input snapshots now include accepted proposals alongside transcript, flags, and owned notes;
  SQLite reads them in the same transaction and refuses corrupt proposal state. The read-only runtime
  delegates proposal and grounding-basis reads to the selected ledger.
- [x] Render the owned analysis note with Summary and Actions sections, escaped claim text, source
  citations, inherited uncertainty, profile frontmatter, and reciprocal transcript/analysis links.
  Route tag and relation proposals through their existing product rules. `note write` rerenders
  both notes from saved state.
  The bounded content renderer now writes ordered Summary/Actions bullets, escaped claims and
  citations, and grounding annotations. The shared owned-document renderer supports transcript and
  analysis stages in both profiles. Generated analysis documents pass ownership checks and preserve
  operator fields, tags, and prose when rewritten. Shared note preparation now updates both existing
  notes with reciprocal links, selected verification resolutions, and aggregate counts. Explicit
  regeneration restores expired analysis from saved content; automatic updates preserve expiration.
  Shared preparation can now create the first analysis note from checked content while updating the
  transcript's reciprocal link and aggregate finding count. Both profiles and link styles are covered.
  Standalone analysis now uses this preparation, including proposal tag/relation selection. Wiki
  counterpart links now distinguish identical filenames and preserve renamed targets. CLI acceptance
  covers both profiles and link styles, exact escaped section content, ordered frontmatter, preserved
  operator fields, and regeneration from saved state with the agent unavailable.
- [x] Recompute enabled analysis when the rendered content digest changes. Invalidate derived
  verification findings and inherited uncertainty when their transcript or lexical-flag inputs
  change; expired outputs remain expired during automatic runs.
  Both ledgers now retain verification input digests, claim fingerprints, and grounding bases, including
  empty result sets.
  Changed transcript or lexical inputs remove stale findings from active review and counts, retain
  their decisions for rechecking, and mark previously successful analysis pending. Note rewrites
  remove stale analysis annotations while preserving current lexical corrections. Changed claims,
  including readability changes, invalidate that artifact before note counts are rendered and saved.
  Fingerprints ignore verification annotations and nonclaim structure, so annotation-only updates
  keep their findings current. Schema 11 upgrades retain old finding rows and require rechecking when
  the original grounding basis, claim fingerprint, or grounding window was not recorded.
  Automatic recomputation and rendered-input comparison now run in the pipeline. Stale transcript
  findings are checked before agent input is selected; unchanged runs skip engines and agents.
  Automatic analysis rechecking also works with analysis disabled or the transcript expired.
  Configuration-driven disabled/pending states now commit with matching history. Completed and
  expired outputs are preserved, stale enabled inputs become pending before artifact work, and
  interrupted child and pipeline runs retain their interrupted outcome. Grounding-window changes
  trigger automatic rechecking, including empty finding sets and when analysis is disabled, while
  preserving unchanged review decisions and expiration. CLI acceptance now also confirms that a lexical
  decision refreshes inherited uncertainty once without retranscription, while operator prose does not
  rerun analysis. Both ledger implementations pass the verification freshness and empty-result checks.

### Note grounding

- [x] Add the product command `vpt verify-note <path> --recording <id>`. Read claims from Markdown
  list items and paragraph sentences, excluding frontmatter, headings, fenced code, link syntax,
  and existing annotations.
  The bounded Markdown adapter extracts list items and paragraph sentences, retains citation syntax,
  and excludes document structure and prior verification annotations. It supports whole external
  bodies and already-selected owned content regions. The command applies ownership and bounded I/O
  checks, and publishes owned-note annotations through the durable journal.
  Claims expose original byte ranges and one-based line numbers for command reports.
- [x] Classify claims in order as `unsourced`, `bad-reference`, `unsupported`, or
  `built-on-flagged-text`. Resolve time references against the recording; compare digits, money,
  dates, and capitalized whole tokens to the cited transcript plus `analysis.grounding_window_secs`,
  default 15 seconds. Preserve meaningful number/currency/date punctuation in grounding.
  The domain checker now validates citations, uses corrected transcript evidence, and keeps inherited
  open lexical uncertainty separate from the highest-priority finding. It bounds input, normalized
  tokens, and work across a proposal. Artifact-specific storage and Markdown extraction are implemented;
  generated analysis re-verifies with the same grounding results. `verify-note` uses this checker.
- [x] Report findings for readable external notes without modifying their bytes. Annotate only the
  correctly identified, ledger-owned artifact with valid markers and safe paths; a mismatched
  recording is an ownership refusal. Findings alone are a successful command result.
- [x] Persist finding identity by artifact, claim position, and annotation-stripped claim digest.
  Replace only that artifact's findings, retain resolutions for unchanged claims, update open counts,
  and annotate claim endings. Apply this grounding to new analysis before reporting analysis success.
  Both ledgers now replace findings per owned artifact, preserve unchanged resolutions, and expose
  findings through review snapshots. SQLite recounts all open lexical and verification flags; review
  output and transcript frontmatter include verification counts. Stable identity uses recording,
  artifact stage, claim index, and claim digest. Markdown annotation removal/application is implemented;
  both ledgers bind findings and annotation updates to the exact checked source. An explicitly supplied
  disk snapshot allows verifying edited claims; stale snapshots or unchecked claim replacements refuse.
  Proposal acceptance now derives findings from the actual rendered Markdown and saves them with the
  proposal and analysis publication in one transaction. It shares the checker with `verify-note`,
  preserves resolutions for unchanged claims, and leaves other artifacts' findings intact. Input changes
  now invalidate findings without losing their resolutions. Automatic rechecking in the pipeline and
  analysis notifications are now connected. Alerts retain one final count set per recording and
  aggregate across recordings; accepted publication recovery does not rerun agents or resend hooks.
  Saved checks now include the grounding window; window changes trigger rechecking without an agent
  when analysis is disabled. Legacy checks retain decisions but require rechecking, and migration
  rollback and corrupt-window refusals are covered. Acceptance checks cover late transaction rollback,
  acknowledgement failure/recovery, preserved committed identifiers, and log-export warnings. Accepted
  analysis also retains its vocabulary publication dependency, so vocabulary-only recovery preserves
  the accepted proposal without rerunning the agent or resending hooks.
  Annotation rendering now honors confirmed, dismissed, and corrected findings; correction markers
  preserve claim identity on re-extraction. Owned-document content replacement preserves frontmatter,
  managed links, and operator prose. These operations are connected to `verify-note`, review resolution,
  and subsequent note rewrites. Standalone analysis now uses the same checker before publication.

### Occasions and briefs

- [x] Add `vpt brief <occasion-id>`, manual `vpt brief --title <title> --at <RFC3339>` with duration,
  participant, and tag options, and `vpt brief --upcoming`. Persist stable occasion identities;
  provider keys survive title/time changes and manual identity is reproducible. Typed identities,
  canonical provider/manual keys, exact start times, and atomic occasion/history updates now exist
  in both ledgers. Rescheduling preserves the existing identity, pinned path, saved pack, and briefed
  marker. Manual and existing-ID commands now assemble, save, and publish packs; their previews are
  observational. The opt-in upcoming command now collects configured context once, publishes each
  unbriefed identity once, and preserves provider labels for confirmed-tag evidence selection.
  Repeated invocations and rescheduling retain the original brief and completion marker.
- [x] Add `vpt occasions` and read-only `vpt path --occasion <id> --stage brief`. Keep unbriefed
  occasions visible, pin each brief's path, and rewrite that same brief for a repeated request.
  Both observational commands now exist and list unbriefed occasions without creating state.
  Both ledgers pin the first accepted brief and enforce that pin on rewrites. Command tests verify
  repeated requests and provider rescheduling rewrite the original file and retain first-write metadata.
- [x] Select deduplicated transcript notes using confirmed participant, term, and tag matches,
  followed by recent continuation/shared-confirmed-tag matches. Rank by match class then newest
  capture, with defaults of 12 notes, 180 days of lookback, and five spans per note.
  The domain selector and quote assembler now implement these rules over supplied snapshots,
  including transitive recent matches, deduplication before limits, and verbatim accepted segments.
  Typed brief settings now carry configured limits and trigger timing into runtime settings.
  The command streams a consistent bounded ledger snapshot and reads current owned artifacts and
  the term/name index. It retains accepted quotes with current source digests, excludes missing or
  expired sources, and refuses unaccepted managed transcript changes.
- [x] Preserve rejected and omitted candidate/span counts. An unconfirmed participant term appears
  under Not selected with the applicable term-confirmation command. `--explain` describes selection
  and rejection without publishing a brief. Domain and command tests cover ambiguity, zero note/span
  limits, confirmation guidance, matched values, and unchanged ledger/store/history during explanation.
- [x] Persist each assembled brief pack with source identities and digests. Render the required
  occasion frontmatter and managed sections: Occasion, Unresolved, From your notes, Open tasks, and
  Not selected, including empty sections. Quotes are verbatim transcript spans with recording and
  timecode provenance; uncertain quotes carry `[unverified]` inline.
  Markdown rendering now produces these sections and metadata, sourced open tasks, escaped text,
  and portable/Obsidian links. The brief formatter validates ownership and preserves operator
  prose/properties/tags during rewrites. Both ledgers now save the complete evidence/context snapshot,
  exact rendered text, path pin, and publication journal atomically. Startup recovers saved brief
  bytes and marks the occasion briefed only after publication succeeds. Operator replacements refuse
  repair; completed occasion IDs survive later failures. Manual and existing-ID commands now use
  current-source snapshots and publish through this path. Acceptance covers all four profile/link
  combinations, exact confirmed and uncertain quotes with per-line provenance, empty sections,
  editable properties/tags/prose, and recovery of a populated pack without source files or context
  collection. Both ledger contracts, saved-pack decoding, and all brief command tests pass.
  The shared publication race conditions documented below remain open.
- [x] Emit `vpt.brief/1` with the occasion, unresolved issues, sourced items carrying certainty and
  sources, not-selected information, separate calendar/task context status, and generation time.
  Mark an occasion briefed only after durable publication. Requested briefs emit no event;
  successful automatic briefs emit one `brief_written` report.
  The bounded JSON encoder now requires certainty and sources and validates source paths/digests
  against the assembled pack. CLI output and saved publication are connected; requested briefs emit
  no event. Automatic publication commits one occasion-specific report event with the briefed marker
  after durable bytes exist. Tests cover report-hook input, rollback on completion failure, completed
  IDs after partial failure or broken stdout, and recovery without notification replay.
- [x] Support a wholly observational `brief --dry-run` that may collect the configured read-only
  context and returns the pack it would write.
  Both providers use the same read-only preview path. Command tests snapshot stores, ledger, and
  history to verify observational behavior, including configured collection and failure statuses.

### Calendar and task context

- [x] Add `context.type = none|dam|google` and bounded `vpt.context/1` normalization. Calendar and
  task halves independently report present data or absence with disabled/unsupported/failed reason
  and safe detail. A context collection has one 30-second deadline.
  Typed settings and bounded context encoding/decoding now exist, including provider identities,
  independent status, private errors, and additive-field diagnostics. Dam collection now uses this
  document and one shared 30-second deadline. Native Google collection now shares the same bounded
  normalization and deadline, including token refresh and every calendar page.
- [x] Implement `dam` collection through the configured command with `--json --no-pull`, restricted
  to configured calendar path prefixes and task project prefixes. Reject missing selections when
  the requested supported context half needs them. VPT does not use Todoist's command or `gog`.
  Manual briefs and previews now collect through the application context port. Queries include each
  selected path; invalid query syntax in a path refuses before state creation. Replies are bounded,
  scope-checked, deduplicated, and normalized; conflicting versions fail the half. Zoned/all-day
  events, organizer names, attendee emails, and open tasks are supported. Provider failures stay
  private and independent; interruption aborts. Tests cover the shared deadline, aggregate capacity,
  exact time-window boundaries, and observational previews.
- [x] Implement native Google Calendar reading over HTTPS (Hypertext Transfer Protocol Secure) with
  `rustls`, configured calendar identifiers, time-bounded expanded recurring events, pagination,
  a ten-page-per-calendar cap, and only the read-only events permission. Google task context is
  explicitly unsupported.
  Tests cover exact start-window filtering, stable calendar/event identities, all-day DST, cursor
  repetition, scope refusal before event requests, page limits, shared deadlines, interruption, HTTPS
  enforcement, bounded bodies, and sensitive authorization headers. No live account was accessed.
- [x] Preserve read-only context and secret handling: configured OAuth credentials remain private,
  no calendar or task writes occur, and failures reveal no token or provider response body.
  Google uses only token refresh POST and events GET against fixed HTTPS hosts, refuses redirects,
  and marks authorization headers sensitive. Dam uses only scoped read queries. Error tests cover
  credential preflight and fixed provider-failure details without copying private response text.
- [x] Add the opt-in calendar trigger, default off with 24-hour lead time. Select occasions in
  `now <= start < now + lead_time`. Disabled `--upcoming` is a configuration error; failed or
  incomplete discovery fails without marking occasions briefed. A manual brief can still publish
  while clearly showing a failed context half.
  Failed or disabled discovery exits 1 before state creation; disabled triggering exits 2 naming
  brief.trigger.enabled. Adapter tests cover exact window boundaries and incomplete collection;
  command tests cover durable publication, rescheduling, repeated invocations, and typed reports.

### Redaction and released copies

- [x] Add `vpt redact <id> [--stage transcript|analysis] [--to <directory>] [--title <title>]` and
  `[share]` settings. `share.automatic`, default false, creates releases after new transcript or
  analysis artifacts during `run`, independently of whether analysis is enabled. It does not
  regenerate the same release on every unchanged run. Audio and briefs are not redaction sources.
  All share keys now reach typed runtime settings, with bounded strings/lists, validated masks and
  pattern classes, and private diagnostic output. The on-demand command now loads both source
  stages, masks current terms/names, publishes both files, and emits human/JSON output. The pipeline now
  releases transcripts before the analysis step and analysis notes after publication/verification. Finalized matching
  content is skipped on unchanged runs; enabling sharing processes existing notes. Dry runs report
  pending redaction observationally, and failed finalization recovers without an extra sequence.
- [x] Use the configured released store or per-command destination, refusing destinations inside
  private stores, state, configuration, or Voice Memos, as specified in section 8.5. Preserve source
  artifacts and publish without overwriting an existing released file.
  Release filesystem preparation now resolves aliases, names the protected root on refusal,
  creates missing destination folders, and preserves existing shared-folder permissions. The command
  selects and publishes under the write lock, preserving source bytes and existing released files.
- [x] Construct minimal released frontmatter with the optional title/date and source statement.
  Strip private identity/vault metadata, managed links, link targets, denied sections, and timecodes
  by default. Parse Markdown and decode entities; remove raw HTML (Hypertext Markup Language),
  comments, and reference targets rather than leaving hidden private content.
  Pure preparation now extracts an owned note's managed content, removes hidden HTML, decodes
  entities (including code), strips link destinations and configured sections/timecodes, and escapes
  retained text. It builds minimal title/date/source frontmatter and rechecks residue including
  metadata. Both source stages now use this preparation in the command; title/date/source settings
  and source preservation have command coverage alongside the Markdown adapter tests.
- [x] Always mask confirmed terms and indexed note names, plus configured email, web-address, phone,
  four-or-more-digit number, and money classes. Apply deterministic longest-match precedence and
  masks numbered from one for each copy; the default mask is `[{class} {n}]`.
  The bounded domain redactor now implements Unicode case folding, possessives, deterministic
  longest-match selection, class precedence, Unicode digit/currency classes, per-copy numbering,
  private mappings, and residue errors that omit values. The command loads current confirmed terms,
  indexed names, and aliases, and refuses an incomplete index before publishing.
- [x] Omit and count open flagged transcript spans and analysis claims built on flagged text by
  default, or retain them explicitly marked `[unverified]` when configured. Report capitalized
  survivors as candidates. A residue scan of decoded output refuses publication if forbidden text
  remains, without exposing that text in the error.
  Pure preparation now binds open transcript flags to exact rendered source spans, refuses stale
  offsets, and applies analysis policy to whole saved claims. Omission counts original occurrences;
  marking retains accepted text and protects generated warning markers through masking. Candidate
  reports retain capitalized surviving tokens and release line numbers, excluding generated masks
  and discarded text. The command reports candidates and policy counts and refuses residue before
  reservation. Analysis claims are rechecked against current accepted words and resolutions without
  changing the source, so stale findings cannot omit a now-resolved claim.
- [x] Save a private mode-0600 report for each release, mapping masks and released lines to source
  spans. Keep that report outside the released copy and never expose mask mappings through logs.
  Bounded report preparation now records both digests, exact mask values, source byte spans, and
  explicit generated-line entries. The ledger saves report bytes privately with the pending copy;
  publication now writes the separate report with mode 0600 through a retained directory handle.
  Recovery checks owner, mode, link count, exact bytes, and durability before finalization.
  The command creates the report, prints only its path and aggregate mask counts, and keeps mappings
  out of events and errors. Report/source span preparation and mode checks have focused coverage.
- [x] Use standardized release names without the private note slug. Refuse byte-identical copies
  already anywhere in the destination; resolve different-content name collisions with a local
  timestamp and then the smallest available suffix. Released content remains immutable.
  Bounded destination selection now scans every direct regular filename for identical bytes and
  uses capture date, source stage, and hash without a private title. Collision tests cover local
  time, occupied directories/symlinks, and suffix gaps. Command tests cover an unrelated duplicate
  filename, repeated releases with changed content, and collision reporting in human/JSON output.
- [x] Persist release sequence, destination, source/content digests, private report path, and frozen
  provenance: capture time, engines, lexical and verification uncertainty, diagnostics, and reviewed
  state. A release becomes selectable only after both its copy and private report are durable.
  Both ledgers now reserve monotonic sequences, immutable destinations/report paths, digests, and
  frozen provenance with exact recovery payloads and publication intents in one transaction.
  Finalization requires the expected copy/report/source digests and commits completion/history
  together; finalized sequences alone are selectable. Reopen, rollback, corruption, and lost commit
  acknowledgement tests pass. Mutating startup now publishes saved copy/report bytes, synchronizes
  both files and directories, verifies digests and report privacy, then finalizes the release.
  Recovery honors saved override destinations and retains completed IDs on later failures.
  Command tests cover frozen provenance after review changes, failed completion followed by startup
  recovery, and completed IDs on broken stdout. Automatic creation also recovers saved bytes and keeps
  a completed transcript release when later analysis fails. Shared publication race conditions remain
  tracked below.

### Text handoff

- [x] Add read-only `vpt handoff <id> --stage released|transcript|analysis|brief`, emitting
  `vpt.handoff/1` with text, deterministic title, kind-specific identity, provenance, and generation
  time. Private stages require `handoff.allow_private`, default false.
- [x] For released handoffs, select the newest finalized existing release by sequence, retain its
  frozen provenance, and refuse changed bytes. Remove released frontmatter and prepend the
  declarative VPT handoff header. No release is an input error; only pending publication is an
  operational failure.
- [x] For private handoffs, use the owned artifact's current content and freshly computed digest,
  preserving operator prose outside managed regions. Briefs resolve an occasion and its source
  records. Unreviewed artifacts are labelled rather than refused solely for being unreviewed.
- [x] Keep handoff text-only: no notebook transport, upload, clipboard action, credential access,
  or file write occurs as part of the command.
  Eleven command tests cover all four kinds, current private prose and digests, immutable release
  provenance, missing/changed/pending artifacts, available-stage errors, and observational output.
  Protocol bounds, retained-file checks, and the production vocabulary independence test pass.
- [x] Fold `handoff` into `vpt show --stage released|transcript|analysis|brief`, returning the same
  text and provenance beside the structured fields with no opt-in, and add `--redact` for a masked
  view that saves nothing. The `handoff` verb, `vpt.handoff/1` and `handoff.allow_private` are gone.

### Full pipeline and retention across all stores

- [x] Extend `vpt run` with enabled analysis, independent automatic redaction, and opt-in retention.
  Its result lists ingested, transcribed, noted, analyzed, and redacted work plus retention
  outcomes. Resume the earliest pending/failed enabled stage and preserve committed partial results.
  The pipeline now invokes the existing journaled retention sweep after processing when enabled,
  merges recovered moves with new moves, and reports observational retention plans for all stores.
  Full pipeline acceptance now covers stage retries, configuration changes, committed partial results,
  independent sharing, later-store retention, startup recovery, and observational dry runs.
- [x] Extend the ledger from audio-only operations to accepted transcripts, proposals, flags, tags,
  relations, occasions/brief packs, engine outputs, redaction reports, and release provenance.
  Keep accepted render inputs after raw-engine-output retention so notes remain regenerable.
  Both ledger implementations preserve accepted transcripts, saved notes, brief packs, and release
  payloads through artifact expiration; migration, rollback, and reopen tests cover durable state.
- [x] Extend retention to ledger-owned engine outputs, transcript and analysis notes, briefs,
  private reports, and releases, using each store's hold time. Validate note/brief identity, stage,
  pinned path, and markers; validate other artifacts by digest. Preserve modified/untracked files
  and all expiration/recovery protections established for audio.
  Both ledgers now expose bounded ownership metadata for all seven artifact kinds, including
  recording versus occasion ownership, digests, publication state, and existing expiration state.
  The query rejects duplicate paths and inconsistent note/brief pins without loading private bodies.
  Typed journal owners and atomic expiration timestamps now cover every artifact kind. Explicit
  note/brief regeneration clears its marker, and pending retention blocks engine republication.
  The sweep and startup recovery now cover all stores, preserve operator prose in owned notes,
  and report occasion IDs for briefs. Scans are bounded; per-store holds, dry runs, modified-target
  refusal, interrupted moves, and CLI/pipeline reporting have behavioral coverage.
- [x] Keep expired stages terminal for automatic runs. Explicit `note write` and `brief <occasion-id>`
  may deliberately regenerate their artifacts; pending retention never triggers regeneration.
  End-to-end tests expire real files, recover absent pending targets before stage selection, preserve
  changed pending files, and regenerate saved notes after audio/engine-output expiration. Changed
  analysis inputs and rescheduled calendar events leave expired analysis and briefs absent.
- [x] Extend default logged events and configured hooks to all later-stage producers, including
  transcription failures, uncertainty alerts, completed automatic briefs, and combined retention.
  Retention now persists actual per-store counts and uses the same counts in logs and hooks. Older
  audio-only history remains readable. Pipeline startup recovery and new moves now contribute to one
  final event, including partial failures. Failed event commits preserve journaled moves and send no
  hook. Command tests cover default logging, independent channels, content-free diagnostics, automatic
  briefs, transcription/analysis review alerts, publication ordering, and no notification replay.

## Shared remaining command and configuration behavior

Product sources: specification sections 3.1, 3.5, 9, 10, 11, and 13.

- [x] Add the typed settings and command arguments needed by unfinished features, honoring every
  corresponding key and default in specification section 10. Existing schema acceptance of a key
  does not imply that its runtime feature exists. Keep strict unknown-key/value/version refusal,
  `--config` and `VPT_CONFIG` overrides, and independent per-store paths.
  Brief count settings now accept their specified full 32-bit range through typed conversion,
  selection, preview, and publication; resource limits still apply to actual input and output data.
  Configuration declarations are split by feature without changing their text or order. The 81
  static declarations match the specified defaults; runtime-setting evidence is mapped in the
  ingestion report. A 260-invocation command matrix covers precedence and strict refusal, and the
  pipeline/brief acceptance case publishes artifacts into all seven independently configured stores.
- [x] Give every new command human output and `--json`: exactly one final success document on
  stdout, or one safe `vpt.error/1` on stderr with empty stdout and committed identifiers on failure.
  Use exit 0 for success, 1 for operational failure, 2 for invalid input/configuration, and 3 for a
  named product refusal. Preserve safe logging warnings without changing completed work's status.
  Existing mutating commands now retain committed recording IDs and safe diagnostics if writing
  their final stdout result fails. Human errors also list committed IDs, including recovered
  retention work, without duplicating partial-report IDs. Dry runs and unchanged ingestion/pipeline
  retries claim no new completions. Pipeline output preserves successful engine/proposal diagnostics,
  including when a later stage or final stdout fails; standalone analysis shows them in human output.
  All 26 command forms pass human/JSON success acceptance; configuration and history/refusal matrices
  exercise their safe failure envelopes. Upcoming briefs now carry the required `vpt.result/1`
  envelope, and final analysis-history failures preserve accepted proposal diagnostics. Nested
  startup failures retain each export warning once.
- [x] Keep all advertised dry runs observational. `run`, `transcribe`, and `analyze` spawn no
  engine/agent/hook and perform no migration, repair, publication, or Trash move; the brief context
  exception is limited to configured read-only collection.
  All six commands preserve filesystem bytes and metadata with absent state, a version-one database,
  an unsupported future schema, pending publication/retention/history recovery, and a held write lock.
  Existing brief previews and explanations preserve pending work; automatic pipeline brief context
  is not collected during a dry run.
- [x] Apply bounded versioned documents to the new engine, proposal, context, brief, and handoff
  interfaces: unknown major versions refuse, additive fields produce named diagnostics, and input
  limits apply before unbounded allocation. Engine documents allow 16 MiB, proposals 1 MiB, other
  incoming documents 64 KiB; depth is eight and text values are bounded at 65,536 characters, with
  the specification's per-array limits.
  Protocol and adapter cases cover exact limits, overflow, versions, additive fields, and safe errors.
  Command cases retain accepted engine/proposal diagnostics through success, rollback, publication
  failure, and later pipeline failure in both output modes. Brief and handoff are output interfaces;
  their bounded encoders validate domain content and reject excessive JSON escape expansion.
- [x] Apply the existing direct-argument subprocess ownership, shared deadline, bounded output,
  and interrupt cleanup to each new external engine, analysis, context, and helper command.
  Never invoke a shell to interpret configured arguments or expose raw child output in diagnostics.
  All external adapters use the shared process owner. Acceptance covers literal argv, simultaneous
  stdin/stdout/stderr, output overflow, shared deadlines, the one-second termination grace, exact
  child reaping, and held descendant pipes. Command cases cover interrupted engine/agent runs and
  all three signals during manual/upcoming context collection. Evidence is in the ingestion report.
- [ ] Extend durable publication to every new artifact, retaining committed render state and expected
  previous/intended digests. Repair interrupted publication only when the destination still matches
  the recorded state; preserve operator changes and refuse foreign replacement content.
  The store adapter now receives the expected digest, rechecks after staging, and publishes expected-
  absent targets with exclusive rename. Staged identity/content checks and post-sync digest checks
  preserve detected changes and pending journal entries. Existing-file and temporary-name changes
  after the last check remain open race conditions; this is not yet atomic compare-and-replace.
- [x] Preserve private configuration/state permissions and the strengthened database/path protection
  throughout new commands. Existing shared folders are never made private by changing permissions;
  unsafe existing configuration folders are refused, and newly created private folders are allowed.
  The loader now validates current-user ownership and modes 0600/0700 through the opened configuration
  file and directory. A 104-invocation matrix verifies refusal without changing bytes or metadata
  across all 26 new command forms in both output modes. A further 416 command invocations refuse
  symbolic/multiple links at the main database and every companion, preserving external referents
  and source/artifact files. All 17 mutating forms refuse unsafe state without writes or child work;
  observational forms preserve complete snapshots. Setup now refuses unsafe existing or later-shared
  directories without chmod, while newly created private leaves retain mode 0700. Database containment,
  configuration/store adapters, setup, and focused command acceptance pass 388 tests. The independent
  publication race and intermittent timing rows remain open; details are in the ingestion report.
- [x] Complete standalone installation of the Rust command and Swift helper, useful missing-helper
  diagnostics, and operation with configured external engines when Apple Speech is unavailable.
  Support externally scheduled run-to-completion invocations; VPT itself has no daemon or scheduler.
  An isolated installation from the current checkout passes release-binary version/doctor checks,
  command and Whisply adapter operation without the helper, repeated pipeline invocations, notes,
  briefs, redaction, handoff, and refusal to remove expired audio without the helper. The external
  engines use fixture executables; the native Apple Speech check is documented in the Stage 2 report.

## Explicitly deferred or outside VPT

- The later Open Notebook integration still needs source-creation mapping, transport and credentials,
  notebook-authored-note return handling, and a remote identifier for replacement rather than duplicate
  handoffs. Those are deferred until the external notebook is deployed; the text handoff above remains
  part of VPT's current feature scope.
- Built-in Apple notifications are outside the roadmap. Logs and user-configured report/alert commands
  are the notification-related product behavior.
- Dotfiles builders, templates, package declarations, and launchd installation are outside this
  standalone VPT work. VPT does not read or depend on the author's dotfiles repository.
- Voice capture, iCloud downloads/sync, backups, cloud-spend limits, sending shared files, and changes
  to FluidVoice are outside the product scope. The source Voice Memos container remains read-only.
