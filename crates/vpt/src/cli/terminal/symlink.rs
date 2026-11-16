use super::text;
use crate::style::{Summary, Verdict};
use serde_json::Value;

pub(super) fn deploy(document: &Value, _: &[String]) -> String {
    link(document, "links to")
}

pub(super) fn verify(document: &Value, _: &[String]) -> String {
    link(document, "points at")
}

fn link(document: &Value, relation: &str) -> String {
    let mut summary = Summary::default();
    let name = text(document, "link").escape_debug().to_string();
    let target = text(document, "target").escape_debug();
    summary.row(Verdict::Ok, &name, &format!("{relation} {target}"));
    summary.finish("no link")
}
