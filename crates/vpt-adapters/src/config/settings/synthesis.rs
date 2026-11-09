use super::*;
use std::time::Duration;
use vpt_application::SynthesisSettings;

pub(super) fn from_table(table: &Table) -> Result<SynthesisSettings, ConfigError> {
    let command = strings(table, "synthesis.command")?;
    if !command.is_empty() && !crate::arguments::valid(&command, 1_048_576) {
        return Err(ConfigError::OutOfRange {
            key: "synthesis.command".into(),
            rule: "must contain a nonempty program and at most 256 NUL-free arguments totaling at most 1 MiB".into(),
        });
    }
    Ok(SynthesisSettings {
        command,
        timeout: Duration::from_secs(u64::from(small(table, "synthesis.timeout_secs")?)),
        grounding_window_secs: small(table, "synthesis.grounding_window_secs")?,
    })
}
