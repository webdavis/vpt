use crate::cli::output::Outcome;
use std::path::Path;
use vpt_adapters::verification::FindingLine;

pub(super) fn success(path: &Path, owned: bool, report: &[FindingLine]) -> Outcome {
    let flags: Vec<_> = report.iter().map(|row| serde_json::json!({
        "class": row.finding.as_str(), "line": row.line,
        "range": if row.sources.is_empty() { None } else { Some(row.sources.iter().map(|source| [source.start_secs, source.end_secs]).collect::<Vec<_>>()) }
    })).collect();
    let mut human = format!(
        "{} findings in {}{}\n",
        report.len(),
        path.display(),
        if owned { "" } else { " (report only)" }
    );
    for row in report {
        human.push_str(&format!("Line {}: {}\n", row.line, row.finding.as_str()));
    }
    Outcome::Success {
        completed: Vec::new(),
        diagnostics: Vec::new(),
        document: vpt_protocol::result::document(
            "verify-note",
            serde_json::json!({"path": path, "owned": owned, "flags": flags}),
        ),
        human,
    }
}
