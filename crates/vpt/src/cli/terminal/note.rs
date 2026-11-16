use super::{items, text};
use crate::style::{Summary, Verdict};
use serde_json::Value;

pub(super) fn write(document: &Value, _: &[String]) -> String {
    let mut summary = Summary::default();
    let id = text(&document["record"], "id");
    for path in items(document, "paths") {
        let path = path.as_str().unwrap_or_default().escape_debug();
        summary.row(Verdict::Ok, id, &format!("wrote {path}"));
    }
    summary.finish("no note written")
}

pub(super) fn verify(document: &Value, _: &[String]) -> String {
    let mut summary = Summary::default();
    let checked = if document["owned"] == true {
        "checked"
    } else {
        "checked, report only"
    };
    let path = text(document, "path").escape_debug().to_string();
    summary.row(Verdict::Ok, &path, checked);
    let mut findings = items(document, "flags").peekable();
    if findings.peek().is_none() {
        summary.mark(Verdict::Ok, "no findings");
    }
    for finding in findings {
        let name = format!("line {}", finding["line"]);
        summary.row(Verdict::Warning, &name, text(finding, "class"));
    }
    summary.finish("nothing checked")
}
