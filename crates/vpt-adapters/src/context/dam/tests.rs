use super::*;
use serde_json::{Value, json};
use vpt_domain::brief::ContextStatus;

pub(super) fn event() -> Value {
    json!({"oid":"event-1","subject":"Private planning","path":"calendar/work/","labels":[],
        "status":"confirmed","transparency":"busy","all_day":false,
        "start":"2026-10-01T10:00:00-06:00","end":"2026-10-01T10:30:00-06:00","busy":true})
}
pub(super) fn shown(oid: &str) -> Value {
    json!({"oid":oid,"kind":"event","subject":"Private planning","body":"","path":"calendar/work/",
        "labels":[],"depends":[],"reminders":[],"author":"owner","details":null,
        "event":{"start":"2026-10-01T10:00:00-06:00","end":"2026-10-01T10:30:00-06:00",
            "attendees":[{"email":"guest@example.org","response":"accepted"}],
            "organizer":{"email":"host@example.org","name":"Private host"},
            "status":"confirmed","transparency":"busy","visibility":"default",
            "event_type":"default","attachments":[],"location":"Room","timezone":"America/Denver",
            "conference":{"provider":"meet","url":"https://example.org"},"color":"1"},
        "id":"caller-id","future_key":true})
}
pub(super) fn task() -> Value {
    json!({"oid":"task-1","kind":"task","subject":"Private task\nexact text","body":"",
        "path":"work/team/","labels":[],"depends":[],"reminders":[],"author":"owner",
        "details":null,"task":{"done":false,"priority":3,"due":"2026-10-02"}})
}
pub(super) fn bytes(rows: Vec<Value>) -> Vec<u8> {
    serde_json::to_vec(&json!({"objects":rows})).unwrap()
}
pub(super) fn agenda(rows: Vec<Value>) -> Vec<u8> {
    serde_json::to_vec(&json!({"events":rows})).unwrap()
}
fn scopes() -> Vec<String> {
    vec!["calendar/".into()]
}

#[test]
fn agenda_rows_normalize_zoned_times_and_keep_identity_and_title() {
    let rows = decode::agenda(&agenda(vec![event()]), &scopes()).unwrap();
    let row = &rows[0].occasion;
    assert!(!rows[0].attended);
    assert_eq!(row.id, "event-1");
    assert_eq!(row.details.title, "Private planning");
    assert_eq!(row.details.at.as_str(), "2026-10-01T10:00:00-06:00");
    assert_eq!(row.details.duration_secs, Some(1800));
    assert!(row.details.participants.is_empty());
    assert!(!format!("{rows:?}").contains("Private"));
}

#[test]
fn shown_event_supplies_participants_by_email_with_the_organizer_name() {
    let participants =
        decode::attendees(&serde_json::to_vec(&shown("event-1")).unwrap(), "event-1").unwrap();
    assert_eq!(participants[0].email.as_deref(), Some("guest@example.org"));
    assert_eq!(participants[1].name, "Private host");
    let mut alone = shown("event-1");
    alone["event"]["attendees"] = json!([]);
    alone["event"].as_object_mut().unwrap().remove("organizer");
    assert!(
        decode::attendees(&serde_json::to_vec(&alone).unwrap(), "event-1")
            .unwrap()
            .is_empty()
    );
    for (key, value) in [("oid", json!("other")), ("kind", json!("task"))] {
        let mut object = shown("event-1");
        object[key] = value;
        assert_eq!(
            decode::attendees(&serde_json::to_vec(&object).unwrap(), "event-1"),
            Err(Failure::Invalid)
        );
    }
}

#[test]
fn agenda_rows_that_carry_attendees_need_no_show() {
    let mut row = event();
    row["attendees"] = json!([{"email":"guest@example.org","response":"accepted"}]);
    row["organizer"] = json!({"email":"guest@example.org","name":"Private guest"});
    row["future_key"] = json!({"nested": [1, 2]});
    let rows = decode::agenda(&agenda(vec![row]), &scopes()).unwrap();
    assert!(rows[0].attended);
    assert_eq!(rows[0].occasion.details.participants.len(), 1);
    assert_eq!(
        rows[0].occasion.details.participants[0].name,
        "Private guest"
    );
}

#[test]
fn agenda_labels_become_occasion_tags_and_cancelled_rows_are_dropped() {
    let mut row = event();
    row["labels"] = json!(["Private project", "team/planning"]);
    let mut cancelled = event();
    cancelled["oid"] = json!("cancelled");
    cancelled["status"] = json!("cancelled");
    let rows = decode::agenda(&agenda(vec![row, cancelled]), &scopes()).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].occasion.details.tags,
        ["Private project", "team/planning"]
    );
}

#[test]
fn dam_tasks_preserve_project_due_priority_and_exclude_completed_rows() {
    let mut done = task();
    done["future_key"] = json!("added later");
    done["oid"] = json!("done");
    done["task"]["done"] = json!(true);
    let snapshot = decode::read(&bytes(vec![task(), done]), "work/").unwrap();
    assert_eq!(snapshot.status.calendar, ContextStatus::Disabled);
    assert_eq!(snapshot.status.tasks, ContextStatus::Present);
    assert_eq!(snapshot.tasks.len(), 1);
    assert_eq!(snapshot.tasks[0].project, "work/team/");
    assert_eq!(snapshot.tasks[0].content, "Private task\nexact text");
    assert_eq!(snapshot.tasks[0].due.as_deref(), Some("2026-10-02"));
    assert_eq!(snapshot.tasks[0].priority, Some(3));
}

#[test]
fn all_day_agenda_rows_use_the_declared_zone_across_dst() {
    let mut row = event();
    row["all_day"] = json!(true);
    row["start"] = json!("2026-11-01");
    row["end"] = json!("2026-11-02");
    row["timezone"] = json!("America/Denver");
    let rows = decode::agenda(&agenda(vec![row]), &scopes()).unwrap();
    assert_eq!(
        rows[0].occasion.details.at.as_str(),
        "2026-11-01T00:00:00-06:00"
    );
    assert_eq!(rows[0].occasion.details.duration_secs, Some(25 * 3600));
}

#[test]
fn malformed_unselected_oversized_and_inconsistent_provider_data_fail_privately() {
    let mut outside = event();
    outside["path"] = json!("private/");
    for payload in [
        bytes(vec![task()]),
        agenda(vec![outside]),
        b"{\"events\":{},\"private\":true}".to_vec(),
        vec![b' '; 65_537],
    ] {
        let error = decode::agenda(&payload, &scopes()).unwrap_err();
        assert!(!format!("{error:?}").contains("private"));
    }
    for (key, value) in [
        ("end", json!("2026-09-30T10:00:00-06:00")),
        ("start", json!("2026-10-01T10:00:00-03:00[America/Denver]")),
        ("status", json!("private")),
    ] {
        let mut row = event();
        row[key] = value;
        assert!(decode::agenda(&agenda(vec![row]), &scopes()).is_err());
    }
}
