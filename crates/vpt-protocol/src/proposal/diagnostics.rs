use crate::additive::collect;
use serde_json::Value;
use std::collections::BTreeSet;

pub(super) fn unknown_fields(value: &Value) -> Vec<String> {
    let mut fields = BTreeSet::new();
    collect(
        value,
        "",
        &["schema", "by", "summary", "actions", "tags", "relations"],
        &mut fields,
    );
    for (key, known) in [
        ("summary", &["text", "sources"][..]),
        ("actions", &["text", "sources", "owner", "due"][..]),
        ("tags", &["tag", "reason"][..]),
        ("relations", &["kind", "target", "reason"][..]),
    ] {
        if let Some(items) = value[key].as_array() {
            for item in items {
                collect(item, &format!("{key}."), known, &mut fields);
            }
        }
    }
    if let Some(actions) = value["actions"].as_array() {
        for action in actions {
            collect(
                &action["due"],
                "actions.due.",
                &["date", "time", "sources"],
                &mut fields,
            );
        }
    }
    fields
        .into_iter()
        .map(|field| format!("unknown additive proposal field: {field}"))
        .collect()
}
