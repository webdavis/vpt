use super::{items, text};
use crate::commands::search::printable;
use crate::style::{Summary, Verdict, counted};
use serde_json::Value;

pub(super) fn render(document: &Value, _: &[String]) -> String {
    let mut summary = Summary::default();
    for hit in items(document, "hits") {
        let at = hit["moment"].as_str().unwrap_or(text(hit, "recording"));
        let title = hit["title"].as_str().unwrap_or("(untitled)");
        let detail = format!(
            "{} ({}, {})",
            printable(text(hit, "quote")),
            printable(title),
            text(hit, "stage")
        );
        summary.row(Verdict::Ok, at, &detail);
    }
    if document["truncated"] == true {
        let limit = document["limit"].as_u64().unwrap_or_default();
        let shown = format!("showed the first {}", counted(limit, "match", "matches"));
        summary
            .mark(Verdict::Warning, &shown)
            .advice("see more with --limit");
    }
    let skipped = document["skipped"].as_u64().unwrap_or_default();
    if skipped > 0 {
        let unread = format!("{} could not be read", counted(skipped, "note", "notes"));
        summary.mark(Verdict::Warning, &unread);
    }
    summary.finish("no matches")
}
