use crate::additive::collect;
use serde_json::Value;
use std::collections::BTreeSet;

pub(super) fn unknown_fields(value: &Value) -> Vec<String> {
    let mut fields = BTreeSet::new();
    collect(
        value,
        "",
        &["schema", "context", "occasions", "tasks"],
        &mut fields,
    );
    collect(
        &value["context"],
        "context.",
        &["calendar", "tasks"],
        &mut fields,
    );
    for half in ["calendar", "tasks"] {
        collect(
            &value["context"][half],
            &format!("context.{half}."),
            &["status", "reason", "detail"],
            &mut fields,
        );
    }
    if let Some(rows) = value["occasions"].as_array() {
        for row in rows {
            collect(
                row,
                "occasions[].",
                &[
                    "source",
                    "id",
                    "calendar",
                    "title",
                    "at",
                    "duration_secs",
                    "participants",
                    "tags",
                ],
                &mut fields,
            );
            if let Some(people) = row["participants"].as_array() {
                for person in people {
                    collect(
                        person,
                        "occasions[].participants[].",
                        &["name", "email"],
                        &mut fields,
                    );
                }
            }
        }
    }
    if let Some(rows) = value["tasks"].as_array() {
        for row in rows {
            collect(
                row,
                "tasks[].",
                &[
                    "source",
                    "project",
                    "id",
                    "content",
                    "due",
                    "completed",
                    "priority",
                ],
                &mut fields,
            );
        }
    }
    fields.into_iter().collect()
}
