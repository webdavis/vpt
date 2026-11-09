use super::{Kind, types::key};

pub(super) const KEYS: &[super::KeySpec] = &[
    key(
        "share.automatic",
        Kind::Bool,
        "false",
        "redact every recording's notes during vpt run",
    ),
    key(
        "share.source_line",
        Kind::Text,
        "\"redacted extract, not a verbatim record\"",
        "the one frontmatter line telling a recipient what they hold",
    ),
    key(
        "share.include_date",
        Kind::Bool,
        "false",
        "put the capture date in the released frontmatter",
    ),
    key(
        "share.keep_timecodes",
        Kind::Bool,
        "false",
        "keep [mm:ss] references in released text",
    ),
    key(
        "share.deny_sections",
        Kind::TextList,
        "[]",
        "sections dropped from the content region before redaction",
    ),
    key(
        "share.redact.classes",
        Kind::TextList,
        "[\"email\", \"url\", \"phone\", \"number\", \"money\"]",
        "pattern classes; terms and note names are always masked",
    ),
    key(
        "share.redact.mask",
        Kind::Text,
        "\"[{class} {n}]\"",
        "rendered in place of a removed value",
    ),
    key(
        "share.redact.flagged_spans",
        Kind::Enum(&["omit", "mark"]),
        "\"omit\"",
        "omit or mark",
    ),
    key(
        "handoff.allow_private",
        Kind::Bool,
        "false",
        "let transcripts, analysis notes and briefs be handed off",
    ),
];
