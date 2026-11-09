use crate::cli::output::Outcome;
use crate::compose::events::warning_text;
use serde_json::json;
use vpt_application::{RetentionFailure, RetentionReport};
use vpt_domain::layout::StoreKey;

pub(crate) fn outcome(
    result: Result<RetentionReport, Box<RetentionFailure>>,
    dry_run: bool,
) -> Outcome {
    let (report, failure) = match result {
        Ok(report) => (report, None),
        Err(failure) => (failure.report, Some(failure.error)),
    };
    let mut diagnostics = report.diagnostics.clone();
    diagnostics.extend(
        report
            .warnings
            .iter()
            .map(|warning| warning_text(*warning).to_owned()),
    );
    let completed: Vec<_> = report.completed.iter().map(ToString::to_string).collect();
    let completed_human = completed_summary(&completed);
    if report
        .moved
        .iter()
        .any(|entry| entry.path.to_str().is_none())
        || report
            .kept
            .iter()
            .any(|entry| entry.path.to_str().is_none())
    {
        let error = failure.map(super::errors::document).unwrap_or_else(|| {
            vpt_protocol::error::ErrorDocument::new(
                vpt_protocol::error::ErrorKind::Store,
                "retention report path is not UTF-8",
            )
        });
        return Outcome::FailedReport {
            error: error.completed(completed).diagnostics(diagnostics),
            human: format!(
                "{} file(s) moved before report failure\n",
                if dry_run { 0 } else { report.moved.len() },
            ),
        };
    }
    let mut human = format!(
        "{} {} file(s); kept {}\n",
        if dry_run { "Would move" } else { "Moved" },
        report.moved.len(),
        report.kept.len()
    );
    for moved in &report.moved {
        human.push_str(&format!(
            "{}: {}\n",
            moved.store.key_name(),
            moved.path.display()
        ));
    }
    for kept in &report.kept {
        human.push_str(&format!(
            "Kept {}: {}\n",
            kept.path.display(),
            kept.reason.as_str()
        ));
    }
    if let Some(error) = failure {
        return Outcome::FailedReport {
            error: super::errors::document(error)
                .completed(completed)
                .diagnostics(diagnostics),
            human,
        };
    }
    human.push_str(&completed_human);
    for diagnostic in &diagnostics {
        human.push_str(&format!("Warning: {diagnostic}\n"));
    }
    let counts: serde_json::Map<_, _> = StoreKey::all()
        .into_iter()
        .map(|store| {
            (
                store.key_name().to_owned(),
                json!(
                    report
                        .moved
                        .iter()
                        .filter(|entry| entry.store == store)
                        .count()
                ),
            )
        })
        .collect();
    Outcome::Success {
        completed: if dry_run {
            Vec::new()
        } else {
            completed.clone()
        },
        diagnostics: diagnostics.clone(),
        human,
        document: vpt_protocol::result::document(
            "retention run",
            json!({
                "dry_run": dry_run,
                "moved": report.moved.iter().map(|entry| json!({"store": entry.store.key_name(), "path": entry.path, "id": entry.owner.as_str()})).collect::<Vec<_>>(),
                "kept": report.kept.iter().map(|entry| json!({"path": entry.path, "reason": entry.reason.as_str()})).collect::<Vec<_>>(),
                "counts": counts, "completed": completed, "diagnostics": diagnostics,
            }),
        ),
    }
}

fn completed_summary(completed: &[String]) -> String {
    if completed.is_empty() {
        String::new()
    } else {
        format!("Completed: {}\n", completed.join(", "))
    }
}

#[cfg(test)]
mod tests;
