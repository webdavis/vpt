#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NoteStage {
    Transcript,
    Analysis,
}

impl NoteStage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Transcript => "transcript",
            Self::Analysis => "analysis",
        }
    }
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "transcript" => Some(Self::Transcript),
            "analysis" => Some(Self::Analysis),
            _ => None,
        }
    }
}
