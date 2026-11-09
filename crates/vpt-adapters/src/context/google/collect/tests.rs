use super::*;
use serde_json::json;
use vpt_domain::{brief::ContextStatus, time::UtcInstant};

pub(super) fn credentials() -> GoogleCredentials {
    GoogleCredentials {
        client_id: "PRIVATE-id".into(),
        client_secret: "PRIVATE-secret".into(),
        refresh_token: "PRIVATE-refresh".into(),
    }
}
pub(super) fn request(calendars: &[String]) -> ContextRequest<'_> {
    ContextRequest {
        window: Some(ContextWindow {
            start: UtcInstant { secs: 1000 },
            end: UtcInstant { secs: 2000 },
        }),
        tasks: true,
        calendars,
        projects: &[],
    }
}
pub(super) fn token(scope: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({"access_token":"PRIVATE-token","token_type":"Bearer","expires_in":3600,"scope":scope})).unwrap()
}
pub(super) fn page(id: &str, at: i64, next: Option<&str>) -> Vec<u8> {
    let mut value = json!({"items":[{"id":id,"summary":"Private event","start":{"dateTime":UtcInstant {secs:at}.rfc3339()},"end":{"dateTime":UtcInstant {secs:at+60}.rfc3339()}}]});
    if let Some(next) = next {
        value["nextPageToken"] = json!(next);
    }
    serde_json::to_vec(&value).unwrap()
}

#[test]
fn collection_scopes_every_page_and_filters_by_start_after_pagination() {
    let calendars = ["one@example.org".into(), "two@example.org".into()];
    let mut calls = Vec::new();
    let snapshot = run(
        &credentials(),
        &request(&calendars),
        |call, _| match call {
            Request::Refresh(value) => {
                assert_eq!(value.client_id, "PRIVATE-id");
                Ok(token(token::SCOPE))
            }
            Request::Events {
                token,
                calendar,
                window,
                page: cursor,
            } => {
                assert_eq!(token, "PRIVATE-token");
                assert_eq!(window.start.secs, 1000);
                calls.push((calendar.to_owned(), cursor.map(str::to_owned)));
                Ok(match cursor {
                    None => page("early", 999, Some("next")),
                    Some("next") => page("at-start", 1000, Some("last")),
                    _ => page("at-end", 2000, None),
                })
            }
        },
        Instant::now,
        || false,
    )
    .unwrap();
    assert_eq!(snapshot.status.calendar, ContextStatus::Present);
    assert_eq!(snapshot.status.tasks, ContextStatus::Unsupported);
    assert_eq!(snapshot.occasions.len(), 2);
    assert_eq!(calls.len(), 6);
    assert_ne!(snapshot.occasions[0].key(), snapshot.occasions[1].key());
}

#[test]
fn broader_token_scope_stops_before_any_calendar_request() {
    let mut calls = 0;
    let snapshot = run(
        &credentials(),
        &request(&["cal".into()]),
        |_, _| {
            calls += 1;
            Ok(token("https://www.googleapis.com/auth/calendar"))
        },
        Instant::now,
        || false,
    )
    .unwrap();
    assert_eq!(calls, 1);
    assert!(matches!(
        snapshot.status.calendar,
        ContextStatus::Failed { .. }
    ));
    assert!(snapshot.occasions.is_empty());
}

#[test]
fn a_tenth_page_with_more_results_marks_the_whole_discovery_failed() {
    let mut pages = 0;
    let snapshot = run(
        &credentials(),
        &request(&["cal".into()]),
        |call, _| match call {
            Request::Refresh(_) => Ok(token(token::SCOPE)),
            _ => {
                pages += 1;
                Ok(page(
                    &format!("event-{pages}"),
                    1000,
                    Some(&format!("next-{pages}")),
                ))
            }
        },
        Instant::now,
        || false,
    )
    .unwrap();
    assert_eq!(pages, 10);
    assert!(snapshot.occasions.is_empty());
    assert_eq!(
        snapshot.status.calendar.detail(),
        Some(Failure::PageLimit.detail())
    );
}

#[test]
fn unrequested_calendar_has_no_credentials_or_selection_requirement() {
    let snapshot = run(
        &GoogleCredentials {
            client_id: String::new(),
            client_secret: String::new(),
            refresh_token: String::new(),
        },
        &ContextRequest {
            window: None,
            tasks: true,
            calendars: &[],
            projects: &[],
        },
        |_, _| panic!("unexpected network request"),
        Instant::now,
        || false,
    )
    .unwrap();
    assert_eq!(snapshot.status.calendar, ContextStatus::Disabled);
    assert_eq!(snapshot.status.tasks, ContextStatus::Unsupported);
}
