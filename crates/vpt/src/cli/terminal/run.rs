use super::{actions, items, text};
use crate::style::{Summary, Verdict, counted};
use serde_json::Value;

pub(super) fn render(document: &Value, diagnostics: &[String]) -> String {
    let mut summary = Summary::default();
    let recordings = |key| {
        counted(
            items(document, key).count() as u64,
            "recording",
            "recordings",
        )
    };
    for (step, key) in [("ingested", "ingested"), ("transcribed", "transcribed")] {
        if items(document, key).next().is_some() {
            summary.row(Verdict::Ok, step, &recordings(key));
        }
    }
    let notes = items(document, "notes").count() as u64;
    if notes > 0 {
        summary.row(Verdict::Ok, "notes", &format!("{notes} published"));
    }
    if items(document, "analyzed").next().is_some() {
        summary.row(Verdict::Ok, "analyzed", &recordings("analyzed"));
    }
    for released in items(document, "redacted") {
        summary.row(Verdict::Ok, "released", text(released, "written"));
    }
    for planned in items(document, "plan") {
        let detail = format!("would {}", text(planned, "stage"));
        summary.row(Verdict::NotRun, text(planned, "id"), &detail);
    }
    for path in items(document, "would_ingest") {
        summary.row(Verdict::NotRun, text(path, "path"), "would ingest");
    }
    retention(
        &mut summary,
        &document["retention"],
        document["dry_run"] == true,
    );
    actions::rows(&mut summary, &document["actions"], "vpt actions resend");
    actions::warnings(&mut summary, diagnostics);
    summary.finish("nothing to do")
}

fn retention(summary: &mut Summary, retention: &Value, dry_run: bool) {
    let moved = items(retention, "moved").count() as u64;
    let kept = items(retention, "kept").count() as u64;
    if moved + kept == 0 {
        return;
    }
    let files = counted(moved, "file", "files");
    let (verdict, moved) = match dry_run {
        true => (Verdict::NotRun, format!("would move {files}")),
        false => (Verdict::Ok, format!("moved {files}")),
    };
    summary.row(verdict, "retention", &format!("{moved}, kept {kept}"));
}
