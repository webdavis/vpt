# vpt

The Voice Processing Tool is under construction. Implemented commands include `vpt --version`,
`vpt --version --json`, and `vpt setup`. The macOS helper provides version reporting, Trash,
and Apple speech transcription. Feature completion is tracked in
[the remaining-work roadmap](docs/vpt-remaining-work.md).
The design is [the approved specification](docs/superpowers/specs/2026-09-20-vpt-design.md).

## Build

`cargo build --locked` builds `target/debug/vpt`. The helper requires Xcode and Swift 6.
`just swift-build` builds it with isolated project-local caches. `just ship` runs the Rust and Swift
workspace checks.

## Machine-readable output

Every command accepts one of `--json`, `--toon` or `--yaml`, in any position. Each flag prints the
same document: a `vpt.result/1` document on standard output when the command succeeds, or a
`vpt.error/1` document on standard error when it fails, with the same exit code. `--json` prints the
document as one line of compact JSON. `--toon` prints it as TOON (Token-Oriented Object Notation), with
nesting by indentation, a declared length on every array and quotes only where a bare value would
read as something else. `--yaml` prints it as block YAML with every string double-quoted. Without a
flag, commands print text for people. Passing two different format flags is a usage error, reported
as text with exit code 2.

`--progress`, in any position, adds progress lines on standard error while `run`, `transcribe` or
`analyze` works. Each engine run and each analysis command writes one line of compact JSON when it
starts and one when it ends: `{"schema": "vpt.progress/1", "recording", "step", "state", "audio_secs",
"at"}`, where `step` is `transcribe` or `analyze`, `state` is `started`, `done` or `failed`, and
`audio_secs` is the recording's length for a transcription and null otherwise. Standard output is
unchanged. On failure the error document follows the progress lines, so under `--json` standard error
stays one JSON object per line. Other verbs and dry runs write no progress lines.

## Configuration permissions

VPT requires the selected configuration file to have mode `0600` and its containing directory to
have mode `0700`, both owned by the current user. This applies to the default location, `VPT_CONFIG`,
and `--config`. VPT refuses to read unsafe configuration without changing its permissions.
`vpt setup` creates private configuration, home, state, and store directories. Existing directories
must already have mode `0700` and be owned by the current user; otherwise setup refuses before
prompting or writing configuration. It also rechecks each directory before using it. `--force`
allows configuration replacement; it does not change existing directory permissions.

A configured home, state directory, or store may sit below folders that do not exist yet. Writing
commands create every missing folder with mode `0700`; `vpt doctor` and other read-only commands
report the path as usable and create nothing. VPT checks overlaps from the nearest existing folder,
so it refuses a path with `..` below a missing folder, a dangling symlink, a symlink that appears
where a folder is about to be created, or a file where a folder belongs.

## Apple speech helper

`vpt-macos transcribe <audio-path> --locale en-US` emits a `vpt.engine/1` transcript with word
timings and optional confidence. It requires macOS 26 or later, a supported Mac and locale, and
installed Apple speech assets. Missing support or assets produce an error; the helper does not
download models. It reserves the installed locale for use by the helper before starting analysis.

