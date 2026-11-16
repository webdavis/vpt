use super::{items, text};
use crate::style::{Summary, Verdict};
use serde_json::Value;

pub(super) fn render(document: &Value, _: &[String]) -> String {
    let mut summary = Summary::default();
    for occasion in items(document, "occasions") {
        let (verdict, state) = if occasion["briefed_at"].is_null() {
            (Verdict::NotRun, "unbriefed")
        } else {
            (Verdict::Ok, "briefed")
        };
        let title = text(occasion, "title").escape_debug();
        let detail = format!("{}, {state}, {title}", text(occasion, "at"));
        summary.row(verdict, text(occasion, "id"), &detail);
    }
    summary.finish("no occasions")
}
