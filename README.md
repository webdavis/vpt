# vpt

The Voice Processing Tool. This first Stage 1 slice provides `vpt --version`, `vpt --version --json`,
and argument validation. The remaining Stage 1 commands and the macOS helper are under construction.
The design is [the approved specification](docs/superpowers/specs/2026-09-20-vpt-design.md).

## Build

`cargo build --locked` builds `target/debug/vpt`. `just ship` runs the workspace checks.

## Planned installation

These are the two installation steps for the completed Stage 1 release:

1. `cargo install --git https://github.com/webdavis/vpt vpt` installs `vpt` into `~/.cargo/bin`.
2. Once `helper/vpt-macos` ships, run `cd helper/vpt-macos && swift build -c release` and copy
   `.build/release/vpt-macos` beside `vpt` in `~/.cargo/bin` for the system Trash.

The reporting pipeline will log reports and alerts by default. Optional `[report].command` and
`[alert].command` argument lists will receive a documented JSON event on standard input. Both commands
default to empty lists, so no notifications will be sent unless configured. The script chooses the
notification system and message format.
