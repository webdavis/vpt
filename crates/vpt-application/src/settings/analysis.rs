use std::{fmt, time::Duration};

#[derive(Clone, PartialEq, Eq)]
pub struct AnalysisSettings {
    pub command: Vec<String>,
    pub timeout: Duration,
    pub grounding_window_secs: u32,
}

impl Default for AnalysisSettings {
    fn default() -> Self {
        Self {
            command: vec![],
            timeout: Duration::from_secs(600),
            grounding_window_secs: 15,
        }
    }
}

impl fmt::Debug for AnalysisSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnalysisSettings")
            .field("arguments", &self.command.len())
            .field("timeout", &self.timeout)
            .field("grounding_window_secs", &self.grounding_window_secs)
            .finish()
    }
}
