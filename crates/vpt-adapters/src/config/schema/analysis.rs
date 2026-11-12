use super::{Kind, types::key};

pub(super) const KEYS: &[super::KeySpec] = &[
    key(
        "analysis.command",
        Kind::Argv,
        "[]",
        "the agent command receives the full transcript, the capture time through {captured_at} and, through {known_tags} and {refused_tags}, tag names; empty disables analysis only",
    ),
    key(
        "analysis.timeout_secs",
        Kind::PositiveInt,
        "600",
        "the command's deadline",
    ),
    key(
        "analysis.grounding_window_secs",
        Kind::PositiveInt,
        "15",
        "verify-note's tolerance around a cited span",
    ),
];
