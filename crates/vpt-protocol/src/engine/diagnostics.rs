use crate::additive::collect;
use serde_json::Value;
use std::collections::BTreeSet;

pub(super) fn unknown_fields(value: &Value) -> Vec<String> {
    let mut fields = BTreeSet::new();
    collect(
        value,
        "",
        &["schema", "engine", "language", "local", "segments"],
        &mut fields,
    );
    collect(
        &value["engine"],
        "engine.",
        &["name", "tool", "version", "model", "family"],
        &mut fields,
    );
    if let Some(segments) = value["segments"].as_array() {
        for segment in segments {
            collect(
                segment,
                "segments.",
                &["start", "end", "text", "words"],
                &mut fields,
            );
            if let Some(words) = segment["words"].as_array() {
                for word in words {
                    collect(
                        word,
                        "segments.words.",
                        &["start", "end", "text", "confidence"],
                        &mut fields,
                    );
                }
            }
        }
    }
    fields
        .into_iter()
        .map(|field| format!("unknown additive engine field: {field}"))
        .collect()
}
