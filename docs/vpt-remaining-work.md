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
implementations. Ingestion and audio retention have partial application implementations. Their
complete command-line behavior remains unfinished. Transcription, notes, synthesis, and the other
later-stage commands remain unfinished.

All source changes from the implementation worktrees are preserved in `main` and its history.
The main implementation locations are `crates/` and `helper/vpt-macos/`.

## Stage 1: complete extraction and audio retention

Product sources: specification sections 4, 5, 8.7, and 9.

### Ingestion

- [ ] Complete `vpt ingest [--dry-run] [--once <path>]`, connecting the existing source, archive,
  ledger, history, and reporting capabilities to the command. A normal sweep considers only
  immediate `.m4a` children of the configured Voice Memos Recordings directory; sidecars and
  nested directories are excluded.
- [ ] Complete candidate selection and deferrals: never open an iCloud dataless entry, skip an
  unchanged already-ingested entry, refuse symlink/nonregular source paths, apply the audio-size
  ceiling before reading content, require the complete media container defined in specification
  section 5.2, and enforce the quiet period and unchanged size after an earlier deferral. The defaults
  are 2 GiB and 30 seconds.
- [ ] Persist each deferral reason and consecutive count, retry on later sweeps, and emit one
  `deferred` alert when the configured threshold is reached, default four. Include changed-during-read
  and staged-container failures, with safe source-path metadata and no audio content in events.
- [ ] Complete descriptor-based staging and publication: derive capture time, duration, full digest,
  and identity from staged bytes; retain private file permissions; publish without replacing an
  existing archive; and save the recording only after the archive and directory are durable.
  Cross-volume copies preserve the same behavior and report that the archive is not copy-on-write.
- [ ] Complete duplicate handling across source renames and time-zone changes. Reuse the identity and
  pinned paths for a known digest, update the source association, and report already-ingested work
  without creating another recording.
- [ ] Recover an existing matching archive when recording or source rows are missing. Validate its
  container and full digest, recover the rows, and move the redundant staged copy to Trash. Different
  content at the expected archive path is `archive_collision` and remains untouched.
- [ ] Recover valid orphaned archives at sweep startup, including when the original source is gone.
  Preserve the capture instant and offset represented by the archived identity; do not invent a new
  identity using the machine's current time zone.
- [ ] Mark vanished source paths with `source_gone_at`, retain their archive, and expose the condition
  in `doctor`. An edited or trimmed recording becomes a distinct recording; a renamed source with
  identical bytes retains its identity.
- [ ] Complete the sweep's abort behavior for unreadable sources, an unexpectedly empty previously
  populated source, no space, and failed synchronization. Preserve and report recordings already
  committed before an abort. Title-copy failure continues with an unavailable title.
- [ ] Complete `--once <path>` with the same safety and eligibility rules as a sweep, and complete
  the observational dry run with no clone, title-copy refresh, history, event, migration, or durable
  write. Known titles come from existing ledger state; otherwise the title is unavailable.
- [ ] Complete ingestion startup under the write lock, including unfinished publication repair,
  pending retention reconciliation, and logging recovery before new mutating work. Failed staging
  goes to Trash when possible; if the helper is unavailable, retain the staged file and report it.

### Logged history and command output

- [ ] Connect the existing logging implementation to every unfinished mutating command. Save a run
  start before its work, save state changes and corresponding history together, and distinguish an
  external-effect intent from a confirmed result. Preserve known completed work on later failure.
- [ ] Export new history to private `<state_dir>/events.jsonl` after each history commit. A log-export
  failure preserves the operation's result and exit status and adds a visible warning to the human
  output or existing JSON (JavaScript Object Notation) result/error. Failure to save required
  state/history, or an unsafe state root, stops new work.
- [ ] Complete startup recovery that restores pending log records and appends an evidence-based
  summary of affected runs, observed logging failures, recovered records, completed/failed work,
  and unresolved steps or detectable missing history. Retain original occurrence times separately
  from recovery times, stable record identifiers, and sequence numbers. Resume interrupted recovery;
  never invent an outage cause, repair time, operator action, or an attempt with no surviving evidence.
