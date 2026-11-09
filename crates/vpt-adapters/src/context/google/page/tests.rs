use super::*;
use serde_json::{Value, json};

fn event() -> Value {
    json!({"id":"provider-event","status":"confirmed","summary":"Private planning",
        "start":{"dateTime":"2026-10-01T10:00:00-06:00"},
        "end":{"dateTime":"2026-10-01T10:30:00-06:00"},
        "attendees":[{"email":"guest@example.org","displayName":"Private guest"}],
        "organizer":{"email":"host@example.org","displayName":"Private host"}})
}
fn bytes(items: Vec<Value>) -> Vec<u8> {
    serde_json::to_vec(&json!({"kind":"calendar#events","timeZone":"America/Denver","items":items}))
        .unwrap()
}

#[test]
fn page_retains_provider_calendar_identity_and_exact_participants() {
    let page = decode(&bytes(vec![event()]), "private-calendar").unwrap();
    let row = &page.occasions[0];
    assert_eq!(row.id, "provider-event");
    assert_eq!(row.calendar.as_deref(), Some("private-calendar"));
    assert_eq!(row.details.duration_secs, Some(1800));
    assert_eq!(row.details.at.as_str(), "2026-10-01T10:00:00-06:00");
    assert_eq!(row.details.participants[0].name, "Private guest");
    assert_eq!(
        row.details.participants[1].email.as_deref(),
        Some("host@example.org")
    );
    assert!(!format!("{page:?}").contains("Private"));
}

#[test]
fn all_day_and_local_datetime_use_calendar_zone_and_cancelled_instances_are_omitted() {
    let mut day = event();
    day["id"] = json!("day");
    day["start"] = json!({"date":"2026-11-01"});
    day["end"] = json!({"date":"2026-11-02"});
    let mut local = event();
    local["id"] = json!("local");
    local["start"] = json!({"dateTime":"2026-10-01T10:00:00","timeZone":"America/Denver"});
    let cancelled = json!({"id":"cancelled","status":"cancelled"});
    let page = decode(&bytes(vec![day, local, cancelled]), "private-calendar").unwrap();
    assert_eq!(page.occasions.len(), 2);
    assert_eq!(page.occasions[0].details.duration_secs, Some(25 * 3600));
    assert_eq!(
        page.occasions[1].details.at.as_str(),
        "2026-10-01T10:00:00-06:00"
    );
}

#[test]
fn empty_pages_retain_continuation_tokens_and_invalid_provider_data_is_private() {
    let page = decode(br#"{"items":[],"nextPageToken":"PRIVATE-page"}"#, "cal").unwrap();
    assert_eq!(page.next.as_deref(), Some("PRIVATE-page"));
    assert!(!format!("{page:?}").contains("PRIVATE"));
    for value in [
        json!({"error":{"message":"PRIVATE"}}),
        json!({"items":null}),
        json!({"items":[],"nextPageToken":""}),
    ] {
        assert!(
            !format!(
                "{:?}",
                decode(value.to_string().as_bytes(), "cal").unwrap_err()
            )
            .contains("PRIVATE")
        );
    }
    let mut row = event();
    row["end"] = json!({"dateTime":"2026-09-01T10:00:00Z"});
    assert!(decode(&bytes(vec![row]), "cal").is_err());
}
