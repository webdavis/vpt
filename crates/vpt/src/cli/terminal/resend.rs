use super::actions;
use crate::style::{Summary, Verdict};
use serde_json::Value;

pub(super) fn render(document: &Value, diagnostics: &[String]) -> String {
    let mut summary = Summary::default();
    let counts = &document["actions"];
    actions::rows(&mut summary, counts, "vpt actions resend");
    let withdrawn = counts["withdrawn"].as_u64().unwrap_or_default();
    if withdrawn > 0 {
        let detail = format!("{withdrawn} withdrawn");
        summary.row(Verdict::Ok, "actions", &detail);
    }
    actions::warnings(&mut summary, diagnostics);
    summary.finish("nothing to send")
}