- [ ] Preserve recognizable record identifiers after an append succeeds but its acknowledgement
  fails. Handle incomplete final log lines, and acknowledge only complete durable records. Recovery
  neither repeats the underlying work nor resends hooks, and recovery summaries produce no hook.
- [ ] Connect independent `[report] command = [...]` and `[alert] command = [...]` hooks to the
  remaining producers. Both default to empty lists, so the default is logs only. Reports cover routine
  summaries; alerts cover failures or items requiring the user's attention. Each configured command
  receives one `vpt.event/1` JSON document on standard input and owns notification formatting/delivery.
- [ ] Preserve operation results when a hook fails; log the bounded failure without retries,
  cross-channel fallback, or native notifications. A log-only export failure still permits the
  original configured hook. Events expose safe identifiers, paths, classes, and counts, never
  transcript text, titles, tags, raw child output, secrets, or redaction mappings.
- [ ] Complete ingest/retention JSON envelopes and human summaries, including safe warnings and
  committed recording identifiers on partial failure. Config refusal before a safe state directory
  exists uses command output only. Observational commands and dry runs do not create or repair logs.

### Audio retention

- [ ] Complete `vpt retention run [--dry-run]` command behavior over the existing audio-retention
  implementation. Retention defaults off, audio defaults excluded, and each hold time defaults to
  `0` (never expire). Expiration uses the file's modification time.
- [ ] Move only ledger-owned, unchanged expired audio through the Swift Trash helper. Preserve
  untracked files, changed content, redirected paths, and files outside the configured authority,
  with an explicit kept reason. A missing helper refuses removal; no artifact is unlinked.
- [ ] Save the retention intent and corresponding history before the move, then save the completed
  move and expiration state together. Preserve actual completed moves if a later state write fails.
- [ ] Reconcile an interrupted or unknown move: an absent path completes expiration, an unchanged
  original may be moved, and replacement content is refused. Preserve the first expiration time and
  distinguish completed reconciliation from a newly performed move.
- [ ] Emit one combined retention report for recovered and newly completed moves, with accurate
  per-store move counts, kept reasons, partial progress, logged history, and only the optional report
  hook. Dry run reports the candidates without taking a mutating lock or changing anything.

## Stage 2: transcription and uncertainty

Product sources: specification sections 3.4, 6, 8.7, 9, and 10.

### Engines and transcription commands

- [ ] Add `vpt transcribe <id> [--language <tag>] [--dry-run]`. Use archived audio without transcoding
  it first, respect the selected language, and preserve the audio and prior accepted transcript if
  a later transcription attempt fails.
- [ ] Support a configured main engine and optional checker, with `checker_runs = always|on_failure`,
  per-language main/checker overrides, and sequential execution. Defaults are Apple as main, no
  checker, language `en-US`, and a 1,800-second per-engine deadline.
- [ ] Reject two engines of the same model family before starting them unless `allow_same_family`
  is enabled. Expose each engine's local/cloud posture; local is the default and sending audio to a
  configured cloud engine remains the user's choice.
- [ ] Extend `vpt-macos` with `transcribe <audio-path> --locale <tag>` using Apple's
  `SpeechAnalyzer`/`SpeechTranscriber`, with per-word time ranges and confidence. Report unsupported
  macOS/framework/language availability and required speech permissions. The helper remains a Swift
  command-line executable alongside its existing version and Trash commands.
- [ ] Add the Whisply adapter using its configured executable arguments, device, model, and language.
  Read its generated JSON file and normalize it to VPT's transcript model. The default device is
  `mlx` and model is `large-v3-turbo`; absent MLX confidence remains absent, while reported
  CPU (central processing unit) scores may supply confidence. Do not interpret an echoed language
  option as independent detection.
