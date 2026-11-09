use super::{Kind, types::key};

pub(super) const KEYS: &[super::KeySpec] = &[
    key(
        "readability.enabled",
        Kind::Bool,
        "false",
        "the readability pass over the note body",
    ),
    key(
        "note.profile",
        Kind::Enum(&["portable", "obsidian"]),
        "\"portable\"",
        "portable or obsidian",
    ),
    key(
        "note.link_style",
        Kind::Enum(&["markdown", "wiki"]),
        "\"markdown\"",
        "markdown or wiki",
    ),
    key(
        "note.slug_max_chars",
        Kind::PositiveInt,
        "60",
        "slug length cap",
    ),
    key(
        "note.obsidian.hub_transcripts",
        Kind::Text,
        "\"transcripts\"",
        "folder note name for hub",
    ),
    key(
        "note.obsidian.hub_analysis",
        Kind::Text,
        "\"analysis\"",
        "folder note name for hub",
    ),
    key(
        "note.obsidian.hub_briefs",
        Kind::Text,
        "\"briefs\"",
        "folder note name for hub",
    ),
    key(
        "note.obsidian.vault_root",
        Kind::Text,
        "\"\"",
        "the vault's root directory, required by the obsidian profile",
    ),
    key(
        "tags.new_tags",
        Kind::Enum(&["write", "hold"]),
        "\"hold\"",
        "write (automatic) or hold (suggested until confirmed)",
    ),
    key(
        "tags.known_tags_path",
        Kind::Text,
        "\"~/.config/vpt/known-tags.txt\"",
        "the confirmed-tags list",
    ),
    key(
        "tags.max_per_note",
        Kind::NonNegativeInt,
        "5",
        "confirmed tags per note",
    ),
    key(
        "tags.max_suggested",
        Kind::NonNegativeInt,
        "10",
        "held tags per note",
    ),
    key(
        "relations.session_gap_minutes",
        Kind::NonNegativeInt,
        "60",
        "continues window",
    ),
    key(
        "relations.max_suggested",
        Kind::NonNegativeInt,
        "5",
        "agent-proposed related links kept per note",
    ),
];
