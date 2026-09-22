use serde_json::Value;

pub fn document(command: &str, body: Value) -> Value {
    let mut ordered = serde_json::Map::new();
    ordered.insert("schema".into(), "vpt.result/1".into());
    ordered.insert("command".into(), command.into());
    if let Value::Object(fields) = body {
        ordered.extend(fields);
    }
    Value::Object(ordered)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_result_document_leads_with_schema_and_command() {
        let json = document("storage", serde_json::json!({"stores": {"audio": 2}}));
        let text = json.to_string();
        assert!(
            text.starts_with(r#"{"schema":"vpt.result/1","command":"storage","#),
            "{text}"
        );
        assert_eq!(json["stores"]["audio"], 2);
    }
}
