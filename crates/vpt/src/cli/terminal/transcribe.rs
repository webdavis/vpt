use super::{items, text};
use crate::style::{Summary, Verdict};
use serde_json::Value;

pub(super) fn render(document: &Value, diagnostics: &[String]) -> String {
    let mut summary = Summary::default();
    let id = text(document, "id");
    let language = text(document, "language");
    if document["dry_run"] == true {
        let engine = items(document, "engines")
            .next()
            .map(|engine| text(engine, "name"));
        let detail = format!(
            "would transcribe in {language} with {}",
            engine.unwrap_or_default()
        );
        summary.row(Verdict::NotRun, id, &detail);
        return summary.finish("");
    }
    let detail = format!("transcribed with {} ({language})", text(document, "engine"));
    summary.row(Verdict::Ok, id, &detail);
    if let Some(note) = document["note"].as_str() {
        summary.row(Verdict::Ok, "note", note);
    }
    for diagnostic in diagnostics {
        summary.row(Verdict::Warning, id, diagnostic);
    }
    summary.finish("")
}
