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

pub(super) const RUN: &str = examples!["vpt run", "vpt run --preview", "vpt run -o json"];
pub(super) const SETUP: &str = examples![
    "vpt setup",
    "vpt setup --engine apple",
    "vpt setup --overwrite"
];
pub(super) const DOCTOR: &str = examples![
    "vpt doctor",
    "vpt doctor helper engines",
    "vpt doctor --fix -o json"
];
pub(super) const TEST: &str = examples![
    "vpt test",
    "vpt test engines speakers",
    "vpt test summary -o json"
];
pub(super) const INGEST: &str = examples![
    "vpt ingest",
    "vpt ingest --preview",
    "vpt ingest --once ~/Desktop/memo.m4a -o json",
];
pub(super) const IMPORT: &str = examples![
    "vpt import ~/Downloads/interview.m4a",
    "vpt import talk.mp4 --title \"Design review\" --captured-at 2026-08-24T14:47:00-06:00",
];
pub(super) const SHOW: &str = examples![
    "vpt show 2026-08-24T144736-4f3ab19c02de",
    "vpt show 2026-08-24T144736-4f3ab19c02de --stage transcript",
    "vpt show 2026-08-24T144736-4f3ab19c02de --stage analysis --redact -o json",
];
pub(super) const TRANSCRIBE: &str = examples![
    "vpt transcribe 2026-08-24T144736-4f3ab19c02de",
    "vpt transcribe 2026-08-24T144736-4f3ab19c02de --language fr-CA",
    "vpt transcribe 2026-08-24T144736-4f3ab19c02de --preview -o json",
];
pub(super) const REVIEW_LIST: &str = examples![
    "vpt review list 2026-08-24T144736-4f3ab19c02de",
    "vpt review list 2026-08-24T144736-4f3ab19c02de -o json",
];
pub(super) const REVIEW_CONFIRM: &str = examples![
    "vpt review confirm 2026-08-24T144736-4f3ab19c02de \"$FLAG\"",
    "vpt review confirm 2026-08-24T144736-4f3ab19c02de \"$FLAG\" -o json",
];
pub(super) const REVIEW_CORRECT: &str = examples![
    "vpt review correct 2026-08-24T144736-4f3ab19c02de \"$FLAG\" Kubernetes",
    "vpt review correct 2026-08-24T144736-4f3ab19c02de \"$FLAG\" \"Kubernetes cluster\" -o json",
    "vpt review correct 2026-08-24T144736-4f3ab19c02de \"$FLAG\" -- \"-5 degrees\"",
];
pub(super) const REVIEW_DISMISS: &str = examples![
    "vpt review dismiss 2026-08-24T144736-4f3ab19c02de \"$FLAG\"",
    "vpt review dismiss 2026-08-24T144736-4f3ab19c02de \"$FLAG\" -o json",
];
pub(super) const CONFIRM_TERM: &str = examples![
    "vpt confirm term Kubernetes",
    "vpt confirm term \"Acme Robotics\" -o json",
];
pub(super) const CONFIRM_TAG: &str = examples![
    "vpt confirm tag 2026-08-24T144736-4f3ab19c02de planning",
    "vpt confirm tag 2026-08-24T144736-4f3ab19c02de planning -o json",
];
pub(super) const CONFIRM_REJECT_TAG: &str = examples![
    "vpt confirm reject-tag 2026-08-24T144736-4f3ab19c02de budget",
    "vpt confirm reject-tag 2026-08-24T144736-4f3ab19c02de budget -o json",
];
pub(super) const CONFIRM_RELATION: &str = examples![
    "vpt confirm relation 2026-08-24T144736-4f3ab19c02de continues:2026-08-20T091500-0a1b2c3d4e5f",
    "vpt confirm relation 2026-08-24T144736-4f3ab19c02de related:2026-08-20T091500-0a1b2c3d4e5f -o json",
];
pub(super) const TAG_RENAME: &str = examples![
    "vpt tag rename plannning planning --preview",
    "vpt tag rename plannning planning",
];
pub(super) const ANALYZE: &str = examples![
    "vpt analyze 2026-08-24T144736-4f3ab19c02de",
    "vpt analyze 2026-08-24T144736-4f3ab19c02de --preview -o json",
    "vpt list --stage analysis | cut -d' ' -f1 | vpt analyze -",
];
pub(super) const ANALYZE_ALL: &str = examples![
    "vpt analyze all",
    "vpt analyze all --since 2026-08-01T00:00:00Z --preview",
];
pub(super) const PATH: &str = examples![
    "vpt path 2026-08-24T144736-4f3ab19c02de --stage transcript",
    "vpt path 2026-08-24T144736-4f3ab19c02de --stage analysis -o json",
];
pub(super) const PATH_OCCASION: &str = examples![
    "vpt path occasion 2026-08-25T100000-9f8e7d6c5b4a --stage brief",
    "vpt path occasion 2026-08-25T100000-9f8e7d6c5b4a --stage brief -o json",
];
pub(super) const NOTE_WRITE: &str = examples![
    "vpt note write 2026-08-24T144736-4f3ab19c02de",
    "vpt note write 2026-08-24T144736-4f3ab19c02de -o json",
];
pub(super) const NOTE_VERIFY: &str = examples![
    "vpt note verify notes/standup.md --recording 2026-08-24T144736-4f3ab19c02de",
    "vpt note verify notes/standup.md --recording 2026-08-24T144736-4f3ab19c02de -o json",
];
pub(super) const ACTIONS_LIST: &str = examples![
    "vpt actions list",
    "vpt actions list 2026-08-24T144736-4f3ab19c02de",
    "vpt actions list -o json",
];
pub(super) const ACTIONS_RESEND: &str = examples![
    "vpt actions resend --preview",
    "vpt actions resend",
    "vpt actions resend 2026-08-24T144736-4f3ab19c02de -o json",
];
pub(super) const OCCASIONS: &str = examples!["vpt occasions", "vpt occasions -o json"];
pub(super) const LIST: &str =
    examples!["vpt list", "vpt list --stage analysis", "vpt list -o json"];
