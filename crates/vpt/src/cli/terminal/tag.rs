use super::{items, text};
use crate::style::{Summary, Verdict, counted};
use serde_json::Value;

pub(super) fn render(document: &Value, _: &[String]) -> String {
    let mut summary = Summary::default();
    let (verdict, verb) = if document["dry_run"] == true {
        (Verdict::NotRun, "would rename")
    } else {
        (Verdict::Ok, "renamed")
    };
    if document["changed"] == false {
        summary.mark(
            Verdict::NotRun,
            "no recording or known tag carries that tag",
        );
    }
    for recording in items(document, "recordings") {
        let notes = items(recording, "notes").map(|note| {
            let path = note.as_str().unwrap_or_default().escape_debug();
            format!(", {path}")
        });
        let detail = format!("{verb}{}", notes.collect::<String>());
        summary.row(verdict, text(recording, "id"), &detail);
    }
    if document["known_tags_changed"] == true {
        summary.row(verdict, "known tags", verb);
    }
    let left = |key: &str, one: &str, many: &str| {
        let count = document[key].as_u64().unwrap_or_default();
        (count > 0).then(|| format!("{} left alone", counted(count, one, many)))
    };
    if let Some(briefs) = left("untouched_briefs", "brief", "briefs") {
        let why = "because a brief's tags are its meeting's labels";
        summary.row(Verdict::NotRun, "briefs", &format!("{briefs}, {why}"));
    }
    if let Some(notes) = left("untouched_notes", "other note", "other notes") {
        summary.row(Verdict::NotRun, "other notes", &notes);
    }
    summary.finish("nothing to rename")
}