- [ ] Add generic command engines with declared family and locality, direct argument substitution
  for `{audio}`, `{language}`, and `{out_dir}`, and `vpt.engine/1` on stdout. Their text has no
  VPT lexical uncertainty classification, including when such an engine occupies the checker slot.
- [ ] Implement bounded `vpt.engine/1` decoding: engine/tool/version/model/family metadata, language,
  locality, ordered timed segments and words, optional confidence, valid finite time ranges, and
  confidence in `[0, 1]`. Refuse unknown major versions, malformed/empty transcripts, and invalid
  identity or range data; retain diagnostics for unknown additive fields.
- [ ] Keep the successful main engine's transcript as the record. If only the checker succeeds, use
  it and add `single-engine`; otherwise diagnose any one-engine-only result as specified. If both
  fail, retain earlier accepted state, mark transcription failed, and emit `transcribe_failed`.
  Independently detected language disagreement adds `language-mismatch`.
- [ ] Persist the accepted transcript and timings, engine metadata, raw documents at
  `engine-outputs/<id>.<engine>.json`, flags, language, and stage state. Publish from saved state so
  interrupted artifact publication does not require rerunning an engine.

### Alignment, flags, and readable text

- [ ] Implement Unicode compatibility normalization and full case folding for alignment, punctuation
  removal, digit-group joining, and ordinal-suffix normalization. Retain a mapping to original word
  positions; known-term matching uses intact whole tokens rather than punctuation-stripped substrings.
- [ ] Align main and checker word streams, retaining original text and time ranges. A divergence
  ratio above `reconcile.max_divergence_ratio`, default `0.35`, produces a whole-recording `divergent`
  diagnostic while confidence and risk classification still operate.
- [ ] Classify and rank lexical flags: unsurfaced formatting differences, numeric disagreement,
  proper-noun disagreement, low confidence below the default `0.35`, agreed-but-unverified risk
  spans, and other disagreement. Risk spans include proper nouns, numbers, dates, times, and money.
- [ ] Suppress agreed-unverified flags only for exact whole-token entries in `known-terms.txt`.
  Aggregate repeated surface forms for display while preserving each occurrence. Keep whole-record
  diagnostics distinct from lexical spans, with stable identifiers that preserve unchanged resolutions
  on re-transcription.
- [ ] Add the optional readability pass, off by default: sentence capitalization, segment-ending
  punctuation, and the specified filler removal. Modify rendered note text only, preserving every
  flagged span and the accepted/raw transcript.
- [ ] Produce the first transcript note with the selected portable or Obsidian profile, full required
  frontmatter, filename-matching heading, empty managed links region, and timecoded content region.
  Render uncertainty and correction markers inline without a separate uncertainty-summary block.
- [ ] Emit privacy-filtered `review_needed` only after publication, with identity, path, and class
  counts. Apply the configured aggregation threshold, default three recordings, for one aggregate
  alert when the threshold is exceeded; keep transcript text and titles out of events.

### Transcript corrections and known terms

- [ ] Add `vpt review <id>` to display ranked open flags, record text, alternatives, and timecodes.
- [ ] Add `vpt review <id> --resolve <flag> --confirm|--correct <text>|--dismiss`. Persist the
  resolution, update counts and rendered notes, and keep corrections separate from the immutable
  accepted word stream. Preserve unchanged resolutions when transcription is repeated.
- [ ] Add idempotent `vpt confirm --term <text>` and the specified automatic known-term additions
  from proper-noun/agreed-unverified confirmations and corrections.
- [ ] Define the remaining locale-mapping details for backend language tags versus regional tags,
  and the divergence-ratio denominator for unequal word-stream lengths. The draft uses the longer
  stream's length; the product specification does not yet settle those details completely.

## Stage 3: notes, tags, relations, and the initial pipeline

Product sources: specification sections 3.3, 4.3-4.5, 7, 9, and 10.

### Owned notes and paths