pub(super) const SEARCH: &str = examples![
    "vpt search budget",
    "vpt search quarterly budget --limit 10",
    "vpt search -- --force",
];
pub(super) const STORAGE: &str = examples!["vpt storage", "vpt storage -o json"];
pub(super) const RETENTION_RUN: &str =
    examples!["vpt retention run --preview", "vpt retention run"];
pub(super) const BRIEF: &str = examples![
    "vpt brief 2026-08-25T100000-9f8e7d6c5b4a --explain",
    "vpt brief --title \"Design review\" --at 2026-08-25T10:00:00-06:00 --participant Alice",
    "vpt brief 2026-08-25T100000-9f8e7d6c5b4a --preview -o json",
];
pub(super) const BRIEF_UPCOMING: &str =
    examples!["vpt brief upcoming", "vpt brief upcoming -o json"];
pub(super) const REDACT: &str = examples![
    "vpt redact 2026-08-24T144736-4f3ab19c02de",
    "vpt redact 2026-08-24T144736-4f3ab19c02de --stage analysis --to ~/Shared",
];
pub(super) const SYMLINK_DEPLOY: &str =
    examples!["vpt symlink deploy", "vpt symlink deploy -o json"];
pub(super) const SYMLINK_VERIFY: &str =
    examples!["vpt symlink verify", "vpt symlink verify -o json"];
pub(super) const LEDGER_SHOW: &str = examples!["vpt ledger show", "vpt ledger show -o json"];
pub(super) const LEDGER_ACCEPT: &str = examples![
    "vpt ledger accept --preview",
    "vpt ledger accept",
    "vpt ledger accept --preview -o json",
];
pub(super) const VERSION: &str = examples!["vpt version", "vpt version -o json"];
pub(super) const COMPLETION: &str = examples![
    "vpt completion zsh > \"${fpath[1]}/_vpt\"",
    "vpt completion bash > ~/.local/share/bash-completion/completions/vpt",
    "vpt completion fish > ~/.config/fish/completions/vpt.fish",
];
