use serde_json::{Value, json};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn document(helper_version: Option<&str>) -> Value {
    vpt_protocol::result::document(
        "version",
        json!({"version": VERSION, "helper_version": helper_version}),
    )
}

pub fn human(helper_version: Option<&str>) -> String {
    match helper_version {
        Some(helper) => format!("vpt {VERSION} (helper {helper})\n"),
        None => format!("vpt {VERSION} (helper absent)\n"),
    }
}