- [ ] Complete both `portable` and `obsidian` note profiles and independently selectable Markdown or
  wiki link style. Include the profile's exact provenance, engine, language, uncertainty, and VPT
  identity/stage fields. Obsidian notes also carry aliases, reference/hub links, status, description,
  and the local first-write start date.
- [ ] Maintain separate `vpt:links` and `vpt:content` regions. Preserve operator text outside them,
  operator tags, and the specified editable frontmatter fields. Escape external strings for Markdown
  and frontmatter; reject missing/duplicate markers, foreign identities, and unsupported schemas.
- [ ] Pin flat note paths at first publication as `{capture-date}-{slug}-{hash8}.md`. Derive the slug
  only from the Voice Memos title, with Unicode normalization/case folding, safe characters, collapsed
  hyphens, the configured length limit (default 60), and `untitled` fallback. Later title or settings
  changes do not rename an existing note.
- [ ] Add `vpt note write <id>` to render existing ledger state into the transcript and, when present,
  analysis note. It also provides deliberate regeneration after expiration; automatic runs leave
  expired artifacts expired and do not regenerate artifacts with pending retention intents.
- [ ] Add read-only `vpt path <id> --stage <stage>`, returning the pinned absolute path even if its
  artifact is currently missing.
- [ ] Recover a uniquely renamed owned note by its recording identity during a mutating operation.
  Update the pinned path and log the move once. Refuse ambiguous claimants, foreign occupants,
  symlinks, and escaped paths; recreate a missing note only when its state permits regeneration.
- [ ] Build a per-run read-only index of Markdown names, aliases, and tag vocabulary from the
  configured Obsidian vault or portable transcript/analysis/brief stores, including untracked notes.
  Do not follow symlinks or treat indexed external notes as VPT-owned.

### Tags and relations

- [ ] Implement normalized tag vocabulary and `known-tags.txt`: case/separator variants reuse an
  existing spelling; multi-word tags become kebab-case; accepted syntax contains letters, digits,
  underscores, hyphens, or slashes, has a nonnumeric character, and has no leading `#`.
- [ ] Implement `tags.new_tags = hold|write`. The default `hold` keeps unknown tags in
  `vptSuggestedTags`; `write` confirms and records them automatically. Deduplicate in proposal order,
  default to five confirmed tags and ten suggestions, and preserve the operator's tags.
- [ ] Add `vpt confirm <id> --tag <tag>` and `--reject-tag <tag>`, with durable tag state, known-tag
  updates where appropriate, and note changes. Manual confirmation works without a synthesis command.
- [ ] Derive `continues` from the immediate predecessor by capture instant then identity, provided
  its end precedes the current start and the gap is within `relations.session_gap_minutes`, default
  60. An overlapping predecessor does not cause a search for an older substitute.
- [ ] Derive `mentions` from confirmed whole-token terms that uniquely identify an indexed note name
  or alias. Preserve all source ranges; ambiguous matches produce no link.
- [ ] Represent and render `related` suggestions with provenance and the configured cap, default
  five. Synthesis proposals supply these later. Allow `supersedes` only through explicit
  `vpt confirm <id> --relation <kind>:<target>`; support the four specified relation kinds.
- [ ] Render deterministic managed links, including the later transcript/analysis relationship,
  without editing any external note that VPT merely indexed. Filing remains flat per store.

### Pipeline and visibility

- [ ] Add `vpt run [--dry-run]` initially composing ingestion, transcription, and note publication.
  Resume each recording from its earliest enabled pending/failed stage. Re-enable previously disabled
  incomplete stages when configuration enables them; preserve earlier success and expiration.
- [ ] Keep the exact rendered transcript content-region bytes and digest available as synthesis
  input. Corrections and readability changes affect that input; link-only/frontmatter/operator-prose
  changes outside the content region do not.
- [ ] Extend `doctor` for engine availability/locality and the configured Obsidian vault, and expose
  the placement of audio, engine outputs, and private redaction reports inside Git-tracked trees.
  These observations never alter the vault or its ignore files.
