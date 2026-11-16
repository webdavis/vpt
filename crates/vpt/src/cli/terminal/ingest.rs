use super::{actions, items, text};
use crate::style::{Summary, Verdict};
use serde_json::Value;

pub(super) fn render(document: &Value, diagnostics: &[String]) -> String {
    let mut summary = Summary::default();
    for record in items(document, "ingested") {
        summary.row(Verdict::Ok, text(record, "id"), "ingested");
    }
    for path in items(document, "would_ingest") {
        summary.row(Verdict::NotRun, text(path, "path"), "would ingest");
    }
    for deferred in items(document, "deferred") {
        let detail = format!("deferred, {}", text(deferred, "reason"));
        summary.row(Verdict::NotRun, text(deferred, "path"), &detail);
    }
    actions::warnings(&mut summary, diagnostics);
    summary.finish("nothing new to ingest")
}
