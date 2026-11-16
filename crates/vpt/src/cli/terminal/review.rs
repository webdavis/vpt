use super::{items, text};
use crate::style::{Summary, Verdict, counted};
use serde_json::Value;

pub(super) fn render(document: &Value, _: &[String]) -> String {
    let mut summary = Summary::default();
    if document["transcribed"] == false {
        summary.mark(Verdict::NotRun, "no accepted transcript");
    }
    for flag in items(document, "flags") {
        let name = format!("{} {}", text(flag, "timecode"), text(flag, "class"));
        let mut quoted = flag["record_text"].to_string();
        if !flag["alternative_text"].is_null() {
            quoted.push_str(&format!(" | {}", flag["alternative_text"]));
        }
        let count = flag["occurrence_count"].as_u64().unwrap_or_default();
        let occurrences = counted(count, "occurrence", "occurrences");
        let detail = format!("{quoted}, {occurrences}, {}", text(flag, "identifier"));
        summary.row(Verdict::Warning, &name, &detail);
    }
    for finding in items(document, "findings") {
        let name = format!(
            "{} claim {}",
            text(finding, "artifact"),
            finding["claim_index"]
        );
        let detail = format!(
            "{} {}, {}",
            text(finding, "class"),
            finding["record_text"],
            text(finding, "identifier")
        );
        summary.row(Verdict::Warning, &name, &detail);
    }
    for diagnostic in items(document, "diagnostics") {
        let detail = format!(
            "{} {}",
            text(diagnostic, "class"),
            text(diagnostic, "identifier")
        );
        summary.row(Verdict::NotRun, "diagnostic", &detail);
    }
    summary.finish("no open flags")
}