- [ ] Extend `show`, `list --stage`, and `storage` to expose the newly implemented stage states,
  engine/language metadata, flags, note paths, and artifacts while remaining observational.

## Stage 4: synthesis, briefs, sharing, and handoff

Product sources: specification sections 3.3, 8, 9, and 10.

### Synthesis and analysis notes

- [ ] Add optional `[synthesis]` settings and `vpt synthesize <id> [--dry-run]`. An empty command
  disables synthesis; explicit invocation without one is a configuration error. No model vendor is
  built in. The default command deadline is 600 seconds.
- [ ] Run the configured argument list with `{transcript}`, `{id}`, and `{language}` substitution.
  Send the exact transcript `vpt:content` bytes on standard input and retain their digest.
- [ ] Decode bounded `vpt.proposal/1` output containing ordered summary/action claims with source
  ranges and proposed tags/relations. Save accepted proposal state and provenance; invalid output
  or a failed command preserves prior accepted state.
- [ ] Render the owned analysis note with Summary and Actions sections, escaped claim text, source
  citations, inherited uncertainty, profile frontmatter, and reciprocal transcript/analysis links.
  Route tag and relation proposals through their existing product rules. `note write` rerenders
  both notes from saved state.
- [ ] Recompute enabled synthesis when the rendered content digest changes. Invalidate derived
  verification findings and inherited uncertainty when their transcript or lexical-flag inputs
  change; expired outputs remain expired during automatic runs.

### Note grounding

- [ ] Add the product command `vpt verify-note <path> --recording <id>`. Read claims from Markdown
  list items and paragraph sentences, excluding frontmatter, headings, fenced code, link syntax,
  and existing annotations.
- [ ] Classify claims in order as `unsourced`, `bad-reference`, `unsupported`, or
  `built-on-flagged-text`. Resolve time references against the recording; compare digits, money,
  dates, and capitalized whole tokens to the cited transcript plus `verify.context_window_secs`,
  default 15 seconds. Preserve meaningful number/currency/date punctuation in grounding.
- [ ] Report findings for readable external notes without modifying their bytes. Annotate only the
  correctly identified, ledger-owned artifact with valid markers and safe paths; a mismatched
  recording is an ownership refusal. Findings alone are a successful command result.
- [ ] Persist finding identity by artifact, claim position, and annotation-stripped claim digest.
  Replace only that artifact's findings, retain resolutions for unchanged claims, update open counts,
  and annotate claim endings. Apply this grounding to new analysis before reporting synthesis success.

### Occasions and briefs

- [ ] Add `vpt brief <occasion-id>`, manual `vpt brief --title <title> --at <RFC3339>` with duration,
  participant, and tag options, and `vpt brief --upcoming`. Persist stable occasion identities;
  provider keys survive title/time changes and manual identity is reproducible.
- [ ] Add `vpt occasions` and read-only `vpt path --occasion <id> --stage brief`. Keep unbriefed
  occasions visible, pin each brief's path, and rewrite that same brief for a repeated request.
- [ ] Select deduplicated transcript notes using confirmed participant, term, and tag matches,
  followed by recent continuation/shared-confirmed-tag matches. Rank by match class then newest
  capture, with defaults of 12 notes, 180 days of lookback, and five spans per note.
- [ ] Preserve rejected and omitted candidate/span counts. An unconfirmed participant term appears
  under Not selected with the applicable term-confirmation command. `--explain` describes selection
  and rejection without publishing a brief.
- [ ] Persist each assembled brief pack with source identities and digests. Render the required
  occasion frontmatter and managed sections: Occasion, Unresolved, From your notes, Open tasks, and
  Not selected, including empty sections. Quotes are verbatim transcript spans with recording and
  timecode provenance; uncertain quotes carry `[unverified]` inline.
- [ ] Emit `vpt.brief/1` with the occasion, unresolved issues, sourced items carrying certainty and
  sources, not-selected information, separate calendar/task context status, and generation time.
  Mark an occasion briefed only after durable publication. Requested briefs emit no event;
  successful automatic briefs emit one `brief_written` report.
