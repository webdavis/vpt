use super::text;
use crate::style::{Summary, Verdict, counted};
use serde_json::Value;

pub(super) fn render(document: &Value, _: &[String]) -> String {
    let mut summary = Summary::default();
    let title = document["title"].as_str().unwrap_or("(untitled)");
    summary.row(
        Verdict::Ok,
        text(document, "id"),
        &title.escape_debug().to_string(),
    );
    let duration = document["duration_secs"].as_u64().unwrap_or_default();
    let captured = format!(
        "{}, {}",
        text(document, "captured_at"),
        counted(duration, "second", "seconds")
    );
    summary.row(Verdict::Ok, "captured", &captured);
    for stage in ["transcribe", "note", "analysis"] {
        let state = text(&document["stages"], stage);
        let verdict = match state {
            "failed" => Verdict::Issue,
            "succeeded" | "disabled" => Verdict::Ok,
            _ => Verdict::NotRun,
        };
        summary.row(verdict, stage, state);
    }
    let paths = [("audio", &document["audio_path"])]
        .into_iter()
        .chain(["transcript", "analysis"].map(|stage| (stage, &document["note_paths"][stage])));
    for (name, path) in paths {
        if let Some(path) = path.as_str() {
            summary.row(Verdict::Ok, name, &path.escape_debug().to_string());
        }
    }
    summary.finish("no recording")
}
