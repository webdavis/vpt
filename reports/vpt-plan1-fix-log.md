# Stage 1 plan review repairs

One line per finding: applied or declined, the commit sha once committed, and a reason when declined or
applied in a different form. The statuses describe repairs to the plan, not implemented product features.
The first review covered `153eddd`. Pass 2 covered `4a72f2f` and found residual defects; its dispositions
and verification limits are recorded below.

- F01: applied cee0a80, a7411f6: prior interface repairs through Task 24, 0f0e4eb shared diagnostics and
  read-only interfaces, 09bebc1 Tasks 25 and 26, and runtime and retention repairs through Task 32
- F02: applied cee0a80, a7411f6: prior module-registration repairs through Task 24, 09bebc1 private
  helper tests, and runtime and retention tasks register and select every new test before the red run
- F03: applied f7f3f67 (test moved to Task 1), 589cb20 (Task 2 wording)
- F04: applied cee0a80: 589cb20 Task 2 and the Task 30 replacement emitter retain stdout write failures;
  injectable-writer tests cover both output modes
- F05: applied 0d52377 (schema.rs with schema/types.rs and schema/dynamic.rs)
- F06: applied 0d52377 (quoted() through serde_json, round-trip test)
- F07: applied 0d52377 (table_is_declared, WrongType for a table where a scalar belongs, two tests)
- F08: applied 0d52377 (fixed sentence on both parse sites, Unreadable carries the io kind, canary test)
- F09: applied 0d52377 (four kinds) and 01bb10d (boundary tests over u32)
- F10: applied 01bb10d (number() widens integers for both ratio kinds)
- F11: applied 6a44793: (resolve writes nothing; Roots::create_state_dir and create_leaves; container
  metadata and nonexistence assertions)
