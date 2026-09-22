use serde_json::{Value, json};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn document(helper_version: Option<&str>) -> Value {
    json!({
        "schema": "vpt.result/1",
        "command": "version",
        "version": VERSION,
        "helper_version": helper_version,
    })
}

pub fn human(helper_version: Option<&str>) -> String {
    match helper_version {
        Some(helper) => format!("vpt {VERSION} (helper {helper})\n"),
        None => format!("vpt {VERSION} (helper absent)\n"),
    }
}
