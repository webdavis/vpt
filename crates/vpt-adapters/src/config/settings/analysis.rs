use super::*;
use std::time::Duration;
use vpt_application::AnalysisSettings;

pub(super) fn from_table(table: &Table) -> Result<AnalysisSettings, ConfigError> {
    let command = strings(table, "analysis.command")?;
    if !command.is_empty() && !crate::arguments::valid(&command, 1_048_576) {
        return Err(ConfigError::OutOfRange {
            key: "analysis.command".into(),
            rule: "must contain a nonempty program and at most 256 NUL-free arguments totaling at most 1 MiB".into(),
        });
    }
    Ok(AnalysisSettings {
        command,
        timeout: Duration::from_secs(u64::from(small(table, "analysis.timeout_secs")?)),
        grounding_window_secs: small(table, "analysis.grounding_window_secs")?,
    })
}
