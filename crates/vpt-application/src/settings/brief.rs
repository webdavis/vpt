use std::time::Duration;
use vpt_domain::brief::Options;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BriefSettings {
    pub options: Options,
    pub trigger_enabled: bool,
    pub lead_time: Duration,
}

impl Default for BriefSettings {
    fn default() -> Self {
        Self {
            options: Options::default(),
            trigger_enabled: false,
            lead_time: Duration::from_secs(86_400),
        }
    }
}