- F12: applied 6a44793, cee0a80: (resolve takes config_dir; RootName::Config; Tasks 27 and 30 pass the
  selected file's parent)
- F13: applied 6a44793: (Roots.container is the parent of recordings_dir; fixtures use
  voice-memos/Recordings)
- F14: applied 6a44793: (the refusal names source.recordings_dir first)
- F15: applied fc02c04 SetupOutcome derives Debug (Task 6), 67fd6db SqliteLedger derives Debug (Task 11)
- F16: applied fc02c04: (the check is exact code with SetupWriter imported)
- F17: applied fc02c04: (run takes config: Option\<&Path>; dispatch passes invocation.config)
- F18: applied fc02c04: (Sandbox::vpt() setsid in pre_exec; libc dev-dependency; the pty test builds its
  own script command)
- F19: applied fc02c04: (let vpt)
- F20: applied fc02c04: (zero-length read is Closed)
- F21: applied fc02c04: (set_permissions 0600 on the file and 0700 on directories, tests for both)
- F22: applied fc02c04: (write_config takes force; create_new(!force); AlreadyExists is Exists)
- F23: applied 09bebc1: 633f045 Task 7, 971b3c0 Task 8, 58a44d4 Task 9, 32c2f6e Task 19; Task 26 uses
  2026-09-16T22:53:20Z for 1_789_599_200
- F24: applied 633f045
- F25: applied 633f045 Task 7 age_secs and the expected 29, e24d803 Task 10 nanosecond rest case
- F26: applied 26b95d3: 633f045 Task 7, 971b3c0 Task 8, 58a44d4 Task 9, 6dbfba7 Task 20 maps an
  unrepresentable capture to invalid_container; 26b95d3 fixes the far-future fixture to use a zero offset
  so it cannot fall back into year 9999
- F27: applied 58a44d4
- F28: applied 58a44d4
- F29: applied 58a44d4 in the F79 form: BoxReader removed, inspect takes len and a read callback
- F30: applied a7411f6: 48de0b7 Task 12 and Task 31 contract macros import their invocation parent; the
  retention factories reuse open_ledger and fresh
- F31: applied 48de0b7
- F32: applied 48de0b7
- F33: applied bca62c2 Task 13 and eb02c7e Task 24 (Controls with cancellation, clock, wait and grace; no
  test touches the production flag or asserts elapsed time; zero grace in group tests)
- F34: applied 345453a (the journal fake and the imports are exact; DirtyPublication comes from ports)
- F35: applied 345453a (Stores::digest_bytes, RepairError::RenderedDigestMismatch, mismatch test)
- F36: applied 757b59f (FileExt::read_exact_at)
- F37: applied in the moved form: 757b59f and cbf8508 (title arrives with Task 16 on the trait), 85021c0
  and 7a6f95f (publish and sync_existing arrive with Task 18 on the trait); no stand-in remains, the
  numbering is kept
- F38: applied cbf8508 (a separate state temp dir, sorted names compared)
- F39: applied cbf8508 (the fixture keeps its connection, the live WAL is asserted nonzero before
  refresh)
- F40: applied cbf8508 (backslash escaped, ESCAPE in the printed query, no prose amendment)
- F41: applied 85021c0 Task 17 StageFailure.owned_staging, 32c2f6e Task 19 owned cleanup on every exit
  before publication, 7d15839 Task 21 fault cases
- F42: applied 7a6f95f (no link-and-unlink fallback; the typed failure keeps staging for the owned
  cleanup)
- F43: applied 32c2f6e, 6dbfba7, 7d15839, 76c3fd4, 9405d1c (every ingest test imports Archive explicitly)
- F44: applied cee0a80, a7411f6: 527ce14 and 6dbfba7 use fixed fixture times; Tasks 27 and 32 now derive
  child clocks and mtimes from FIXED_NOW_SECS
- F45: applied 6dbfba7 (Task 23 passes the full mode to every call)
- F46: applied 527ce14 (ingested requires no deferral reason) and 6dbfba7 (edit, fresh deferral, clock
  advance, second recording)
- F47: applied 527ce14 (unchanged refreshes last_seen and clears source_gone_at, not in a dry run)
- F48: applied 6dbfba7 (saturating_add, page when previous < threshold \<= count)
- F49: applied 7d15839 (resolve_existing) and 76c3fd4 (recover_orphans syncs the archive before the
  commit)
- F50: applied 633f045 Civil::instant and is_valid, 971b3c0 parse_local_timestamp, 76c3fd4 Task 22
  archived_offset with recovery tested at 19_800 and -21_600
- F51: applied cee0a80: 32c2f6e and 9405d1c carry the complete Mode; Task 27 passes dry_run and once
  together, with a binary acceptance test
- F52: applied cee0a80: 32c2f6e and 9405d1c suppress dry-run events; Task 27 covers invalid roots and
  unreadable sources without notification or state creation
- F53: applied 9405d1c, completed c746712: pass 2 reproduced the ordinary-disappearance contradiction;
  Task 22 now adds a surviving recording after the first sweep and before removing the original source
- F54: applied 9405d1c (two new files, b's target prepopulated, exactly a's identity in completed)
- F55: applied 9405d1c (by_id through ? in would_ingest)
- F56: applied eb02c7e, completed c746712: pass 2 found a leader's exit status still overrode later
  deadline and interruption; the repaired executor replaces that status while descendant pipe workers
  remain
- F57: applied 09bebc1: Task 25 bounded deserialization checks depth and the next array entry before
  decoding children, checks text and keys before retention, and requires the end of the document
- F58: applied 09bebc1: Task 25 captures the 65,537th byte so a valid oversized prefix is refused
- F59: applied cee0a80, a7411f6: 09bebc1 records escaped additive field names for all helper replies;
  Tasks 27, 30 and 32 retain diagnostic drains in final output
- F60: applied cee0a80, a7411f6: 09bebc1 caches successful compatibility before notify and Trash; 0f0e4eb
  and Tasks 27 and 32 preserve the typed cleanup refusal and map it to helper_version, exit 3; optional
  notification failures preserve work status under spec section 8.7
- F61: applied 09bebc1: Task 25 accepts Trash confirmation only for the exact requested path
- F62: applied 09bebc1: Task 25 version dispatch honors the selected configuration and its helper path
- F63: applied 09bebc1: Task 5 and Task 26 use notify-command, with no notification engine named
- F64: applied 09bebc1: Task 26 substitutes tokens once without rescanning inserted text
- F65: applied cee0a80, a7411f6: 09bebc1 records command failure before one fallback and disables absent
  desktop delivery after one diagnostic; Tasks 27, 30 and 32 retain diagnostics in the single final
  document or human log
- F66: applied cee0a80, a7411f6, completed c746712: pass 2 found read-only shared-memory writes; writable
  connections now preserve sidecars, read-only connections require them and use read-only shared memory,
  with snapshots during access and after close
- F67: applied cee0a80: Task 29 accepts relative equivalent links, rejects dangling verification and
  regular-file targets, and creates the approved missing target on deploy
- F68: applied cee0a80: Task 30 checks cleanup through open_read_only, treating absent audio as an empty
  observation without creating it
- F69: applied cee0a80: Task 30 runs separate git_tree, output and doctor test commands, with no
  filtering that silently selects zero tests
- F70: applied cee0a80: Task 30 always emits the same 18 named checks, running independent checks and
  reporting dependency failures explicitly
- F71: applied a7411f6: Task 31 compares i128 nanoseconds; tests include u64::MAX holds, extreme instants
  and one nanosecond before expiry
- F72: applied a7411f6: Task 32 carries RecordingId with every move and serializes completed recording
  identities rather than paths
- F73: applied a7411f6: Task 32 records each successful move before row or journal updates; both failure
  cases retain completed progress
- F74: applied a7411f6: Task 32 carries structured progress through partial reconciliation and startup
  failure, combines it with new work, and emits one retention event
- F75: applied a7411f6: Task 32 accepts only a successful helper preflight; absent, mismatched, malformed
  and failed version checks stop removal
- F76: applied a7411f6: Task 32 prints the complete borrowing-correct retention implementation, with
  direct report.failure calls
- F77: applied a7411f6: Task 33 requires stable gates and a separate installed-nightly per-test timing
  run, rejecting every test reaching one second
- F78: applied cee0a80, a7411f6, completed c746712: pass 2 exposed gaps in configuration, setup, ledger
  reconnects, title copies, archive publication and cleanup; retained-root checks and typed containment
  refusals now cover those operations
- F79: applied a7411f6: prior generic port repairs, 0f0e4eb unsized composition-boundary bounds, and Task
  32 generic retention and reconciliation; no application-owned dynamic dispatch or unused renderer
  abstraction remains
- F80: applied 48de0b7 in the LedgerCommit form: one commit carries recordings, seen rows and journal
  entries; the rollback scenario runs on both implementations
- F81: applied 6dbfba7 (dataless_never_opens, oversize_never_reads, source_change_discards_stage,
  invalid_stage_is_trashed, absent_trash_preserves_private_stage) and 7d15839 (exdev_uses_bounded_copy,
  enospc_aborts_without_row, stage_sync_failure_commits_no_row, directory_sync_failure_commits_no_row);
  each double records operations and injects the named failure, assertions cover rows, publication,
  cleanup and event count
- F82: applied 32c2f6e whole-container snapshot under a titled sweep, 9405d1c the dry run keeps the
  prepopulated copy byte for byte with its mtime and mode
- F83: applied 345453a (per-instance sequence, exclusive creation, only AlreadyExists retried, blockers
  kept)
- F84: applied 32c2f6e IngestReport::completed and 9405d1c the recovery-then-failure test
- F85: applied cee0a80: 09bebc1 fixes structural error redaction with canary tests; Task 30 consumes only
  sanitized helper errors
- F86: applied cee0a80, a7411f6: 0f0e4eb, 09bebc1 and Tasks 27 through 33 keep implementation modules
  private with curated exports; config, domain and protocol capability APIs remain intentional
- F87: applied 757b59f (EXDEV only; StorageFull at creation and on write is NoSpace)
- F88: applied: e24d803 Task 10, 67fd6db Task 11 flags column, 48de0b7 Task 12 SeenRow.flags with checked
  codecs, 9242fcb Task 15, 32c2f6e Task 19 and 6dbfba7 Task 20 carry candidate.flags; 0f0e4eb tests
  preservation of an additional flag alongside the dataless bit
- F89: applied a7411f6: Task 34 is removed, its verification precedes the Task 33 commit, and delivery is
  an unnumbered checklist

Original review: 89 applied dispositions, none declined. The earlier closure summary was premature: pass
2 returned CHANGES with 13 SEV-1, 16 SEV-2 and 2 SEV-3 findings on 2026-09-22.

## Pass 2 dispositions

All 31 plan repairs below are in `c746712`. None were declined or left partial. The approved
specification is unchanged. This records repairs after a CHANGES verdict, not a new whole-plan APPROVE
verdict.

| Finding | Repair                                                                                                            |
| ------- | ----------------------------------------------------------------------------------------------------------------- |
| R01     | Missing-home store exceptions accept one normal component only.                                                   |
| R02     | Prospective configuration directories resolve existing ancestors and preserve failures.                           |
| R03     | Configuration leaves open through the checked root; containment errors retain exit 3.                             |
| R04     | Nonblocking opens reject a named pipe before waiting for a writer.                                                |
| R05     | Forced setup validates the descriptor before truncation and refuses leaf links.                                   |
| R06     | Setup resolves all roots before prompting or writing and retains checked ancestors.                               |
| R07     | SQLite retains its state root and rechecks each connection; composition shares one root with its lock and titles. |
| R08     | Read-only queries preserve database, write-ahead log and shared-memory files, including metadata.                 |
| R09     | The recorder assertion no longer requires an unavailable Debug implementation.                                    |
| R10     | The title fixture calls self::state() to avoid the local path binding.                                            |
| R11     | Gone-source recovery retains another recording; the separate empty-store case still fails.                        |
| R12     | Every non-dry sweep explicitly refreshes the private title copy.                                                  |
| R13     | Title state and copy roots are retained; all database leaves are checked.                                         |
| R14     | Clone identity, publication returns and pre-Trash paths are validated.                                            |
| R15     | Subdirectory counts use checked child handles.                                                                    |
| R16     | Known digests reuse their identity before new local-time derivation.                                              |
| R17     | Dry-run title lookup uses the current digest and rechecks descriptor metadata.                                    |
| R18     | The private executor exposes its usable capabilities; the unused interrupt setter is removed.                     |
| R19     | Task 25 inserts signal-handler installation as the first command-root statement.                                  |
| R20     | Deadline and interruption override a completed leader while pipe workers remain.                                  |
| R21     | Grace applies to the surviving process group.                                                                     |
| R22     | Process fixtures isolate their environment and use readiness-driven controls.                                     |
| R23     | Each command composes only its required capabilities; ingest startup failures retain their event and exit status. |
| R24     | Inventory rejects unrepresentable counts and byte totals.                                                         |
| R25     | Doctor checks resolved store ancestry, including when an unrelated root fails.                                    |
| R26     | Swift red runs include source targets; compiled tests also qualify VptMacos.run to avoid XCTest shadowing.        |
| R27     | Independent red commands both run; an existing dry-run guard is labeled accurately.                               |
| R28     | Value-type interfaces specify fields, receivers and return types.                                                 |
| R29     | All 34 tasks format before their green gate and commit.                                                           |
| R30     | Test-only declarations follow production items.                                                                   |
| R31     | Swift scratch fixtures register teardown.                                                                         |

## Verification

The review was static. The repair checks below executed extracted snippets in isolated scratch
workspaces. No vpt product source, installed binary, real Voice Memos data, notification or Trash
operation changed.

| Probe                                       | Executed result                                                                                                            |
| ------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| Configuration, checked files and SQLite     | 33 unit checks pass; formatting and all-target Clippy with warnings denied pass.                                           |
| Combined ingest and repaired SQLite         | 94 checks pass: 51 unit, 41 planned integration and 2 supplemental boundary checks; formatting and all-target Clippy pass. |
| Executor, helper and notification contracts | 57 checks pass on stable and nightly; maximum measured test time 0.071 seconds; formatting and Clippy pass.                |
| Command composition and inventory           | 12 checks pass against scripted capabilities and a real SQLite sidecar fixture; formatting and Clippy pass.                |
| Swift helper                                | 8 tests pass; maximum 0.009 seconds; release build, explicit dependency checks, SwiftFormat and SwiftLint pass.            |

Rust commands were `cargo test --offline` with the relevant library and integration targets,
`cargo clippy --offline --workspace --all-targets -- -D warnings`, and `cargo fmt --all -- --check`. The
helper's nightly timing command added `-Z unstable-options --report-time`. Swift checks used isolated
home, cache, build, configuration and security paths.

Negative checks reproduced the configuration symlink and missing-root defects, the blocked named pipe,
the gone-source contradiction, and the title/digest regressions. Deliberate mutations independently broke
retained-root checks, read-only shared-memory access, archive validation, title containment, subdirectory
traversal, process-group handling, capability selection, error mapping, inventory overflow handling and
resolved ancestry. Each changed control was restored before its final green run.

Independent bounded cross-reviews checked Tasks 5-6 and 11, Tasks 15-23, and Tasks 27-30 plus Task 32
consumers. Their remaining wording and file-map issues were corrected. Inspected cumulative Rust files
remain below 500 formatted lines. Markdown formatting, whitespace checks, and secret scanning pass; the
approved spec matches its original commit.

These checks do not replace the future five-crate workspace build or command acceptance suite. Production
signal-handler wiring is specified and statically checked, not exercised by the isolated executor tests.
Stage 1 implementation remains unstarted.
