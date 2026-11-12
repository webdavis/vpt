#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Bool,
    PositiveInt,
    NonNegativeInt,
    PositiveInt64,
    Ratio,
    RatioAboveZero,
    Duration,
    Enum(&'static [&'static str]),
    Text,
    Secret,
    TextList,
    Argv,
    ExitCodes,
    EnvSecret,
}

impl Kind {
    pub fn expected(self) -> &'static str {
        match self {
            Kind::Bool => "boolean",
            Kind::PositiveInt | Kind::NonNegativeInt | Kind::PositiveInt64 => "integer",
            Kind::Ratio | Kind::RatioAboveZero => "number",
            Kind::Duration => "duration",
            Kind::Enum(_) | Kind::Text | Kind::Secret | Kind::EnvSecret => "string",
            Kind::TextList | Kind::Argv => "list of strings",
            Kind::ExitCodes => "list of integers",
        }
    }
}

pub struct KeySpec {
    pub path: &'static str,
    pub kind: Kind,
    pub default: &'static str,
    pub comment: &'static str,
}

pub const STORE_KEYS: &[&str] = &[
    "audio",
    "transcripts",
    "analysis",
    "briefs",
    "engine_outputs",
    "drafts",
    "released",
];

pub(super) const fn key(
    path: &'static str,
    kind: Kind,
    default: &'static str,
    comment: &'static str,
) -> KeySpec {
    KeySpec {
        path,
        kind,
        default,
        comment,
    }
}
