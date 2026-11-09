use super::*;
use vpt_adapters::context::dam::DamContext;
use vpt_adapters::context::google::GoogleContext;
use vpt_application::ContextSource;
use vpt_application::ports::{
    ContextConfigError, ContextRequest, ContextSource as _, ContextWindow,
};
use vpt_domain::{context::ContextSnapshot, occasion::OccasionDetails, time::UtcInstant};
pub(super) fn validate(settings: &Settings) -> Result<(), Box<ErrorDocument>> {
    if settings.context.source != ContextSource::None && settings.context.calendars.is_empty() {
        return Err(Box::new(ErrorDocument::new(
            ErrorKind::Config,
            "context.calendars must select at least one calendar",
        )));
    }
    if settings.context.source == ContextSource::Dam && settings.context.projects.is_empty() {
        return Err(Box::new(ErrorDocument::new(
            ErrorKind::Config,
            "context.projects must select at least one project",
        )));
    }
    let request = request(
        settings,
        ContextWindow {
            start: UtcInstant { secs: 0 },
            end: UtcInstant { secs: 1 },
        },
    );
    match settings.context.source {
        ContextSource::None => Ok(()),
        ContextSource::Dam => DamContext {
            command: &settings.context.dam_command,
        }
        .validate(&request)
        .map_err(config_error),
        ContextSource::Google => GoogleContext {
            credentials: &settings.context.google,
        }
        .validate(&request)
        .map_err(config_error),
    }
}
pub(super) fn collect(
    settings: &Settings,
    occasion: &OccasionDetails,
) -> Result<ContextSnapshot, Box<ErrorDocument>> {
    if settings.context.source == ContextSource::None {
        return Ok(ContextSnapshot::default());
    }
    let start = occasion.at.unix_second();
    let duration =
        i64::try_from(occasion.duration_secs.unwrap_or(3600).max(1)).map_err(|_| invalid())?;
    let end = UtcInstant {
        secs: start.secs.checked_add(duration).ok_or_else(invalid)?,
    };
    collect_window(settings, ContextWindow { start, end })
}
pub(super) fn collect_window(
    settings: &Settings,
    window: ContextWindow,
) -> Result<ContextSnapshot, Box<ErrorDocument>> {
    let request = request(settings, window);
    match settings.context.source {
        ContextSource::None => Ok(ContextSnapshot::default()),
        ContextSource::Dam => DamContext {
            command: &settings.context.dam_command,
        }
        .collect(&request)
        .map_err(collection_error),
        ContextSource::Google => GoogleContext {
            credentials: &settings.context.google,
        }
        .collect(&request)
        .map_err(collection_error),
    }
}
fn request(settings: &Settings, window: ContextWindow) -> ContextRequest<'_> {
    ContextRequest {
        window: Some(window),
        tasks: true,
        calendars: &settings.context.calendars,
        projects: &settings.context.projects,
    }
}
fn config_error(error: ContextConfigError) -> Box<ErrorDocument> {
    Box::new(ErrorDocument::new(
        ErrorKind::Config,
        format!("{} is missing or invalid", error.key),
    ))
}
fn collection_error(error: vpt_application::ports::ContextError) -> Box<ErrorDocument> {
    use vpt_application::ports::ContextError;
    match error {
        ContextError::Config(error) => config_error(error),
        ContextError::Interrupted => {
            let mut error =
                ErrorDocument::new(ErrorKind::Command, "context collection interrupted");
            error.interrupted = true;
            Box::new(error)
        }
        ContextError::InvalidDocument => Box::new(ErrorDocument::new(
            ErrorKind::Command,
            "invalid context document",
        )),
    }
}
