use super::*;
pub(in crate::context::google) fn validate(
    credentials: &GoogleCredentials,
    request: &ContextRequest<'_>,
) -> Result<(), ContextConfigError> {
    if request.window.is_none() {
        return Ok(());
    }
    if request.calendars.is_empty() || request.calendars.len() > 256 {
        return Err(ContextConfigError {
            key: "context.calendars",
        });
    }
    let mut bytes = 65_536usize;
    for calendar in request.calendars {
        bytes = bytes
            .checked_sub(calendar.len())
            .ok_or(ContextConfigError {
                key: "context.calendars",
            })?;
        text(calendar, "context.calendars")?;
        if calendar == "." || calendar == ".." {
            return Err(ContextConfigError {
                key: "context.calendars",
            });
        }
    }
    for (value, key) in [
        (&credentials.client_id, "context.google.client_id"),
        (&credentials.client_secret, "context.google.client_secret"),
        (&credentials.refresh_token, "context.google.refresh_token"),
    ] {
        text(value, key)?;
    }
    Ok(())
}
fn text(value: &str, key: &'static str) -> Result<(), ContextConfigError> {
    if value.trim().is_empty() || value.len() > 65_536 || value.chars().any(char::is_control) {
        return Err(ContextConfigError { key });
    }
    Ok(())
}