- [ ] Support a wholly observational `brief --dry-run` that may collect the configured read-only
  context and returns the pack it would write.

### Calendar and task context

- [ ] Add `context.type = none|dam|google` and bounded `vpt.context/1` normalization. Calendar and
  task halves independently report present data or absence with disabled/unsupported/failed reason
  and safe detail. A context collection has one 30-second deadline.
- [ ] Implement `dam` collection through the configured command with `--json --no-pull`, restricted
  to configured calendar path prefixes and task project prefixes. Reject missing selections when
  the requested supported context half needs them. VPT does not use Todoist's command or `gog`.
- [ ] Implement native Google Calendar reading over HTTPS (Hypertext Transfer Protocol Secure) with
  `rustls`, configured calendar identifiers, time-bounded expanded recurring events, pagination,
  a ten-page-per-calendar cap, and only the read-only events permission. Google task context is
  explicitly unsupported.
- [ ] Preserve read-only context and secret handling: configured OAuth credentials remain private,
  no calendar or task writes occur, and failures reveal no token or provider response body.
- [ ] Add the opt-in calendar trigger, default off with 24-hour lead time. Select occasions in
  `now <= start < now + lead_time`. Disabled `--upcoming` is a configuration error; failed or
  incomplete discovery fails without marking occasions briefed. A manual brief can still publish
  while clearly showing a failed context half.

### Redaction and released copies

- [ ] Add `vpt redact <id> [--stage transcript|analysis] [--to <directory>] [--title <title>]` and
  `[share]` settings. `share.automatic`, default false, creates releases after new transcript or
  analysis artifacts during `run`, independently of whether synthesis is enabled. It does not
  regenerate the same release on every unchanged run. Audio and briefs are not redaction sources.
- [ ] Use the configured released store or per-command destination, refusing overlap with private
  stores, state, configuration, or Voice Memos. Preserve source artifacts and publish without
  overwriting an existing released file.
- [ ] Construct minimal released frontmatter with the optional title/date and source statement.
  Strip private identity/vault metadata, managed links, link targets, denied sections, and timecodes
  by default. Parse Markdown and decode entities; remove raw HTML (Hypertext Markup Language),
  comments, and reference targets rather than leaving hidden private content.
- [ ] Always mask confirmed terms and indexed note names, plus configured email, web-address, phone,
  four-or-more-digit number, and money classes. Apply deterministic longest-match precedence and
  masks numbered from one for each copy; the default mask is `[{class} {n}]`.
- [ ] Omit and count open flagged transcript spans and analysis claims built on flagged text by
  default, or retain them explicitly marked `[unverified]` when configured. Report capitalized
  survivors as candidates. A residue scan of decoded output refuses publication if forbidden text
  remains, without exposing that text in the error.
- [ ] Save a private mode-0600 report for each release, mapping masks and released lines to source
  spans. Keep that report outside the released copy and never expose mask mappings through logs.
- [ ] Use standardized release names without the private note slug. Refuse byte-identical copies
  already anywhere in the destination; resolve different-content name collisions with a local
  timestamp and then the smallest available suffix. Released content remains immutable.
- [ ] Persist release sequence, destination, source/content digests, private report path, and frozen
  provenance: capture time, engines, lexical and verification uncertainty, diagnostics, and reviewed
  state. A release becomes selectable only after both its copy and private report are durable.

### Text handoff

- [ ] Add read-only `vpt handoff <id> --stage released|transcript|analysis|brief`, emitting
  `vpt.handoff/1` with text, deterministic title, kind-specific identity, provenance, and generation
  time. Private stages require `handoff.allow_private`, default false.
- [ ] For released handoffs, select the newest finalized existing release by sequence, retain its
  frozen provenance, and refuse changed bytes. Remove released frontmatter and prepend the
  declarative VPT handoff header. No release is an input error; only pending publication is an
  operational failure.
