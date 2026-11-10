# Tasks

## Test suite

- The suites refuse a temp directory that sits behind a symlink. The macOS default
  `TMPDIR` under `/var/folders` is one, and every suite that opens a root then fails with
  `NotADirectory`. Run the suite with `TMPDIR="$(cd "$TMPDIR" && pwd -P)/"` until the fixtures
  resolve the path themselves.
- The sqlite ledger tests run each scenario in a supervised child with a 500 ms limit. Under
  competing load the children time out (`bounded ledger child: ... TimedOut`) and up to 42 of
  them fail; alone they pass. Either raise the limit or make it scale with load.
