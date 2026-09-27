use crate::cli::output::Outcome;
use crate::compose::Environment;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use vpt_adapters::HelperClient;
use vpt_adapters::config::{ConfigError, from_table, load_file};

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

pub fn run(environment: &Environment, config: Option<&Path>) -> Outcome {
    let selected = config
        .map(Path::to_path_buf)
        .or_else(|| environment.config_path().ok());
    let helper_path = selected.and_then(|path| match load_file(&path) {
        Ok(table) => environment
            .home_dir()
            .ok()
            .and_then(|home| from_table(&table, &home).ok())
            .map(|settings| settings.helper_path),
        Err(ConfigError::Missing(_)) => Some(PathBuf::from("vpt-macos")),
        Err(_) => None,
    });
    let helper = helper_path.map(HelperClient::new);
    let version = helper.as_ref().and_then(|helper| helper.version().ok());
    let diagnostics = helper
        .as_ref()
        .map(HelperClient::take_diagnostics)
        .unwrap_or_default();
    let mut result = document(version.as_deref());
    let mut text = human(version.as_deref());
    if !diagnostics.is_empty() {
        result["diagnostics"] = json!(diagnostics);
        for diagnostic in &diagnostics {
            text.push_str(diagnostic);
            text.push('\n');
        }
    }
    Outcome::Success {
        document: result,
        human: text,
    }
}
