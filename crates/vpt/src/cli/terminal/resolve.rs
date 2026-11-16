use super::{review, text};
use crate::style::{self, Verdict};
use serde_json::Value;

pub(super) fn render(document: &Value, diagnostics: &[String]) -> String {
    let resolved = &document["resolved"];
    let already = if resolved["changed"] == false {
        "already "
    } else {
        ""
    };
    let detail = format!("{already}{}", text(resolved, "state"));
    let row = style::row(Verdict::Ok, text(resolved, "identifier"), &detail);
    row + &review::render(document, diagnostics)
}
