use super::*;
use vpt_application::{ContextSettings, ContextSource, GoogleCredentials};
pub(super) fn from_table(table: &Table) -> Result<ContextSettings, ConfigError> {
    let source = match text(table, "context.type")? {
        "none" => ContextSource::None,
        "dam" => ContextSource::Dam,
        "google" => ContextSource::Google,
        _ => {
            return Err(ConfigError::OutOfRange {
                key: "context.type".into(),
                rule: "none, dam, or google".into(),
            });
        }
    };
    Ok(ContextSettings {
        source,
        calendars: scopes(table, "context.calendars")?,
        projects: scopes(table, "context.projects")?,
        dam_command: strings(table, "context.dam.command")?,
        dam_max_age: strings(table, "context.dam.max_age")?,
        google: GoogleCredentials {
            client_id: text(table, "context.google.client_id")?.into(),
            client_secret: text(table, "context.google.client_secret")?.into(),
            refresh_token: text(table, "context.google.refresh_token")?.into(),
        },
    })
}
fn scopes(table: &Table, key: &str) -> Result<Vec<String>, ConfigError> {
    let values = strings(table, key)?;
    if values.len() > 256
        || values.iter().any(|value| {
            value.is_empty() || value.chars().any(char::is_control) || value.len() > 65_536
        })
    {
        return Err(ConfigError::OutOfRange {
            key: key.into(),
            rule:
                "at most 256 nonempty scopes without control characters, each at most 65536 bytes"
                    .into(),
        });
    }
    Ok(values)
}
