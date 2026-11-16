use super::{actions, items, text};
use crate::style::{Summary, Verdict};
use serde_json::Value;

pub(super) fn render(document: &Value, diagnostics: &[String]) -> String {
    let mut summary = Summary::default();
    let (verdict, verb) = if document["dry_run"] == true {
        (Verdict::NotRun, "would move")
    } else {
        (Verdict::Ok, "moved")
    };
    for moved in items(document, "moved") {
        let path = text(moved, "path").escape_debug();
        summary.row(verdict, text(moved, "store"), &format!("{verb} {path}"));
    }
    for kept in items(document, "kept") {
        let path = text(kept, "path").escape_debug().to_string();
        summary.row(
            Verdict::NotRun,
            &path,
            &format!("kept, {}", text(kept, "reason")),
        );
    }
    let completed: Vec<_> = items(document, "completed")
        .filter_map(Value::as_str)
        .collect();
    if !completed.is_empty() {
        summary.row(Verdict::Ok, "completed", &completed.join(", "));
    }
    actions::warnings(&mut summary, diagnostics);
    summary.finish("nothing to move")
}
