use serde_json::Value;
use std::collections::BTreeSet;

pub(crate) fn collect(value: &Value, prefix: &str, known: &[&str], fields: &mut BTreeSet<String>) {
    if let Some(object) = value.as_object() {
        for key in object.keys().filter(|key| !known.contains(&key.as_str())) {
            let escaped: String = key.chars().flat_map(char::escape_default).collect();
            fields.insert(format!("{prefix}{escaped}"));
        }
    }
}
