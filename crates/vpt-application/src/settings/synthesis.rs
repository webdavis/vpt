use std::{fmt, time::Duration};

#[derive(Clone, PartialEq, Eq)]
pub struct SynthesisSettings {
    pub command: Vec<String>,
    pub timeout: Duration,
    pub grounding_window_secs: u32,
}

impl Default for SynthesisSettings {
    fn default() -> Self {
        Self {
            command: vec![],
            timeout: Duration::from_secs(600),
            grounding_window_secs: 15,
        }
    }
}

impl fmt::Debug for SynthesisSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SynthesisSettings")
            .field("arguments", &self.command.len())
            .field("timeout", &self.timeout)
            .field("grounding_window_secs", &self.grounding_window_secs)
            .finish()
    }
}
