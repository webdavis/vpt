#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageState {
    Pending,
    Succeeded,
    Failed,
    Disabled,
    Expired,
}

impl StageState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Disabled => "disabled",
            Self::Expired => "expired",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "pending" => Some(Self::Pending),
            "succeeded" => Some(Self::Succeeded),
            "failed" => Some(Self::Failed),
            "disabled" => Some(Self::Disabled),
            "expired" => Some(Self::Expired),
            _ => None,
        }
    }
}
