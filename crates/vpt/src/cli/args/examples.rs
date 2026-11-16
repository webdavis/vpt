macro_rules! docs {
    () => {
        "Docs: https://github.com/webdavis/vpt#command-line"
    };
}

macro_rules! examples {
    ($($line:literal),+ $(,)?) => {
        concat!("Examples:", $("\n  ", $line),+, "\n\n", docs!())
    };
}

pub(super) const DOCS: &str = docs!();

pub(super) const RUN: &str = examples!["vpt run", "vpt run --preview", "vpt run --json"];
pub(super) const SETUP: &str = examples![
    "vpt setup",
    "vpt setup --engine apple",
    "vpt setup --overwrite"
];
pub(super) const DOCTOR: &str = examples![
    "vpt doctor",
    "vpt doctor helper engines",
    "vpt doctor --fix --json"
];
pub(super) const TEST: &str = examples![
    "vpt test",
    "vpt test engines speakers",
    "vpt test summary --json"
];
pub(super) const INGEST: &str = examples![
    "vpt ingest",
    "vpt ingest --preview",
    "vpt ingest --once ~/Desktop/memo.m4a --json",
];
pub(super) const IMPORT: &str = examples![
    "vpt import ~/Downloads/interview.m4a",
    "vpt import talk.mp4 --title \"Design review\" --captured-at 2026-08-24T14:47:00-06:00",
];
pub(super) const SHOW: &str = examples![
    "vpt show 2026-08-24T144736-4f3ab19c02de",
    "vpt show 2026-08-24T144736-4f3ab19c02de --stage transcript",
    "vpt show 2026-08-24T144736-4f3ab19c02de --stage analysis --redact --json",
];
pub(super) const TRANSCRIBE: &str = examples![
    "vpt transcribe 2026-08-24T144736-4f3ab19c02de",
    "vpt transcribe 2026-08-24T144736-4f3ab19c02de --language fr-CA",
    "vpt transcribe 2026-08-24T144736-4f3ab19c02de --preview --json",
];
pub(super) const REVIEW: &str = examples![
    "vpt review 2026-08-24T144736-4f3ab19c02de",
    "vpt review 2026-08-24T144736-4f3ab19c02de --json",
    "vpt review 2026-08-24T144736-4f3ab19c02de --resolve \"$FLAG\" --correct \"Kubernetes\"",
];
pub(super) const CONFIRM: &str = examples![
    "vpt confirm --term \"Kubernetes\"",
    "vpt confirm 2026-08-24T144736-4f3ab19c02de --tag planning",
    "vpt confirm 2026-08-24T144736-4f3ab19c02de --relation continues:2026-08-20T091500-0a1b2c3d4e5f",
];
pub(super) const TAG_RENAME: &str = examples![
    "vpt tag rename plannning planning --preview",
    "vpt tag rename plannning planning",
];
pub(super) const ANALYZE: &str = examples![
    "vpt analyze 2026-08-24T144736-4f3ab19c02de",
    "vpt analyze --all --since 2026-08-01T00:00:00Z --preview",
    "vpt list --stage analysis | cut -d' ' -f1 | vpt analyze --stdin",
];
pub(super) const PATH: &str = examples![
    "vpt path 2026-08-24T144736-4f3ab19c02de --stage transcript",
    "vpt path --occasion 2026-08-25T100000-9f8e7d6c5b4a --stage brief",
];
pub(super) const NOTE_WRITE: &str = examples![
    "vpt note write 2026-08-24T144736-4f3ab19c02de",
    "vpt note write 2026-08-24T144736-4f3ab19c02de --json",
];
pub(super) const VERIFY_NOTE: &str = examples![
    "vpt verify-note notes/standup.md --recording 2026-08-24T144736-4f3ab19c02de",
    "vpt verify-note notes/standup.md --recording 2026-08-24T144736-4f3ab19c02de --json",
];
pub(super) const OCCASIONS: &str = examples!["vpt occasions", "vpt occasions --json"];
pub(super) const LIST: &str = examples!["vpt list", "vpt list --stage analysis", "vpt list --json"];
pub(super) const SEARCH: &str = examples![
    "vpt search budget",
    "vpt search quarterly budget --limit 10",
    "vpt search -- --force",
];
pub(super) const STORAGE: &str = examples!["vpt storage", "vpt storage --json"];
pub(super) const RETENTION_RUN: &str =
    examples!["vpt retention run --preview", "vpt retention run"];
pub(super) const BRIEF: &str = examples![
    "vpt brief 2026-08-25T100000-9f8e7d6c5b4a --explain",
    "vpt brief --title \"Design review\" --at 2026-08-25T10:00:00-06:00 --participant Alice",
    "vpt brief --upcoming --json",
];
pub(super) const REDACT: &str = examples![
    "vpt redact 2026-08-24T144736-4f3ab19c02de",
    "vpt redact 2026-08-24T144736-4f3ab19c02de --stage analysis --to ~/Shared",
];
pub(super) const SYMLINK_DEPLOY: &str =
    examples!["vpt symlink deploy", "vpt symlink deploy --json"];
pub(super) const SYMLINK_VERIFY: &str =
    examples!["vpt symlink verify", "vpt symlink verify --json"];
pub(super) const LEDGER_SHOW: &str = examples!["vpt ledger show", "vpt ledger show --json"];
pub(super) const LEDGER_ACCEPT: &str = examples![
    "vpt ledger accept --preview",
    "vpt ledger accept",
    "vpt ledger accept --preview --json",
];
pub(super) const VERSION: &str = examples!["vpt version", "vpt version --json"];
pub(super) const COMPLETION: &str = examples![
    "vpt completion zsh > \"${fpath[1]}/_vpt\"",
    "vpt completion bash > ~/.local/share/bash-completion/completions/vpt",
    "vpt completion fish > ~/.config/fish/completions/vpt.fish",
];
