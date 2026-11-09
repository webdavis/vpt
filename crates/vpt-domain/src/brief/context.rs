#[derive(Clone, PartialEq, Eq)]
pub enum ContextStatus {
    Present,
    Disabled,
    Unsupported,
    Failed { detail: String },
}
impl ContextStatus {
    pub fn status(&self) -> &'static str {
        if matches!(self, Self::Present) {
            "present"
        } else {
            "absent"
        }
    }
    pub fn reason(&self) -> Option<&'static str> {
        match self {
            Self::Present => None,
            Self::Disabled => Some("disabled"),
            Self::Unsupported => Some("unsupported"),
            Self::Failed { .. } => Some("failed"),
        }
    }
    pub fn detail(&self) -> Option<&str> {
        match self {
            Self::Failed { detail } => Some(detail),
            _ => None,
        }
    }
}
impl std::fmt::Debug for ContextStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Present => "Present",
            Self::Disabled => "Disabled",
            Self::Unsupported => "Unsupported",
            Self::Failed { .. } => "Failed",
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextSummary {
    pub calendar: ContextStatus,
    pub tasks: ContextStatus,
}
impl Default for ContextSummary {
    fn default() -> Self {
        Self {
            calendar: ContextStatus::Disabled,
            tasks: ContextStatus::Disabled,
        }
    }
}
mod task;
pub use task::ContextTask;
