use super::text;
use crate::style::{Summary, Verdict, counted};
use serde_json::Value;

pub(super) fn render(document: &Value, _: &[String]) -> String {
    let mut summary = Summary::default();
    for (name, store) in document["stores"].as_object().into_iter().flatten() {
        let count = |key: &str| store[key].as_u64().unwrap_or_default();
        let detail = format!(
            "{}, {} bytes, {}",
            counted(count("files"), "file", "files"),
            count("bytes"),
            text(store, "path").escape_debug()
        );
        summary.row(Verdict::Ok, name, &detail);
    }
    summary.finish("no stores")
}
