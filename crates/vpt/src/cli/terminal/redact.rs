use super::{actions, items, text};
use crate::style::{Summary, Verdict, counted};
use serde_json::Value;

pub(super) fn render(document: &Value, diagnostics: &[String]) -> String {
    let mut summary = Summary::default();
    let written = text(document, "written").escape_debug();
    summary.row(
        Verdict::Ok,
        text(document, "id"),
        &format!("wrote {written}"),
    );
    let report = text(document, "report").escape_debug().to_string();
    summary.row(Verdict::Ok, "private report", &report);
    let masks = document["masks"].as_object().into_iter().flatten();
    let masks: Vec<_> = masks
        .map(|(class, count)| format!("{class} {count}"))
        .collect();
    if !masks.is_empty() {
        summary.row(Verdict::Ok, "masked", &masks.join(", "));
    }
    let flagged = |key: &str| document[key].as_u64().unwrap_or_default();
    let (omitted, marked) = (flagged("omitted"), flagged("marked"));
    if omitted + marked > 0 {
        let spans = format!("{omitted} omitted, {marked} marked");
        summary.row(Verdict::Ok, "flagged spans", &spans);
    }
    if document["collision"] == true {
        summary.mark(
            Verdict::Ok,
            "used a timestamped name because the usual one was taken",
        );
    }
    let candidates: Vec<_> = items(document, "candidates").collect();
    if !candidates.is_empty() {
        let count = counted(candidates.len() as u64, "candidate", "candidates");
        summary.row(
            Verdict::Warning,
            "unmasked",
            &format!("{count} to read before sharing"),
        );
    }
    for candidate in candidates {
        let token = text(candidate, "token").escape_debug();
        summary.advice(&format!("line {}: {token}", candidate["line"]));
    }
    actions::warnings(&mut summary, diagnostics);
    summary.finish("nothing written")
}
