use crate::style::{Summary, Verdict};
use serde_json::Value;

pub(super) fn rows(summary: &mut Summary, actions: &Value, resend: &str) {
    let count = |name: &str| actions[name].as_u64().unwrap_or_default();
    let (sent, failed, refused) = (count("sent"), count("failed"), count("refused"));
    if sent > 0 {
        summary.row(Verdict::Ok, "actions", &format!("{sent} sent"));
    }
    if refused > 0 {
        summary
            .row(Verdict::Warning, "actions", &format!("{refused} refused"))
            .advice(&format!("send them again with {resend}"));
    }
    if failed > 0 {
        summary.row(Verdict::Issue, "actions", &format!("{failed} failed"));
    }
}

pub(super) fn warnings(summary: &mut Summary, diagnostics: &[String]) {
    for diagnostic in diagnostics {
        summary.mark(Verdict::Warning, diagnostic);
    }
}
