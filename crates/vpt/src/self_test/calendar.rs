use super::plan::Run;
use crate::commands::brief::context::events;
use crate::report::Row;
use std::path::Path;
use std::time::Instant;
use vpt_adapters::SystemClock;
use vpt_application::ports::{Clock, ContextWindow};
use vpt_application::{ContextSource, Settings};
use vpt_domain::time::{Civil, UtcInstant};

pub(super) fn test(settings: &Settings) -> Result<Run<'_>, Row> {
    if settings.context.source == ContextSource::None {
        return Err(Row::not_run(
            "source",
            "not configured, set `context.type` in the config",
        ));
    }
    Ok(Box::new(|| vec![row(settings)]))
}

fn row(settings: &Settings) -> Row {
    let (source, reach, advice) = match settings.context.source {
        ContextSource::Google => (
            "Google Calendar".to_owned(),
            "vpt contacted Google Calendar and sent it the credentials",
            "see `vpt doctor calendar`; check the three Google keys and that the token grants \
             only calendar.events.readonly",
        ),
        _ => (
            format!(
                "`{}`",
                settings
                    .context
                    .dam_command
                    .first()
                    .and_then(|program| Path::new(program).file_name())
                    .map_or_default(|name| name.to_string_lossy())
            ),
            "whether it contacts a remote is up to that program",
            "see `vpt doctor calendar`, then run the calendar program by hand",
        ),
    };
    let started = Instant::now();
    let result = today().and_then(|window| {
        let snapshot = events(settings, window).map_err(|error| error.message)?;
        match snapshot.status.calendar.detail() {
            Some(detail) => Err(detail.to_owned()),
            None => Ok(snapshot.occasions.len()),
        }
    });
    let seconds = started.elapsed().as_secs_f64();
    match result {
        Ok(count) => {
            let plural = if count == 1 { "" } else { "s" };
            Row::ok(
                "events",
                format!("{count} event{plural} today from {source} in {seconds:.1} s; {reach}"),
            )
        }
        Err(reason) => Row::issue(
            "events",
            format!("failed after {seconds:.1} s asking {source}: {reason}; {reach}"),
        )
        .advice(advice),
    }
}

fn today() -> Result<ContextWindow, String> {
    let midnight = |at: UtcInstant| {
        let offset = SystemClock.offset_at(at);
        Civil {
            hour: 0,
            minute: 0,
            second: 0,
            ..at.civil(offset)
        }
        .instant(offset)
        .ok_or_else(|| "the current day cannot be represented".to_owned())
    };
    let start = midnight(SystemClock.now())?;
    let end = midnight(UtcInstant {
        secs: start.secs + 36 * 3_600,
    })?;
    Ok(ContextWindow { start, end })
}
