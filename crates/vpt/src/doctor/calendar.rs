use super::checks::Context;
use super::parts::{command, not_configured, not_tested};
use super::report::Row;
use vpt_application::ContextSource;

pub(super) fn rows(context: &Context) -> Vec<Row> {
    let settings = &context.settings.context;
    let reached = match settings.source {
        ContextSource::None => return vec![not_configured("source", "context.type")],
        ContextSource::Dam => vec![
            Row::ok("source", "dam"),
            command(
                "command",
                &settings.dam_command,
                "context.dam.command",
                "vpt asks it for events; whether it contacts a remote is up to that command",
            ),
        ],
        ContextSource::Google => vec![
            Row::ok(
                "source",
                "google; vpt asks Google Calendar for events and sends it the credentials",
            ),
            credentials(context),
        ],
    };
    reached
        .into_iter()
        .chain([not_tested("calendar")])
        .collect()
}

fn credentials(context: &Context) -> Row {
    let google = &context.settings.context.google;
    let missing: Vec<&str> = [
        ("client_id", &google.client_id),
        ("client_secret", &google.client_secret),
        ("refresh_token", &google.refresh_token),
    ]
    .into_iter()
    .filter(|(_, value)| value.is_empty())
    .map(|(name, _)| name)
    .collect();
    if missing.is_empty() {
        return Row::ok(
            "credentials",
            "client_id, client_secret and refresh_token set",
        );
    }
    let keys: Vec<String> = missing
        .iter()
        .map(|name| format!("`context.google.{name}`"))
        .collect();
    Row::issue("credentials", format!("missing {}", missing.join(", ")))
        .advice(format!("set {} in the config", keys.join(" and ")))
}
