use super::text;
use crate::style::{Summary, Verdict};
use serde_json::Value;

pub(super) fn render(document: &Value, _: &[String]) -> String {
    let mut summary = Summary::default();
    let config = text(document, "written").escape_debug().to_string();
    let engine = text(document, "main_engine").escape_debug();
    summary.row(
        Verdict::Ok,
        &config,
        &format!("written with main engine {engine}"),
    );
    summary.finish("no configuration written")
}
