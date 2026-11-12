# Tasks

## Test suite

- The suites refuse a temp directory that sits behind a symlink, and the macOS default `TMPDIR`
  under `/var/folders` is one. `just test` resolves the path before it runs the suites; a bare
  `cargo test` still fails with `NotADirectory` until the fixtures resolve the path themselves.
- Timed test children have a fixed 500 ms limit. They now run one at a time, which fixed the
  failures under parallel load, but a single child still overruns now and then on the CI runner.
  The limit is being measured there to size it as a hang watchdog.
- `crates/vpt-adapters/src/database/tests/support.rs` takes a second snapshot of `vpt.db-shm` times
  without flushing first, so it may see the same late write the command-test snapshot did.
