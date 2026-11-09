use super::*;
use serde_json::{Value, json};
use vpt_domain::brief::ContextStatus;

pub(super) fn event() -> Value {
    json!({"oid":"event-1","kind":"event","subject":"Private planning","path":"calendar/work/",
        "event":{"start":"2026-10-01T10:00:00-06:00[America/Denver]",
            "end":"2026-10-01T10:30:00-06:00[America/Denver]", "status":"confirmed",
            "attendees":[{"email":"guest@example.org","response":"accepted"}],
            "organizer":{"email":"host@example.org","name":"Private host"}}})
}
pub(super) fn task() -> Value {
    json!({"oid":"task-1","kind":"task","subject":"Private task\nexact text","path":"work/team/",
        "task":{"done":false,"priority":3,"due":"2026-10-02"}})
}
pub(super) fn bytes(rows: Vec<Value>) -> Vec<u8> {
    serde_json::to_vec(&json!({"objects":rows})).unwrap()
}

#[test]
fn dam_events_normalize_real_wire_fields_and_keep_names_and_emails() {
    let snapshot = decode::read(&bytes(vec![event()]), true, "calendar/").unwrap();
    assert_eq!(snapshot.status.calendar, ContextStatus::Present);
    assert_eq!(snapshot.status.tasks, ContextStatus::Disabled);
    let row = &snapshot.occasions[0];
    assert_eq!(row.id, "event-1");
    assert_eq!(row.details.title, "Private planning");
    assert_eq!(row.details.at.as_str(), "2026-10-01T10:00:00-06:00");
    assert_eq!(row.details.duration_secs, Some(1800));
    assert_eq!(
        row.details.participants[0].email.as_deref(),
        Some("guest@example.org")
    );
    assert_eq!(row.details.participants[1].name, "Private host");
    assert!(!format!("{snapshot:?}").contains("Private"));
}

#[test]
fn dam_event_labels_become_occasion_tags_and_invalid_labels_fail_privately() {
    let mut row = event();
    row["labels"] = json!(["Private project", "team/planning"]);
    let snapshot = decode::read(&bytes(vec![row.clone()]), true, "calendar/").unwrap();
    assert_eq!(
        snapshot.occasions[0].details.tags,
        ["Private project", "team/planning"]
    );
    assert!(!format!("{snapshot:?}").contains("Private"));
    row["labels"] = json!(["private\u{0}label"]);
    assert_eq!(
        decode::read(&bytes(vec![row]), true, "calendar/"),
        Err(Failure::Invalid)
    );
}

#[test]
fn dam_tasks_preserve_project_due_priority_and_exclude_completed_rows() {
    let mut done = task();
    done["oid"] = json!("done");
    done["task"]["done"] = json!(true);
    let snapshot = decode::read(&bytes(vec![task(), done]), false, "work/").unwrap();
    assert_eq!(snapshot.status.calendar, ContextStatus::Disabled);
    assert_eq!(snapshot.status.tasks, ContextStatus::Present);
    assert_eq!(snapshot.tasks.len(), 1);
    assert_eq!(snapshot.tasks[0].project, "work/team/");
    assert_eq!(snapshot.tasks[0].content, "Private task\nexact text");
    assert_eq!(snapshot.tasks[0].due.as_deref(), Some("2026-10-02"));
    assert_eq!(snapshot.tasks[0].priority, Some(3));
}

#[test]
fn dam_all_day_events_use_the_declared_zone_across_dst() {
    let mut row = event();
    row["event"]["start"] = json!("2026-11-01");
    row["event"]["end"] = json!("2026-11-02");
    row["event"]["timezone"] = json!("America/Denver");
    let snapshot = decode::read(&bytes(vec![row]), true, "calendar/").unwrap();
    assert_eq!(
        snapshot.occasions[0].details.at.as_str(),
        "2026-11-01T00:00:00-06:00"
    );
    assert_eq!(snapshot.occasions[0].details.duration_secs, Some(25 * 3600));
}

#[test]
fn malformed_wrong_kind_oversized_and_inconsistent_provider_data_fail_privately() {
    for payload in [
        bytes(vec![task()]),
        b"{\"objects\":{},\"private\":true}".to_vec(),
        vec![b' '; 65_537],
    ] {
        let error = decode::read(&payload, true, "calendar/").unwrap_err();
        assert!(!format!("{error:?}").contains("private"));
    }
    for (key, value) in [
        ("end", json!("2026-09-30T10:00:00-06:00")),
        ("start", json!("2026-10-01T10:00:00-03:00[America/Denver]")),
    ] {
        let mut row = event();
        row["event"][key] = value;
        assert_eq!(
            decode::read(&bytes(vec![row]), true, "calendar/"),
            Err(Failure::Invalid)
        );
    }
}
