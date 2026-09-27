# vpt

The Voice Processing Tool is under construction. Implemented commands include `vpt --version`,
`vpt --version --json`, and `vpt setup`. The macOS helper provides version reporting and Trash
operations. The remaining Stage 1 commands are unfinished.
The design is [the approved specification](docs/superpowers/specs/2026-09-20-vpt-design.md).

## Build

`cargo build --locked` builds `target/debug/vpt`. The helper requires Xcode and Swift 6.
`just swift-build` builds it with isolated project-local caches. `just ship` runs the Rust and Swift
workspace checks.

## Planned installation

These are the two installation steps for the completed Stage 1 release:

1. `cargo install --git https://github.com/webdavis/vpt vpt` installs `vpt` into `~/.cargo/bin`.
2. From the repository checkout, run `cd helper/vpt-macos && swift build -c release` and copy
   `.build/release/vpt-macos` beside `vpt` in `~/.cargo/bin` for the system Trash.

The reporting pipeline will log reports and alerts by default. Optional `[report].command` and
`[alert].command` argument lists will receive a documented JSON event on standard input. Both commands
default to empty lists, so no notifications will be sent unless configured. The script chooses the
notification system and message format.
