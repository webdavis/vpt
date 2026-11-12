use super::{Kind, types::key};

pub(super) const KEYS: &[super::KeySpec] = &[
    key(
        "recording.language",
        Kind::Text,
        "\"en-US\"",
        "the language engines are asked for",
    ),
    key(
        "engines.main",
        Kind::Text,
        "\"apple\"",
        "the transcript of record",
    ),
    key(
        "engines.checker",
        Kind::Text,
        "\"\"",
        "the second slot; empty runs one engine",
    ),
    key(
        "engines.checker_runs",
        Kind::Enum(&["always", "on_failure"]),
        "\"always\"",
        "always or on_failure",
    ),
    key(
        "engines.allow_same_family",
        Kind::Bool,
        "false",
        "let two engines of one family run",
    ),
    key(
        "engines.timeout_secs",
        Kind::PositiveInt,
        "1800",
        "per-engine wall-clock deadline",
    ),
    key(
        "engines.apple.kind",
        Kind::Enum(&["apple"]),
        "\"apple\"",
        "the Apple Speech adapter through the helper",
    ),
    key(
        "engines.whisply.kind",
        Kind::Enum(&["whisply"]),
        "\"whisply\"",
        "the whisply adapter",
    ),
    key(
        "engines.whisply.command",
        Kind::Argv,
        "[\"whisply\"]",
        "how whisply is invoked",
    ),
    key(
        "engines.whisply.device",
        Kind::Text,
        "\"mlx\"",
        "mlx reports no confidence, cpu does",
    ),
    key(
        "engines.whisply.model",
        Kind::Text,
        "\"large-v3-turbo\"",
        "the model name",
    ),
    key(
        "engines.whisply.hf_token",
        Kind::Secret,
        "\"\"",
        "a Hugging Face token; when set, whisply labels speakers",
    ),
    key(
        "reconcile.confidence_floor",
        Kind::Ratio,
        "0.35",
        "word confidence below this is flagged",
    ),
    key(
        "reconcile.max_divergence_ratio",
        Kind::RatioAboveZero,
        "0.35",
        "past this edit-distance ratio, one divergent flag",
    ),
    key(
        "reconcile.known_terms_path",
        Kind::Text,
        "\"~/.config/vpt/known-terms.txt\"",
        "the confirmed-terms list",
    ),
];
