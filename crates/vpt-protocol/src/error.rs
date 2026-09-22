use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Refused,
    Usage,
    Config,
    Engine,
    Helper,
    Command,
    Store,
    Ledger,
}

impl ErrorKind {
    pub fn exit_code(self) -> i32 {
        match self {
            Self::Refused => 3,
            Self::Usage | Self::Config => 2,
            Self::Engine | Self::Helper | Self::Command | Self::Store | Self::Ledger => 1,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Refused => "refused",
            Self::Usage => "usage",
            Self::Config => "config",
            Self::Engine => "engine",
            Self::Helper => "helper",
            Self::Command => "command",
            Self::Store => "store",
            Self::Ledger => "ledger",
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Check {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorDocument {
    pub kind: ErrorKind,
    pub rule: Option<String>,
    pub message: String,
    pub ids: Vec<String>,
    pub completed: Vec<String>,
    pub checks: Option<Vec<Check>>,
    pub diagnostics: Vec<String>,
}

impl ErrorDocument {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            rule: None,
            message: message.into(),
            ids: vec![],
            completed: vec![],
            checks: None,
            diagnostics: vec![],
        }
    }
    pub fn rule(mut self, rule: &str) -> Self {
        self.rule = Some(rule.into());
        self
    }
    pub fn ids(mut self, ids: Vec<String>) -> Self {
        self.ids = ids;
        self
    }
    pub fn completed(mut self, completed: Vec<String>) -> Self {
        self.completed = completed;
        self
    }
    pub fn checks(mut self, checks: Vec<Check>) -> Self {
        self.checks = Some(checks);
        self
    }
    pub fn diagnostics(mut self, diagnostics: Vec<String>) -> Self {
        self.diagnostics = diagnostics;
        self
    }
    pub fn exit_code(&self) -> i32 {
        self.kind.exit_code()
    }
    pub fn to_json(&self) -> Value {
        let mut error = serde_json::Map::new();
        error.insert("kind".into(), self.kind.name().into());
        error.insert(
            "rule".into(),
            self.rule.clone().map_or(Value::Null, Value::String),
        );
        error.insert("message".into(), self.message.clone().into());
        error.insert("ids".into(), self.ids.clone().into());
        error.insert("completed".into(), self.completed.clone().into());
        if let Some(checks) = &self.checks {
            error.insert(
                "checks".into(),
                Value::Array(
                    checks
                        .iter()
                        .map(|check| {
                            let mut fields = serde_json::Map::new();
                            fields.insert("name".into(), check.name.clone().into());
                            fields.insert("ok".into(), check.ok.into());
                            fields.insert("detail".into(), check.detail.clone().into());
                            Value::Object(fields)
                        })
                        .collect(),
                ),
            );
        }
        if !self.diagnostics.is_empty() {
            error.insert("diagnostics".into(), self.diagnostics.clone().into());
        }
        let mut document = serde_json::Map::new();
        document.insert("schema".into(), "vpt.error/1".into());
        document.insert("error".into(), Value::Object(error));
        Value::Object(document)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
                    detail: "absent".into(),
                },
                Check {
                    name: "config".into(),
                    ok: true,
                    detail: "valid".into(),
                },
            ])
            .to_json();
        assert_eq!(
            json["error"]["checks"],
            json!([
                {"name": "helper", "ok": false, "detail": "absent"},
                {"name": "config", "ok": true, "detail": "valid"}
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
}
