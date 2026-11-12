use super::{Kind, types::key};

pub(super) const KEYS: &[super::KeySpec] = &[
    key(
        "brief.lookback_days",
        Kind::NonNegativeInt,
        "180",
        "how far back selection reaches",
    ),
    key(
        "brief.max_notes",
        Kind::NonNegativeInt,
        "12",
        "selection cap",
    ),
    key(
        "brief.max_spans_per_note",
        Kind::NonNegativeInt,
        "5",
        "quoted spans per selected note",
    ),
    key(
        "brief.trigger.enabled",
        Kind::Bool,
        "false",
        "the calendar trigger behind --upcoming",
    ),
    key(
        "brief.trigger.lead_time",
        Kind::Duration,
        "\"24h\"",
        "how far ahead --upcoming looks",
    ),
    key(
        "context.type",
        Kind::Enum(&["none", "dam", "google"]),
        "\"none\"",
        "none, dam or google",
    ),
    key(
        "context.calendars",
        Kind::TextList,
        "[]",
        "dam path prefixes or Google calendar ids; empty refuses",
    ),
    key(
        "context.projects",
        Kind::TextList,
        "[]",
        "dam task path prefixes; empty refuses",
    ),
    key(
        "context.dam.command",
        Kind::Argv,
        "[\"dam\"]",
        "how dam is invoked",
    ),
    key(
        "context.dam.max_age",
        Kind::TextList,
        "[]",
        "<remote>=<duration> freshness bounds passed to dam agenda as --max-age",
    ),
    key(
        "context.google.client_id",
        Kind::Secret,
        "\"\"",
        "OAuth client id",
    ),
    key(
        "context.google.client_secret",
        Kind::Secret,
        "\"\"",
        "OAuth client secret",
    ),
    key(
        "context.google.refresh_token",
        Kind::Secret,
        "\"\"",
        "a refresh token bearing only the read-only events scope",
    ),
];