The helper reads an existing audio file and uses `SpeechAnalyzer` locally. Apple's legacy
speech-recognition permission request applies to `SFSpeechRecognizer`, not this API; see
[Apple's permission documentation](https://developer.apple.com/documentation/speech/asking-permission-to-use-speech-recognition).

To run the opt-in native regression, generate a WAV of “The quick brown fox jumps over the lazy dog”
with macOS `say`, then pass its absolute path as `VPT_TEST_APPLE_SPEECH_AUDIO` to `swift test`.
The model must already be installed. With the isolated wrapper, put `env
VPT_TEST_APPLE_SPEECH_AUDIO=/absolute/path/to/generated.wav` after the wrapper so the test receives
that variable. Ordinary tests skip this host-dependent case.

## Ingestion and archive recovery

`vpt ingest [--once <path>] [--dry-run] [--json|--toon|--yaml]` archives eligible Voice Memos recordings.
A sweep considers only immediate `.m4a` files; it excludes sidecars and nested folders. `--once`
selects one recording inside that directory and applies the same eligibility checks. The defaults
are a 2 GiB size limit and a 30-second quiet period. Dataless iCloud entries are deferred without
opening them. A previously deferred file must also keep the same size across sweeps.

Deferrals retain their reason and consecutive count. The fourth deferral sends one alert if an alert
command is configured; later retries do not repeat it. `source.deferral_page_threshold` changes that
threshold. Successful ingestion clears the deferral state. Dry runs leave files, ledger state,
history, hooks, and title copies untouched.

A renamed source with known bytes keeps its recording identity and archive path, including after a
time-zone change. Edited bytes create a new recording. Orphan recovery uses the capture offset in the
archived identity even when the source has disappeared.

Before publication, ingestion checks that the inspected staged file still matches its recorded size
and digest. Failed stability checks preserve the staged file and report cleanup pending.

Recovery reads container metadata and the full digest through one retained file descriptor and checks
that its file version and path still match. Synchronization checks the expected digest before and
after syncing the file and directory. Detected edits or replacements refuse recovery/source updates
and preserve the changed file. Different content at an expected archive path is `archive_collision`.

## Importing other files

`vpt import <file> [--title <text>] [--captured-at <RFC 3339 time>] [--json|--toon|--yaml]` archives one audio or
video file from anywhere on disk. vpt never downloads; the caller provides a local file. A `.m4a` file is
archived as it is. Any other file has its first audio stream converted to `.m4a` by the command in
`[import] extract_command`, default `["ffmpeg"]`. vpt appends `-nostdin -loglevel error
-protocol_whitelist file -i <file> -vn -map 0:a:0 -map_metadata 0 -c:a aac <output>`, so a playlist
or other file that points at a network address is not followed. A missing or failing extractor is
refused with exit 3.

The capture time is `--captured-at`, else the container's creation time when it is set, else the file's
modification time. The title is `--title`, else the file name without its extension, recorded with
title source `import`. The recording keeps the original file as its source path. The quiet period does
not apply, and any deferral is refused with exit 3. Importing the same bytes again reports the existing
recording under `already_ingested`. Output uses the `vpt ingest` shapes with `command: "import"`.

Converted audio is identified by its bytes. A different ffmpeg version can encode the same file
differently, so re-importing a converted file after an ffmpeg upgrade can create a second recording.

## Transcription command

`vpt transcribe <id> [--language <tag>] [--dry-run] [--json|--toon|--yaml]` runs the configured main engine
and optional checker against a private copy of archived audio. It saves the accepted transcript
and raw engine documents, then publishes the transcript note using the selected profile and
readability settings. Interrupted publication resumes from saved bytes. Engine failures preserve the
previous accepted text; note failures retain newly accepted text and leave the note pending. Dry runs report the selected engines and their locality without starting them
or changing files.

Pairs from the same model family are refused before either engine starts. The error names both
engines and their shared family; `[engines] allow_same_family = true` explicitly permits the pair.

Transcription also saves lexical uncertainty flags and whole-recording diagnostics. Built-in engines
use the configured confidence and divergence thresholds and the whole-token known-terms list.
Generic command engines remain exempt from lexical classification. Command output includes diagnostics
and safe engine warnings, the published note path, and open uncertainty counts. Repeated occurrences
count separately in class totals. Resolved flags and corrections survive unchanged retranscription.

Risk detection uses non-sentence-initial capitalization, numeric characters, and an English vocabulary
for spelled numbers, ordinals, dates, times, and currencies. It is heuristic: lowercase names and
sentence-initial names are not inferred, and spelled quantities in other languages are not recognized
by the English vocabulary. Known terms suppress only agreed-unverified flags, leaving disagreements
and low-confidence words available for review.

After durable note publication, open lexical flags or diagnostics produce a private-safe `review_needed`
event and the optional alert hook. Hook or log-export failures add safe warnings while preserving
successful work. Failed publication emits no review alert. Retranscription leaves expired notes expired;
`note write` remains the explicit regeneration command.

## Pipeline

`vpt run [--dry-run] [--json|--toon|--yaml]` composes ingestion, transcription, note publication, stale-finding
verification, configured analysis, and optional automatic redaction.
It resumes pending or failed work in capture order, retries previously disabled incomplete stages,
and preserves successful transcription and expired notes. Changed readability/profile settings refresh
notes from saved words without rerunning engines. Unchanged notes retain their bytes and produce no
repeat review alert.

Mutating runs hold one write lock and recover saved publications, pending retention, and logging
before new work. A later failure reports committed recording identities. Review alerts follow durable
note publication; more than `[alert].aggregate_after` recordings (default three) produce one combined
alert with class counts. Dry runs report ingestion candidates and pending stages without starting
engines, changing files, repairing state, or invoking hooks.

Enabled analysis runs after current transcript findings are rechecked. Unchanged rendered inputs and
grounding settings skip the agent. Failed analysis retries without repeating successful transcription;
pending publication repairs accepted analysis without rerunning its agent. Expired notes stay expired,
including when the other artifact still needs verification. When `[retention] enabled = true`, the
pipeline runs retention after processing and includes moved files, kept files, and per-store counts
in its result. Audio also requires `include_audio = true` and a nonzero hold time. Dry runs list the
current candidates without moving them. Startup reconciles pending moves, and a later retention
failure preserves completed processing work. Retention ownership for later stores remains unfinished.
Disabling analysis
marks unfinished work disabled; enabling it again records pending work. Successful and expired outputs
are preserved. Changed enabled inputs become pending before new artifact work. These transitions and
their history commit together. Interrupted engines or agents mark their child run and parent pipeline
interrupted, while preserving committed work and the normal operational-failure exit code.

## Review and known terms

`vpt review <id> [--json|--toon|--yaml]` displays ranked open lexical flags, alternatives, occurrence ranges and
timecodes, with whole-recording diagnostics listed separately. It reads saved state without running an
engine or changing files.

`vpt review <id> --resolve <flag> --confirm|--correct <text>|--dismiss [--json|--toon|--yaml]` resolves a
surfaced flag and rewrites the transcript note. Corrections remain separate from accepted words.
Confirming proper-noun/agreed-unverified flags learns their record text; corrections learn the new
text. Repeating the same action preserves its resolution time and avoids duplicate terms. Diagnostics
can be confirmed or dismissed. Invalid flags/actions refuse before mutating work.

Resolution state, rendered note bytes, and any known-term addition commit together before publication.
Startup recovery resumes saved publications. Operator note edits and renamed owned notes are preserved;
malformed notes and redirected term lists refuse the update. A changed content digest makes succeeded
analysis pending. JSON includes the resolved flag's state, whether it changed, its note path, and the
remaining open flags. History omits correction and transcript text.

`vpt confirm --term <text> [--json|--toon|--yaml]` adds a confirmed term without needing a recording or engine.
It normalizes whitespace, preserves existing list content, and recognizes whole-term Unicode/case
variants as duplicates. New files are private. The ledger saves publication bytes and intent before
writing, and mutating startup recovers interrupted updates. Changed files or a changed configured list
path refuse recovery. Logs and command summaries omit the term text; log-export failures add a warning
without undoing a completed confirmation.

## Inspecting saved state

`vpt show <id> [--json|--toon|--yaml]` and `vpt list [--stage transcribe|note|analysis] [--json|--toon|--yaml]` expose
stage states, accepted engine metadata, requested/reported language, open flag and diagnostic counts,
note paths, and artifact metadata. Artifacts distinguish published, pending publication, and expired
files; expiration retains their recorded paths and digests. Stale verification findings are excluded
from current counts and their artifact stages are listed separately. Each command reads one ledger
snapshot without recovering publications or running engines. Without `--stage`, results contain
metadata, with no transcript or proposal bodies. `list --stage` selects stages whose state is not `succeeded`, including
`disabled` and `expired`. Observation results are bounded to 65,536 recordings and 16 MiB of retained
metadata.

`vpt show <id> --stage released|transcript|analysis|brief` returns one stage's text for a person or a
program. Brief takes an occasion ID; the others take a recording ID. Human output is the text: the
artifact body with its frontmatter removed, under a header that names the stage, identity, open flag
count and review state. JSON output adds beside it the `title`, `content` (that text), `note` (its path),
`content_sha256` (the artifact bytes before frontmatter removal), `provenance` (`produced_by`, `engines`,
`open_flags`, `reviewed`, `redacted`, and `captured_at` or, for a brief, `occasion_at`) and `generated_at`.
A brief also lists its source `recordings`. Transcript, analysis and brief copies keep the current body,
including operator prose outside managed regions. Released selects the newest finalized release whose file
still exists and takes provenance from that release's saved snapshot; a changed copy is refused (3) and
a release still pending is an operational error (1). A missing stage names the stages that exist.

`--redact` with `--stage transcript|analysis` returns the stage's text with private details masked at
call time by the same pipeline and `[share]` settings `vpt redact` uses: confirmed terms, indexed note
names and aliases, the configured pattern classes, denied sections, and open flagged spans omitted or
marked as `share.redact.flagged_spans` says. The text starts with the configured `share.source_line`, and
`provenance.redacted` is true. The redacted document carries only that text: `segments`, `summary` and
`actions` are absent, so nothing leaves outside the one masked text. For timecoded lines, set
`share.keep_timecodes = true` and read the `[mm:ss]` lines in `content`. `note` is null and `title` names
the recording ID, since a note's name can hold private words. `content_sha256` is the digest of the
redacted `content` returned, so a caller can check what it received. A pending note publication stops it
(1), as it does without `--redact`. Nothing is saved: no released copy, report, ledger row or event. Only
the managed content region is masked, as with `vpt redact`. `--redact` without `--stage`, with `--stage
released` (already redacted) or with `--stage brief` (not a redaction source; its quotes have no map to
their recordings' flags) is a usage error (2).

`vpt show <id> --stage transcript` also returns the accepted transcript's segments from saved state, each with
`start` and `end` seconds, a `mm:ss` `timecode` and its `text` with review corrections applied.
`vpt show <id> --stage analysis` also returns the accepted proposal's `summary` and `actions` lines in
proposal order. Each line has its `text`, its `sources` (`start` and `end` seconds and a `mm:ss-mm:ss`
`range`), its verification `finding` or null, and a `moment` of the form `<recording id>@<mm:ss>` taken
from the start of its first source, or null for an unsourced line. Two lines from the same second share
a moment. Each action line also has its `owner`, as the proposal gave it, or null, and its `due`, or
null. A `due` has its `date`, its `time` or null, its own `sources` and its own `finding`; when that
finding is `unsourced` or `bad-reference`, `date` and `time` are null. An action's moment comes from
the action's sources only. No stage needs an opt-in. A recording with no accepted transcript or proposal is an input error (2).

`vpt storage [--json|--toon|--yaml]` reports current file counts, bytes, and oldest/newest timestamps for all seven
stores. It works independently of the recording source and ledger.

`vpt doctor [--json|--toon|--yaml]` reports engine availability and locality for every configured engine. It checks
executable files without running arbitrary engine commands; Apple also requires a compatible helper.
Unavailable engines selected by the default pair or any language override fail the check. Unused
engine definitions are reported without failing it. Generic commands' locality is explicitly labeled
as declared configuration. Doctor also reports the Obsidian vault and every store's Git placement,
calling out audio ignore rules and sensitive engine outputs and redaction reports. It leaves vault
contents, permissions, and ignore files unchanged.

## Note writing

`vpt note write <id> [--json|--toon|--yaml]` writes the transcript and any existing analysis note from saved ledger
state without audio or an engine. Both notes link to the archived audio and to each other. It supports
portable and Obsidian profiles, independent Markdown/wiki link settings, inline uncertainty and
corrections, and the optional readability pass. The first filename is pinned;
later writes retain operator prose, manual tags, editable frontmatter, and first-write metadata.
Malformed markers, foreign occupants, and unsupported schemas refuse rewriting.

Readability is off by default. `[readability] enabled = true` capitalizes sentence starts,
adds missing segment-ending full stops, and removes `um`, `uh`, `erm`, and `like` followed by a
comma. Every lexical flag protects its original words, including formatting flags; corrections and
uncertainty markers are preserved. The note records `vptReadability`, and accepted transcripts and
raw engine documents remain unchanged. Changing the setting refreshes notes from saved state.

Wiki links include the target's folder path, so transcript and analysis notes with the same filename
link to each other correctly. Obsidian notes use the configured vault as the link root. Portable notes
use the common parent of the transcript, analysis, and brief stores. Targets outside that root and
names that cannot safely appear in wiki syntax use encoded relative Markdown links.

Note settings include slug length and Obsidian hubs and vault path. `doctor` reports the configured
vault directory; the Obsidian profile requires an existing absolute directory. These checks leave
vault contents and permissions unchanged.

The ledger saves note bytes, full/content digests, managed tags, and publication intent together.
Startup recovery publishes the saved bytes; stage success and journal removal commit together after
publication. Explicit regeneration of an expired transcript or analysis note retains saved operator edits. Engine
provenance is saved with accepted text, so new two-engine transcriptions retain both engines after
raw-output removal. Older accepted state without checker metadata remains readable.

`vpt path <id> --stage transcript|analysis [--json|--toon|--yaml]` returns the saved absolute pin, including when
the artifact is missing, changed, or pending publication. It does not read the artifact or repair state.

When a note pin is missing, `note write` scans its flat store for the recording identity. A unique
owned claimant becomes the new pin, with one move history record; no claimant allows regeneration at
the old pin. Ambiguous claimants, redirected paths, invalid owned documents, and foreign occupants
refuse rewriting. Operator edits and first-write metadata survive recovery.

Analysis rewrites retain saved claim content and apply matching verification resolutions. Review,
tag, and relation updates publish both notes together; automatic updates leave expired analysis notes
expired. The pipeline detects changes to either note, and dry runs remain observational. Publishing
analysis metadata does not complete analysis whose input digest or grounding basis has changed, or
clear a failed analysis attempt. `analyze` creates the initial analysis note.

## Occasions and briefs

`vpt occasions [--json|--toon|--yaml]` lists saved occasions, including those without a brief, in exact start-time
order. Output includes the occasion identity, start, source, duration, participants, tags, and saved
brief path/status. Provider object keys stay private to the ledger.

`vpt path --occasion <id> --stage brief [--json|--toon|--yaml]` returns the saved absolute brief path even when its
file is missing. Both commands are observational: an absent ledger stays absent, and neither command
publishes or repairs an artifact. Accepted brief packs and rendered text are stored together;
mutating startup repairs pending brief publication and then marks the occasion briefed. Replaced
files are preserved and recovery refuses to overwrite them.

`vpt brief <occasion-id> [--dry-run|--explain] [--json|--toon|--yaml]` writes or previews a saved occasion's brief.
Create a manual occasion with `vpt brief --title <title> --at <RFC3339>`, optionally adding
`--duration-minutes <minutes>`, repeated `--participant <name>`, and repeated `--tag <tag>`.
Repeated requests preserve the original brief path and first-write metadata, including after a
provider occasion is rescheduled. Rewrites preserve operator properties, tags, and prose.

Briefs select current owned transcript notes using confirmed terms, tags, and relations. Quotes use
accepted transcript text and retain recording/timecode provenance and uncertainty. Missing or expired
transcripts supply no evidence; unaccepted changes to managed transcript content refuse the request.
`--json` emits `vpt.brief/1`; `--explain` includes selection decisions and omission counts. Both
`--dry-run` and `--explain` leave stores, ledger, and history unchanged.

With `context.type = none`, both context halves are disabled. `context.type = dam` reads the selected
`context.calendars` and `context.projects` through the configured `context.dam.command` (default
`["dam"]`). Each query includes its selected path and `--json --no-pull`; no remote pull is requested.
Both selections must be nonempty. Paths containing query operators or whitespace are refused before
state creation. Collection shares one 30-second deadline and validates bounded provider output.

Calendar and task failures are reported separately, and manual briefs can still publish with the
successful half. Open task text is escaped and saved with its source. Previews may perform these
read-only queries. Zoned and all-day dam events preserve their local date and daylight-saving duration;
an all-day event without a declared zone uses the system zone. Boundary days are queried to include
events whose local date differs from UTC, then exact start/end bounds select the returned occasions.

`context.type = google` reads the configured calendar IDs through native HTTPS with rustls. Set
`context.google.client_id`, `client_secret`, and `refresh_token`; the access-token response must grant
only `calendar.events.readonly`. Recurring events are expanded within the requested window, with at
most ten pages per calendar and one 30-second deadline for refresh and collection. Google tasks are
unsupported. Provider failures use fixed details without credentials or response bodies.

Enable `[brief.trigger] enabled = true` to use `vpt brief --upcoming [--json|--toon|--yaml]`. The default lead time is
`24h`; discovery selects events with `now <= start < now + lead_time`. A writing run saves its start
before collecting context. Failed calendar discovery exits 1 and records the failed run without
saving an occasion or publishing a brief. Successful discovery writes each unbriefed provider identity
once and returns `{"schema":"vpt.result/1","command":"brief","written":["<occasion-id>", ...]}`.
Dam labels are persisted as occasion tags and
can select confirmed transcript tags. Rescheduling updates the occasion while keeping its identity,
pinned path, saved brief, and completion marker.

Each newly published automatic brief logs one `brief_written` report with its `occasion` identity,
counts, and path, and invokes the configured report command. The completion marker and event are
committed together after publication. Repeated invocations and startup recovery do not resend reports.
Requested briefs emit no automatic notification.

## Analysis

`vpt analyze <id> [--dry-run] [--json|--toon|--yaml]` uses the configured `[analysis] command` argument list.
An empty command disables analysis and explicit invocation returns a configuration error. No model
vendor is built in. The command receives `{transcript}`, `{id}`, `{language}`, `{captured_at}`,
`{known_tags}`, and `{refused_tags}` substitutions and exact rendered transcript content on stdin; the
default deadline is 600 seconds.

`{captured_at}` expands to the recording's capture time in its local offset, as in
`2026-08-24T14:47:36-06:00`, so that the command can turn "by Thursday" into a date. It reaches only
the arguments, never stdin, so it does not change the recorded input digest.

`{known_tags}` expands to the indexed tags and `known-tags.txt` entries, most used first, each with
the number of indexed notes that carry it, as in `invoice (14), garden-club (6)`. It lists at most 300
tags. `{refused_tags}` lists each rename still in effect as `billing -> invoice`, then
each tag refused with `confirm <id> --reject-tag` that no note carries and no recording confirms. A tag
refused on one recording but carried elsewhere comes off that recording only and stays off the list.
Neither list holds the other's tags. An empty list expands to nothing. Each list is cut at a whole
entry, without a marker, so that it stays within 64 KiB and the expanded command within 1 MiB. Tags
never reach stdin, so a tag change does not change the recorded input digest or make `run` redo a
summary. A command without either placeholder runs exactly as before.

`vpt analyze --all [--since <RFC3339>] [--dry-run]` reruns analysis for every recording with an
accepted transcript, in capture order; `--since` keeps those captured at or after the time.
`vpt analyze --stdin [--dry-run]` reads one recording id per line instead, so `vpt list` or
`vpt search` output can choose the set. Both rerun the command even when the saved analysis is current,
so a changed prompt reaches old recordings, and both carry on past a recording that fails. The result
lists each recording with `summary_changed`, `tags_added` and `tags_removed` counts; tag names stay out
of it. When any recording fails, the error document names the failed ids in `ids`, the finished ones
in `completed`, and each failure's kind and message in `diagnostics`. A redo drops tags and related
links an earlier proposal added automatically when the new proposal omits them; hand-set tags and
decisions stay. `--dry-run` lists the recordings
without starting the command.

An action may name whose it is with `owner`: `"me"` when the speaker will do it, `{"other": "Dana"}`
when another person will, or `{"other": null}` when the recording gives no name. A missing or null
`owner` means unknown. The analysis note shows it after the action's sources as `[owner:: me]`,
`[owner:: Dana]` or `[owner:: someone else]`, a Dataview-style inline field whose name is escaped like
any proposal text. `verify-note` does not read the field as part of the claim.

An action may also carry `due`, as in `{"date": "2026-11-19", "time": "15:00", "sources":
["04:12-04:19"]}`, for a due date said in the recording; `time` may be null or left out. A date that
is not a real `YYYY-MM-DD` date, or a time that is not `HH:MM`, makes the document invalid. vpt checks
the due date's sources the way it checks a claim's: with no source it is `unsourced`, and with a source
that does not parse or ends past the recording it is `bad-reference`. Either finding withholds the
date: the action stays, the date is saved with its finding, and neither the note nor `show` gives the
date. A source on flagged text keeps the date and records `built-on-flagged-text`. The note shows a
kept date after the owner as `[due:: 2026-11-19]` or `[due:: 2026-11-19 15:00]`.

VPT grounds the returned `vpt.proposal/1`, then saves its proposal, analysis findings, tags, relations,
and note publications together. New analysis has Summary and Actions sections. Repeated analysis
keeps pinned paths, operator prose/tags, and review decisions for unchanged claims. Failed or invalid
agent output preserves prior accepted analysis. A transcript edited during the agent command is
preserved and the new proposal is refused. Retrying pending note or proposal vocabulary publication
repairs saved output without rerunning the agent when its source and grounding inputs remain current.
This also covers a vocabulary acknowledgement failure after both notes finished publishing.

Proposed tags use the indexed vocabulary and `tags.new_tags = hold|write`; automatic additions to
`known-tags.txt` are journaled with the notes. VPT remembers which lines it appended. When a later
analysis drops such a tag and no recording and no other indexed note carries it, the same journaled
write removes the line. Only tags VPT learned after this version are ever removed from `known-tags.txt`;
lines the owner typed, tags confirmed by hand, and renamed lines stay. Rejections log their proposal index and rule without
private tag text. A proposed tag that `vpt tag rename` renamed away, by key, becomes its new name,
following chains, and logs `tag_renamed` with its proposal index; confirming the old name by hand on
any recording ends that rename's effect. Related suggestions use indexed names/aliases or recording identities, preserve
provenance, and respect the configured cap. Manual tag and relation decisions survive later proposals.
Dry runs render an input plan without starting an agent or creating notes, logs, or directories.
Transcript, lexical, or claim changes invalidate saved findings and remove them from active review counts.
Note rewrites check claim freshness before rendering counts, including when readability settings change.
Note rewrites remove stale analysis annotations. Rechecking with `verify-note` or `analyze` retains
review decisions for unchanged claims. Older saved findings without a recorded input basis need rechecking.
`run` rechecks stale findings and checks made with a different grounding window, including empty
finding sets and when analysis is disabled. Rechecking preserves decisions for unchanged claims
and updates both notes' counts. Expired notes remain expired. Saved checks without a recorded window
require rechecking; VPT does not assume they used the current default. `run` recomputes enabled analysis
when the rendered input or grounding settings change. Analysis review alerts follow publication and
include verification class counts. Pipeline alerts combine the final counts once per recording;
standalone analysis points to the analysis note. Recovery does not resend earlier hooks.

## Analysis harnesses

Any agent command line can write the analysis. vpt runs the configured argument list directly, with no
shell, and inherits its own environment, so the agent finds its login through `HOME` as usual. It
writes the transcript's content region on standard input and adds no prompt of its own. The command
must carry its own prompt, exit 0, and print exactly one `vpt.proposal/1` JSON document of at most
1 MiB on standard output. Any other text on standard output, including a code fence, fails with
`analysis output document is invalid`. Standard error is captured and not shown. The agent needs no
tools; give it none.

vpt grounds every claim against the transcript, so the prompt must ask for sources in its terms. A
source is a `mm:ss-mm:ss` range taken from the `[mm:ss]` timecodes, with the end after the start and
no later than the recording's length. Every capitalized word and every number in a claim must occur
in the cited text, otherwise the claim is marked `unsupported`. A claim that starts with "They" or
"An" is marked for that reason.

Both examples below ran end to end against a synthetic recording, four times each, with Claude Code
2.1.322 and codex-cli 0.167.0. Claude took 20 to 40 seconds a call and Codex about 11, and each
produced an analysis note with no findings. The prompt is the same in both.

`claude -p` with structured output prints only the document. `--safe-mode` skips the user's hooks,
plugins and `CLAUDE.md`, and `--tools ""` removes every tool. `--json-schema` takes the schema as a
string, not a path.

```toml
[analysis]
command = [
  "claude", "-p", "--safe-mode", "--tools", "", "--no-session-persistence",
  "--system-prompt", 'Standard input is a meeting transcript. Each paragraph starts with an [mm:ss] timecode. Write a vpt.proposal/1 JSON document about it. Set schema to "vpt.proposal/1" and by to the name of your model. Put the key points in summary and each agreed task in actions, one short sentence per item. Set the owner of each action to "me" when the speaker will do it, to {"other": name} when another person will, with the name null when the transcript does not give it, and to null when it is unclear. Give an action a due date only when the transcript says one, as {"date": "YYYY-MM-DD", "time": "HH:MM" or null, "sources": the ranges that say it}, and null otherwise; never guess one. The recording was captured on {captured_at}. Give every item one or more sources, each a mm:ss-mm:ss range that starts at the timecode of the paragraph that supports it and ends at the timecode of the next paragraph, or one second after its own timecode for the last paragraph. State only what the transcript says, and use a capitalized word or a number only when it appears in the paragraphs you cite. Leave relations empty. Add up to five tags, each with a short reason; prefer these existing tags: {known_tags}. Never use these tags: {refused_tags}. Print the JSON document and nothing else, with no code fence.',
  "--json-schema", '{"type":"object","additionalProperties":false,"required":["schema","by","summary","actions","tags","relations"],"properties":{"schema":{"type":"string","enum":["vpt.proposal/1"]},"by":{"type":"string"},"summary":{"type":"array","items":{"type":"object","additionalProperties":false,"required":["text","sources"],"properties":{"text":{"type":"string"},"sources":{"type":"array","items":{"type":"string","pattern":"^[0-9]{2,}:[0-5][0-9]-[0-9]{2,}:[0-5][0-9]$"}}}}},"actions":{"type":"array","items":{"type":"object","additionalProperties":false,"required":["text","sources","owner","due"],"properties":{"text":{"type":"string"},"sources":{"type":"array","items":{"type":"string","pattern":"^[0-9]{2,}:[0-5][0-9]-[0-9]{2,}:[0-5][0-9]$"}},"owner":{"anyOf":[{"type":"string","enum":["me"]},{"type":"object","additionalProperties":false,"required":["other"],"properties":{"other":{"type":["string","null"]}}},{"type":"null"}]},"due":{"anyOf":[{"type":"object","additionalProperties":false,"required":["date","time","sources"],"properties":{"date":{"type":"string","pattern":"^[0-9]{4}-[0-9]{2}-[0-9]{2}$"},"time":{"type":["string","null"],"pattern":"^([01][0-9]|2[0-3]):[0-5][0-9]$"},"sources":{"type":"array","items":{"type":"string","pattern":"^[0-9]{2,}:[0-5][0-9]-[0-9]{2,}:[0-5][0-9]$"}}}},{"type":"null"}]}}}},"tags":{"type":"array","items":{"type":"object","additionalProperties":false,"required":["tag","reason"],"properties":{"tag":{"type":"string"},"reason":{"type":"string"}}}},"relations":{"type":"array","items":{"type":"object","additionalProperties":false,"required":["kind","target","reason"],"properties":{"kind":{"type":"string"},"target":{"type":"string"},"reason":{"type":"string"}}}}}}',
]
```

An account manager can go in front of the harness, so that the argument list starts with it. For
example, `"clauth", "start", "<profile>", "-p", ...` in place of `"claude", "-p", ...` ran the same way and
pins the call to one account. vpt has no defaults for any such tool, so the account belongs in the
user's own configuration.

`codex exec` prints only the final message on standard output and appends standard input to the
prompt. Its `--output-schema` takes a file; copy
[`docs/examples/vpt-proposal.schema.json`](docs/examples/vpt-proposal.schema.json) somewhere stable
and give its absolute path. `--ignore-user-config` skips the user's `config.toml`, so the model is
Codex's default; add `--model` to pick another. Codex has no switch to remove its shell, so
`--sandbox read-only` keeps it from writing.

```toml
[analysis]
command = [
  "codex", "exec", "--ephemeral", "--ignore-user-config", "--skip-git-repo-check",
  "--sandbox", "read-only", "--color", "never",
  "--output-schema", "/absolute/path/to/vpt-proposal.schema.json",
  'Standard input is a meeting transcript. Each paragraph starts with an [mm:ss] timecode. Write a vpt.proposal/1 JSON document about it. Set schema to "vpt.proposal/1" and by to the name of your model. Put the key points in summary and each agreed task in actions, one short sentence per item. Set the owner of each action to "me" when the speaker will do it, to {"other": name} when another person will, with the name null when the transcript does not give it, and to null when it is unclear. Give an action a due date only when the transcript says one, as {"date": "YYYY-MM-DD", "time": "HH:MM" or null, "sources": the ranges that say it}, and null otherwise; never guess one. The recording was captured on {captured_at}. Give every item one or more sources, each a mm:ss-mm:ss range that starts at the timecode of the paragraph that supports it and ends at the timecode of the next paragraph, or one second after its own timecode for the last paragraph. State only what the transcript says, and use a capitalized word or a number only when it appears in the paragraphs you cite. Leave relations empty. Add up to five tags, each with a short reason; prefer these existing tags: {known_tags}. Never use these tags: {refused_tags}. Print the JSON document and nothing else, with no code fence.',
]
```

A bad setup fails without touching the earlier analysis:

- `analysis command is unavailable`: the program is not on `PATH`.
- `analysis command exited unsuccessfully`: the harness failed, for example when it is not logged
  in under the `HOME` vpt runs with, or when `--json-schema` is given a path.
- `analysis output document is invalid`: standard output held anything besides the document.

## Note verification

`vpt verify-note <path> --recording <id> [--json|--toon|--yaml]` checks Markdown claims against the recording's
accepted transcript and current lexical flags. Findings are `unsourced`, `bad-reference`, `unsupported`,
or `built-on-flagged-text`, in that priority order. The default citation tolerance is 15 seconds,
configured by `analysis.grounding_window_secs`; no agent or transcription engine is run.

External notes are report-only. Annotation requires the registered transcript or analysis path inside
its configured store, with no symlink traversal, matching recording/stage metadata, and valid managed
regions. A different recording identity is an ownership refusal. Renamed notes become writable after
their new pin is committed by note recovery. Findings alone are a successful result.

Owned-note verification reads the current content, preserves operator prose and metadata, replaces
only that artifact's finding set, and retains resolutions for unchanged claims. Counts and annotations
are saved with the note before publication. A correction stays visible as an escaped annotation;
confirmed or dismissed findings lose their marker. Lexical corrections remain part of transcript claims.
Finding corrections carry an invisible role marker so claim edits cannot confuse them with lexical
corrections; older unmarked finding corrections are recognized by their saved claim and resolution.
JSON reports finding classes, one-based lines, and
cited time ranges without copying claim text. Interrupted publication can be recovered on retry.

Transcript-note updates derive a `Continues` link from the immediate predecessor by capture instant
and identity when its end precedes the new recording within `relations.session_gap_minutes` (default
60). An overlapping predecessor produces no automatic continuation. Setting the window to zero
disables derivation. `run` refreshes changed continuation links without rerunning successful engines.

Saved relations render in the managed links region, preserving transcript content and operator prose.
Links follow the configured Markdown/wiki style and saved recording pins. Related suggestions show
escaped provenance and honor `relations.max_suggested` (default five).

Confirmed whole-token terms produce `mentions` links when they uniquely match an indexed note name
or alias. VPT saves every matched accepted-word range; ambiguous matches save their ranges without
a link. Review confirmations include newly learned terms immediately. `run` refreshes changed
matches without rerunning engines, and indexed external notes remain untouched.

`vpt confirm <id> --relation <kind>:<target> [--json|--toon|--yaml]` confirms `continues`, `mentions`, `related`,
or `supersedes`. Targets can be recording IDs or unique indexed note names, aliases, or absolute
paths. `continues` and `supersedes` require a recording ID or a validated ledger-owned note.
Ambiguous, unknown, and self targets are refused. Confirmation preserves existing proposal provenance
and updates the available transcript and analysis notes atomically with saved relation state and
history. It also works before transcription and leaves expired notes expired.

## Tag confirmation

`vpt confirm <id> --tag <tag> [--json|--toon|--yaml]` confirms a tag, updates `known-tags.txt`, and refreshes the
transcript and existing analysis notes when accepted text is available. It works without analysis or an engine. Existing
vocabulary spelling is retained; new multiword tags become kebab-case. The configured confirmation
limit applies without removing earlier tags. Repeating a confirmation keeps its saved tag timestamp
and avoids rewriting an unchanged vocabulary file.

`vpt confirm <id> --reject-tag <tag> [--json|--toon|--yaml]` removes the tag from managed tags or suggestions while
retaining operator tags and the global vocabulary. Expired notes remain expired; explicit `note write`
can regenerate them. Tag state, note bytes, and pending vocabulary bytes commit together, and startup
recovery completes interrupted publication while preserving operator edits.

`vpt tag rename <old> <new> [--dry-run] [--json|--toon|--yaml]` renames one tag everywhere vpt owns
it: the saved tag state of every recording, the transcript and analysis notes, and `known-tags.txt`.
The ledger remembers the rename. Case and separator variants of `<old>` match. When a recording
already carries `<new>`, the two merge into one entry; a hand-set state beats one a summary set, and
otherwise the existing `<new>` entry wins. Each recording's tag state and note bytes commit together.
A recording that fails is listed in the error's `ids`, the others still change, the command exits
non-zero, and running the same command again finishes the rename. Brief notes are not renamed,
because a brief's tags are its meeting's labels and the next brief would restore them; the result
counts briefs that carry `<old>` as `untouched_briefs`. Expired notes and notes vpt did not create stay
untouched, and `untouched_notes` counts the other indexed notes that still carry `<old>`. `--dry-run` lists the notes it would rewrite and changes nothing. A tag nothing carries
exits 0 with nothing changed; a malformed name, a missing name, or two names that normalize to the
same tag exit 2.

Vocabulary comes from the configured Obsidian vault, or the portable transcript/analysis/brief
stores, plus `known-tags.txt`. Indexed external notes remain unchanged.

## Released copies

`vpt redact <id> [--stage transcript|analysis] [--to <directory>] [--title <title>] [--json|--toon|--yaml]`
creates a released copy and a separate private report. Transcript is the default source. The source
note stays unchanged. Copies contain minimal frontmatter and managed content with confirmed terms,
indexed names/aliases, and configured pattern classes masked. Open flagged spans are omitted by default;
`[share.redact] flagged_spans = "mark"` retains them with an unverified annotation. A term, note name or
pattern value that the paragraphs would only reveal in parts, such as a phone number or a two-word term
split between two transcript segments or around an unverified annotation, is refused (3, `residue`)
before anything is written, even when the same value is masked whole elsewhere. The refusal's
`diagnostics` name the timecodes of the paragraphs the value spans, such as `between 00:05 and 00:10`,
never the value. Resolving the flag, or editing the transcript so the value sits in one segment, clears
it.

Masking covers only what vpt was told about: the configured pattern classes, the confirmed terms in
`known-terms.txt`, and the names and aliases of indexed notes. A person or company named in a recording
that is on none of those lists stays readable, in a released copy and under `vpt show --redact` alike.
A redacted copy has its known items removed; it is not anonymous. Read it before sharing it, or confirm
the names as terms first with `vpt confirm --term`. `vpt redact` lists the capitalized words that
survived as `candidates` to help with that.

Copies go to `[stores] released`, or the directory supplied with `--to`. Destinations inside private
stores, state, configuration, or Voice Memos are refused. Existing copies are never overwritten;
identical bytes anywhere directly in the destination cause a duplicate refusal. JSON and human output
list paths, aggregate mask counts, omitted/marked counts, surviving capitalized candidates, and any
filename collision. Private mask mappings stay in the mode-0600 report under `drafts`.

`[share] automatic = true` enables release creation during `run`, independently of analysis. The
transcript is released before analysis runs; analysis is released after its publication and verification.
Previously finalized matching content in the configured destination is skipped on subsequent runs.
Interrupted copy/report publication is recovered from saved bytes before new work. `run --dry-run`
reports pending redaction without creating files or changing release history.

## Searching transcripts and analysis

`vpt search <word>... [--limit <n>] [--json|--toon|--yaml]` finds every transcript segment and every
analysis summary or action line that holds all the words, in any letter case. A word also matches inside
a longer word. It reads the notes vpt last wrote, as saved in the ledger, and skips a note whose file is
no longer at its path, such as one retention moved to the Trash.

Hits come newest recording first, then in note order, transcript before analysis. `--limit` caps the
hits at a positive number, 50 by default, and `truncated` says whether more existed. Each hit carries
`moment`, `recording`, `title`, `stage` (`transcript` or `analysis`), `seconds`, `timecode` (`mm:ss`,
minutes past 59 kept as they are) and `quote`. `moment` is `<recording id>@<mm:ss>`. An analysis line
takes the start of its first source; an unsourced line has `null` for `moment`, `seconds` and
`timecode`. A quote keeps vpt's review markers, such as `[corrected: ...]` and `[unverified: ...]`, and
drops the analysis line's sources and findings. Human output prints one line per hit: moment, title,
stage and quote. Search runs no engine and writes nothing; without a ledger it finds nothing.

## Installation

Install the command and helper separately:

1. `cargo install --git https://github.com/webdavis/vpt vpt` installs `vpt` into `~/.cargo/bin`.
2. From the repository checkout, run `cd helper/vpt-macos && swift build -c release` and copy
   `.build/release/vpt-macos` beside `vpt` in `~/.cargo/bin` for Apple Speech and system Trash.

To install the current checkout, use `cargo install --locked --path crates/vpt` from the repository
root. Put the installation's `bin` directory on `PATH`; `[helper] path` can instead select an explicit
helper location. `vpt --version --json` reports both versions, and `vpt doctor` reports a missing or
incompatible helper with installation instructions.

Without the helper, configure Whisply or a command engine to transcribe. Ingestion, notes, briefs,
redaction, and `show --stage` remain available. Retention refuses removal when the helper is unavailable.
Apple transcription requires macOS 26 or newer and installed speech assets for the selected locale.

An external scheduler can invoke `vpt run` periodically. Each invocation finishes and releases its
write lock; later invocations skip completed unchanged stages and resume pending work. Configure the
same `HOME`, `PATH`, and `VPT_CONFIG` in the scheduler that you use interactively. VPT supplies no
daemon or scheduler configuration.

The optional installation acceptance suite exercises installed release binaries with isolated fixture
data. After installing both binaries into an isolated prefix, run:

```sh
VPT_TEST_INSTALL_ROOT=/absolute/install/prefix TMPDIR=/private/tmp \
  cargo test -p vpt --features dev-tools --test installation -- --ignored --test-threads=1
```

Implemented mutating commands log reports and alerts by default. Optional `[report].command` and
`[alert].command` argument lists receive a documented JSON event on standard input. Both commands
default to empty lists, so no notifications will be sent unless configured. The script chooses the
notification system and message format.

Each finished transcription logs one `transcribed` report after its transcript note is published, and
each finished analysis logs one `analyzed` report after its analysis note is published, from
`vpt transcribe`, `vpt analyze` and `vpt run` alike. Each is `state: "done"` with the `recording`
identity and `paths.note`, and invokes the configured report command, so a caller hears about
finished work without polling. A transcription that needs review sends its `review_needed` alert as
well.

History is saved in the private ledger before export to `events.jsonl` or hook delivery. Export or
hook failures add a safe warning and preserve the operation's result. A later mutating command repairs
an incomplete final log line, replays pending records with their original identifiers and occurrence
times, and summarizes the saved recovery evidence. It does not resend hooks or repeat completed work.
Dry runs leave history and logs unchanged.

Ledger-backed commands record a `config_refused` alert when store creation fails after the state
directory has been validated and locked. Configuration failures before root validation use command
output only. Ingestion repairs pending publications and retention before refusing an unavailable
source, and its error includes any work completed during that recovery.
