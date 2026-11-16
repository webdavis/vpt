use super::text;
use crate::style::{Summary, Verdict};
use serde_json::Value;

pub(super) fn render(document: &Value, _: &[String]) -> String {
    let mut summary = Summary::default();
    let confirmed = &document["confirmed"];
    let already = if confirmed["changed"] == false {
        "already "
    } else {
        ""
    };
    if confirmed["term"] == true {
        let place = if confirmed["added"] == true {
            "added to"
        } else {
            "already in"
        };
        let path = text(confirmed, "path").escape_debug();
        summary.row(Verdict::Ok, "term", &format!("{place} {path}"));
    } else if confirmed["relation"] == true {
        let detail = format!("{} relation {already}confirmed", text(confirmed, "kind"));
        summary.row(Verdict::Ok, text(&document["record"], "id"), &detail);
    } else {
        let action = if confirmed["rejected"] == true {
            "rejected"
        } else {
            "confirmed"
        };
        let detail = format!("tag {already}{action}");
        summary.row(Verdict::Ok, text(&document["record"], "id"), &detail);
        if confirmed["added"] == true {
            summary.row(Verdict::Ok, "known tags", "added");
        }
    }
    summary.finish("nothing confirmed")
}
