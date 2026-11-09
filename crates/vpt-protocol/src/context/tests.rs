use super::*;
use serde_json::{Value, json};

fn document() -> Value {
    json!({
        "schema":"vpt.context/1",
        "context":{"calendar":{"status":"present"},"tasks":{"status":"present"}},
        "occasions":[{"source":"dam","id":"private-event","title":"Private planning",
            "at":"2026-10-01T10:00:00-06:00","duration_secs":1800,
            "participants":[{"name":"Private person","email":"private@example.org"}]}],
        "tasks":[{"source":"dam","project":"private-project","id":"private-task",
            "content":"Private task\nexact text","due":"2026-10-02","completed":false,"priority":2}]
    })
}

#[test]
fn valid_context_accepts_calendar_and_tasks_with_independent_absence() {
    let mut value = document();
    let decoded = decode(value.to_string().as_bytes()).unwrap();
    assert_eq!(
        decoded.snapshot.occasions[0].details.title,
        "Private planning"
    );
    assert_eq!(
        decoded.snapshot.tasks[0].content,
        "Private task\nexact text"
    );
    assert!(!format!("{decoded:?}").contains("Private"));
    value["occasions"] = json!([]);
    value["context"]["calendar"] = json!({"status":"absent","reason":"failed","detail":"deadline"});
    assert!(decode(value.to_string().as_bytes()).is_ok());
}

#[test]
fn additive_fields_are_named_at_each_level_without_copying_values() {
    let mut value = document();
    for pointer in [
        "",
        "/context",
        "/context/calendar",
        "/occasions/0",
        "/occasions/0/participants/0",
        "/tasks/0",
    ] {
        value.pointer_mut(pointer).unwrap()["future"] = json!("private value");
    }
    let decoded = decode(value.to_string().as_bytes()).unwrap();
    assert_eq!(
        decoded.diagnostics,
        vec![
            "context.calendar.future",
            "context.future",
            "future",
            "occasions[].future",
            "occasions[].participants[].future",
            "tasks[].future"
        ]
    );
}

#[test]
fn google_identity_includes_calendar_and_duplicate_provider_records_are_refused() {
    let mut value = document();
    value["occasions"][0]["source"] = json!("google");
    assert!(decode(value.to_string().as_bytes()).is_err());
    value["occasions"][0]["calendar"] = json!("private-calendar");
    let row = value["occasions"][0].clone();
    value["occasions"].as_array_mut().unwrap().push(row);
    assert!(decode(value.to_string().as_bytes()).is_err());
    value["occasions"][1]["calendar"] = json!("other-calendar");
    let decoded = decode(value.to_string().as_bytes()).unwrap();
    assert_ne!(
        decoded.snapshot.occasions[0].key(),
        decoded.snapshot.occasions[1].key()
    );
    let row = value["tasks"][0].clone();
    value["tasks"].as_array_mut().unwrap().push(row);
    assert!(decode(value.to_string().as_bytes()).is_err());
}

#[test]
fn duplicate_fields_and_deep_additions_are_refused() {
    let bytes = document().to_string().replacen(
        "\"schema\":",
        "\"schema\":\"vpt.context/1\",\"schema\":",
        1,
    );
    assert!(decode(bytes.as_bytes()).is_err());
    let mut value = document();
    value["future"] = json!([[[[[[[[["private"]]]]]]]]]);
    assert!(matches!(
        decode(value.to_string().as_bytes()),
        Err(DecodeError::Limit(_))
    ));
}

#[test]
fn encoding_round_trips_exact_provider_data_and_refuses_oversized_aggregate() {
    let decoded = decode(document().to_string().as_bytes()).unwrap();
    let text = encode(&decoded.snapshot).unwrap();
    assert_eq!(decode(text.as_bytes()).unwrap().snapshot, decoded.snapshot);
    let mut snapshot = decoded.snapshot;
    snapshot.tasks[0].content = "x".repeat(65_536);
    assert!(matches!(encode(&snapshot), Err(DecodeError::Limit(_))));
}

#[test]
fn context_refuses_unknown_major_with_version_and_bounds_before_allocation() {
    let mut value = document();
    value["schema"] = json!("vpt.context/2");
    assert_eq!(
        decode(value.to_string().as_bytes()),
        Err(DecodeError::UnsupportedVersion(2))
    );
    value["schema"] = json!("vpt.context/01");
    assert!(decode(value.to_string().as_bytes()).is_err());
    let mut value = document();
    value["occasions"] = json!(vec![value["occasions"][0].clone(); 257]);
    assert!(matches!(
        decode(value.to_string().as_bytes()),
        Err(DecodeError::Limit(_))
    ));
    assert!(matches!(
        decode(&vec![b' '; 65_537]),
        Err(DecodeError::Limit(_))
    ));
}

#[test]
fn context_refuses_inconsistent_status_identity_and_time_without_private_errors() {
    for (pointer, invalid) in [
        ("/context/calendar/status", json!("absent")),
        ("/occasions/0/source", json!("manual")),
        ("/occasions/0/id", json!("private\nidentity")),
        ("/occasions/0/at", json!("private-invalid-time")),
        ("/occasions/0/title", json!("")),
        ("/occasions/0/duration_secs", json!(-1)),
        ("/tasks/0/completed", json!("false")),
        ("/tasks/0/project", json!("")),
    ] {
        let mut value = document();
        *value.pointer_mut(pointer).unwrap() = invalid;
        let error = decode(value.to_string().as_bytes()).unwrap_err();
        assert!(!format!("{error:?}").contains("private"));
    }
}

#[test]
fn provider_tags_survive_normalization_and_encoding_without_becoming_diagnostics() {
    let mut value = document();
    value["occasions"][0]["tags"] = json!(["Private project", "team/planning"]);
    let decoded = decode(value.to_string().as_bytes()).unwrap();
    assert_eq!(
        decoded.snapshot.occasions[0].details.tags,
        ["Private project", "team/planning"]
    );
    assert!(decoded.diagnostics.is_empty());
    let encoded = encode(&decoded.snapshot).unwrap();
    assert_eq!(
        decode(encoded.as_bytes()).unwrap().snapshot,
        decoded.snapshot
    );
    for tags in [
        json!([""]),
        json!(["private\u{0}label"]),
        json!(vec!["label"; 257]),
    ] {
        value["occasions"][0]["tags"] = tags;
        let error = decode(value.to_string().as_bytes()).unwrap_err();
        assert!(!format!("{error:?}").contains("private"));
    }
    let mut snapshot = decoded.snapshot;
    snapshot.occasions[0].details.tags = vec!["label".into(); 257];
    assert!(matches!(encode(&snapshot), Err(DecodeError::Limit(_))));
}
