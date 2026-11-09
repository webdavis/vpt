use super::{Kind, types::key};

pub(super) const KEYS: &[super::KeySpec] = &[
    key(
        "config_version",
        Kind::PositiveInt,
        "1",
        "the configuration schema version",
    ),
    key(
        "home.path",
        Kind::Text,
        "\"~/.vpt\"",
        "the home; every store defaults under it",
    ),
    key(
        "home.state_dir",
        Kind::Text,
        "\"~/.local/state/vpt\"",
        "the ledger's directory",
    ),
    key(
        "home.symlink_target",
        Kind::Text,
        "\"\"",
        "when set, ~/.vpt is a managed symlink to this directory",
    ),
    key(
        "helper.path",
        Kind::Text,
        "\"vpt-macos\"",
        "the macOS helper, resolved on PATH when relative",
    ),
    key(
        "stores.audio",
        Kind::Text,
        "\"<home>/audio\"",
        "archive clones",
    ),
    key(
        "stores.transcripts",
        Kind::Text,
        "\"<home>/transcripts\"",
        "transcript notes",
    ),
    key(
        "stores.analysis",
        Kind::Text,
        "\"<home>/analysis\"",
        "synthesis notes",
    ),
    key("stores.briefs", Kind::Text, "\"<home>/briefs\"", "briefs"),
    key(
        "stores.engine_outputs",
        Kind::Text,
        "\"<home>/engine-outputs\"",
        "raw engine documents",
    ),
    key(
        "stores.drafts",
        Kind::Text,
        "\"<home>/drafts\"",
        "private redaction reports",
    ),
    key(
        "stores.released",
        Kind::Text,
        "\"<home>/released\"",
        "the shared folder",
    ),
    key(
        "source.recordings_dir",
        Kind::Text,
        "\"~/Library/Group Containers/group.com.apple.VoiceMemos.shared/Recordings\"",
        "Apple's store, read read-only",
    ),
    key(
        "source.read_titles",
        Kind::Bool,
        "true",
        "read titles from a private copy of the database",
    ),
    key(
        "source.quiet_period_secs",
        Kind::PositiveInt,
        "30",
        "seconds a file must be unchanged before it is at rest",
    ),
    key(
        "source.deferral_page_threshold",
        Kind::PositiveInt,
        "4",
        "consecutive deferrals of one recording before one event",
    ),
    key(
        "source.max_audio_bytes",
        Kind::PositiveInt64,
        "2147483648",
        "a larger source file defers before any content is read",
    ),
];
