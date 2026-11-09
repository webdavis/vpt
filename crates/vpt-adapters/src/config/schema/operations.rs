use super::{Kind, types::key};

pub(super) const KEYS: &[super::KeySpec] = &[
    key(
        "report.command",
        Kind::Argv,
        "[]",
        "optional routine-summary argv; receives event JSON on stdin",
    ),
    key(
        "alert.command",
        Kind::Argv,
        "[]",
        "optional failure-or-review argv; receives event JSON on stdin",
    ),
    key(
        "alert.aggregate_after",
        Kind::PositiveInt,
        "3",
        "recordings needing review in one run before one aggregate event",
    ),
    key(
        "retention.enabled",
        Kind::Bool,
        "false",
        "the opt-in retention run",
    ),
    key(
        "retention.include_audio",
        Kind::Bool,
        "false",
        "let the audio store expire",
    ),
    key(
        "retention.hold.audio",
        Kind::Duration,
        "0",
        "files older than this move to the Trash; 0 never",
    ),
    key(
        "retention.hold.transcripts",
        Kind::Duration,
        "0",
        "as above",
    ),
    key("retention.hold.analysis", Kind::Duration, "0", "as above"),
    key("retention.hold.briefs", Kind::Duration, "0", "as above"),
    key(
        "retention.hold.engine_outputs",
        Kind::Duration,
        "0",
        "as above",
    ),
    key("retention.hold.drafts", Kind::Duration, "0", "as above"),
    key("retention.hold.released", Kind::Duration, "0", "as above"),
];
