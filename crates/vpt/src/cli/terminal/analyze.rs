use super::{actions, items, text};
use crate::style::{Summary, Verdict, counted};
use serde_json::Value;

pub(super) fn one(document: &Value, diagnostics: &[String]) -> String {
    let mut summary = Summary::default();
    if document["dry_run"] == true {
        let id = text(document, "id");
        summary.row(
            Verdict::NotRun,
            id,
            "would analyze and publish its analysis note",
        );
        return summary.finish("");
    }
    let id = text(&document["record"], "id");
    summary.row(Verdict::Ok, id, "analyzed");
    for path in items(document, "paths") {
        summary.row(Verdict::Ok, "note", path.as_str().unwrap_or_default());
    }
    let findings = document["verify"]
        .as_object()
        .into_iter()
        .flat_map(|verify| verify.values())
        .filter_map(Value::as_u64)
        .sum();
    if findings > 0 {
        let detail = format!("{} to verify", counted(findings, "finding", "findings"));
        summary
            .row(Verdict::Warning, id, &detail)
            .advice(&format!("vpt review {id}"));
    }
    let resend = format!("vpt analyze {id} --resend-refused");
    actions::rows(&mut summary, &document["actions"], &resend);
    actions::warnings(&mut summary, diagnostics);
    summary.finish("")
}

pub(super) fn batch(document: &Value, diagnostics: &[String]) -> String {
    let mut summary = Summary::default();
    for row in items(document, "recordings") {
        let id = text(row, "id");
        if row["status"] == "planned" {
            summary.row(Verdict::NotRun, id, "would analyze");
            continue;
        }
        let changed = match row["summary_changed"] == true {
            true => "changed",
            false => "unchanged",
        };
        let count = |key: &str| row[key].as_u64().unwrap_or_default();
        let added = counted(count("tags_added"), "tag", "tags");
        let detail = format!(
            "summary {changed}, {added} added, {} removed",
            count("tags_removed")
        );
        summary.row(Verdict::Ok, id, &detail);
    }
    actions::rows(
        &mut summary,
        &document["actions"],
        "vpt analyze <id> --resend-refused",
    );
    actions::warnings(&mut summary, diagnostics);
    summary.finish("no recordings to analyze")
}