- [ ] For private handoffs, use the owned artifact's current content and freshly computed digest,
  preserving operator prose outside managed regions. Briefs resolve an occasion and its source
  records. Unreviewed artifacts are labelled rather than refused solely for being unreviewed.
- [ ] Keep handoff text-only: no notebook transport, upload, clipboard action, credential access,
  or file write occurs as part of the command.

### Full pipeline and retention across all stores

- [ ] Extend `vpt run` with enabled synthesis, independent automatic redaction, and opt-in retention.
  Its result lists ingested, transcribed, noted, synthesized, and redacted work plus retention
  outcomes. Resume the earliest pending/failed enabled stage and preserve committed partial results.
- [ ] Extend the ledger from audio-only operations to accepted transcripts, proposals, flags, tags,
  relations, occasions/brief packs, engine outputs, redaction reports, and release provenance.
  Keep accepted render inputs after raw-engine-output retention so notes remain regenerable.
- [ ] Extend retention to ledger-owned engine outputs, transcript and analysis notes, briefs,
  private reports, and releases, using each store's hold time. Validate note/brief identity, stage,
  pinned path, and markers; validate other artifacts by digest. Preserve modified/untracked files
  and all expiration/recovery protections established for audio.
- [ ] Keep expired stages terminal for automatic runs. Explicit `note write` and `brief <occasion-id>`
  may deliberately regenerate their artifacts; pending retention never triggers regeneration.
- [ ] Extend default logged events and configured hooks to all later-stage producers, including
  transcription failures, uncertainty alerts, completed automatic briefs, and combined retention.

## Shared remaining command and configuration behavior

Product sources: specification sections 3.1, 3.5, 9, 10, 11, and 13.

- [ ] Add the typed settings and command arguments needed by unfinished features, honoring every
  corresponding key and default in specification section 10. Existing schema acceptance of a key
  does not imply that its runtime feature exists. Keep strict unknown-key/value/version refusal,
  `--config` and `VPT_CONFIG` overrides, and independent per-store paths.
- [ ] Give every new command human output and `--json`: exactly one final success document on
  stdout, or one safe `vpt.error/1` on stderr with empty stdout and committed identifiers on failure.
  Use exit 0 for success, 1 for operational failure, 2 for invalid input/configuration, and 3 for a
  named product refusal. Preserve safe logging warnings without changing completed work's status.
- [ ] Keep all advertised dry runs observational. `run`, `transcribe`, and `synthesize` spawn no
  engine/agent/hook and perform no migration, repair, publication, or Trash move; the brief context
  exception is limited to configured read-only collection.
- [ ] Apply bounded versioned documents to the new engine, proposal, context, brief, and handoff
  interfaces: unknown major versions refuse, additive fields produce named diagnostics, and input
  limits apply before unbounded allocation. Engine documents allow 16 MiB, proposals 1 MiB, other
  incoming documents 64 KiB; depth is eight and text values are bounded at 65,536 characters, with
  the specification's per-array limits.
- [ ] Apply the existing direct-argument subprocess ownership, shared deadline, bounded output,
  and interrupt cleanup to each new external engine, synthesis, context, and helper command.
  Never invoke a shell to interpret configured arguments or expose raw child output in diagnostics.
- [ ] Extend durable publication to every new artifact, retaining committed render state and expected
  previous/intended digests. Repair interrupted publication only when the destination still matches
  the recorded state; preserve operator changes and refuse foreign replacement content.
- [ ] Preserve private configuration/state permissions and the strengthened database/path protection
  throughout new commands. Existing shared folders are never made private by changing permissions;
  unsafe existing configuration folders are refused, and newly created private folders are allowed.
- [ ] Complete standalone installation of the Rust command and Swift helper, useful missing-helper
  diagnostics, and operation with configured external engines when Apple Speech is unavailable.
  Support externally scheduled run-to-completion invocations; VPT itself has no daemon or scheduler.

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
