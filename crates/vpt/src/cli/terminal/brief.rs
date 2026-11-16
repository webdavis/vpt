use super::items;
use crate::style::{Summary, Verdict};
use serde_json::Value;

pub(super) fn upcoming(document: &Value, _: &[String]) -> String {
    let mut summary = Summary::default();
    for occasion in items(document, "written") {
        let occasion = occasion.as_str().unwrap_or_default().escape_debug();
        summary.row(Verdict::Ok, &occasion.to_string(), "brief written");
    }
    summary.finish("no brief written")
}
