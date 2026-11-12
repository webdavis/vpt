use super::*;
use serde_json::json;

#[test]
fn interruption_status_preserves_the_public_error_envelope_and_exit_code() {
    let mut document = ErrorDocument::new(ErrorKind::Command, "command was interrupted");
    let wire = document.to_json();
    document.interrupted = true;
    assert_eq!(document.to_json(), wire);
    assert_eq!(document.exit_code(), 1);
}

#[test]
fn a_refusal_carries_its_rule_and_exits_3() {
    let document =
        ErrorDocument::new(ErrorKind::Refused, "engines share a family").rule("same_family");
    assert_eq!(
        document.to_json(),
        json!({
            "schema": "vpt.error/1",
            "error": {"kind": "refused", "rule": "same_family", "message": "engines share a family", "ids": [], "completed": []}
        })
    );
    assert_eq!(document.exit_code(), 3);
}

#[test]
fn every_kind_maps_to_the_spec_exit_code() {
    for (kind, name, code) in [
        (ErrorKind::Refused, "refused", 3),
        (ErrorKind::Usage, "usage", 2),
        (ErrorKind::Config, "config", 2),
        (ErrorKind::Engine, "engine", 1),
        (ErrorKind::Helper, "helper", 1),
        (ErrorKind::Command, "command", 1),
        (ErrorKind::Store, "store", 1),
        (ErrorKind::Ledger, "ledger", 1),
    ] {
        assert_eq!(kind.exit_code(), code, "{name}");
        assert_eq!(
            ErrorDocument::new(kind, "failed").to_json()["error"]["kind"],
            name
        );
    }
}

#[test]
fn a_non_refusal_has_a_null_rule() {
    let json = ErrorDocument::new(ErrorKind::Ledger, "database locked").to_json();
    assert_eq!(json["error"].get("rule"), Some(&Value::Null));
}

#[test]
fn checks_appear_only_when_given() {
    let plain = ErrorDocument::new(ErrorKind::Refused, "checks failed").rule("doctor_checks");
    assert!(plain.to_json()["error"].get("checks").is_none());
    let json = plain
        .checks(vec![
            Check {
                name: "helper".into(),
                ok: false,
                notice: false,
                detail: "absent".into(),
            },
            Check {
                name: "config".into(),
                ok: true,
                notice: false,
                detail: "valid".into(),
            },
            Check {
                name: "recordings_dir".into(),
                ok: true,
                notice: true,
                detail: "absent".into(),
            },
        ])
        .to_json();
    assert_eq!(
        json["error"]["checks"],
        json!([
            {"name": "helper", "ok": false, "detail": "absent"},
            {"name": "config", "ok": true, "detail": "valid"},
            {"name": "recordings_dir", "ok": true, "notice": true, "detail": "absent"}
        ])
    );
}

#[test]
fn diagnostics_are_retained_inside_the_one_error_document_when_present() {
    let plain = ErrorDocument::new(ErrorKind::Store, "read failed");
    assert!(plain.to_json()["error"].get("diagnostics").is_none());
    let json = plain
        .diagnostics(vec!["helper absent".into(), "notification disabled".into()])
        .to_json();
    assert_eq!(
        json["error"]["diagnostics"],
        json!(["helper absent", "notification disabled"])
    );
}

#[test]
fn ids_and_completed_preserve_order_and_escaped_text() {
    let json = ErrorDocument::new(ErrorKind::Store, "failed \"read\"\n雪")
        .ids(vec!["b".into(), "a".into()])
        .completed(vec!["c".into(), "b".into()])
        .to_json();
    let decoded: Value = serde_json::from_str(&json.to_string()).expect("document");
    assert_eq!(decoded["error"]["message"], "failed \"read\"\n雪");
    assert_eq!(decoded["error"]["ids"], json!(["b", "a"]));
    assert_eq!(decoded["error"]["completed"], json!(["c", "b"]));
}
