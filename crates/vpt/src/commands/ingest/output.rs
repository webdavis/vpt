use crate::cli::output::Outcome;
use crate::compose::events::warning_text;
use crate::documents::record_json;
use serde_json::json;
use vpt_application::{IngestError, IngestReport};

pub(super) fn outcome(result: Result<IngestReport, Box<IngestError>>, dry_run: bool) -> Outcome {
    let report = match result {
        Ok(report) => report,
        Err(error) => {
            let mut diagnostics = error.log;
            diagnostics.extend(
                error
                    .warnings
                    .into_iter()
                    .map(warning_text)
                    .map(str::to_owned),
            );
            let human = format!(
                "Ingestion stopped; {} recording(s) committed\n",
                error.completed.len()
            );
            return Outcome::FailedReport {
                error: super::errors::document(error.failure)
                    .completed(error.completed.iter().map(ToString::to_string).collect())
                    .diagnostics(diagnostics),
                human,
            };
        }
    };
    let mut seen = std::collections::HashSet::new();
    let completed = if dry_run {
        Vec::new()
    } else {
        report
            .ingested
            .iter()
            .map(|row| &row.id)
            .chain(&report.recovered)
            .filter(|id| seen.insert(id.as_str()))
            .map(ToString::to_string)
            .collect()
    };
    let mut diagnostics = report.log;
    diagnostics.extend(
        report
            .warnings
            .into_iter()
            .map(warning_text)
            .map(str::to_owned),
    );
    let records = match report
        .ingested
        .iter()
        .map(record_json)
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(records) => records,
        Err(error) => {
            return Outcome::Failure(error.completed(completed).diagnostics(diagnostics));
        }
    };
    let mut human = format!(
        "{} {} recording(s); deferred {}; skipped {}\n",
        if dry_run { "Would ingest" } else { "Ingested" },
        if dry_run {
            report.would_ingest.len()
        } else {
            records.len()
        },
        report.deferred.len(),
        report.skipped
    );
    for row in &report.ingested {
        human.push_str(&format!("{}\n", row.id));
    }
    for row in &report.would_ingest {
        human.push_str(&format!("{}\n", row.path.display()));
    }
    for row in &report.deferred {
        human.push_str(&format!(
            "Deferred {}: {}\n",
            row.path.display(),
            row.reason.as_str()
        ));
    }
    for warning in &diagnostics {
        human.push_str(&format!("Warning: {warning}\n"));
    }
    Outcome::Success {
        completed,
        diagnostics: diagnostics.clone(),
        human,
        document: vpt_protocol::result::document(
            "ingest",
            json!({
                "dry_run": dry_run, "ingested": records,
                "already_ingested": report.already_ingested.iter().map(ToString::to_string).collect::<Vec<_>>(),
                "recovered": report.recovered.iter().map(ToString::to_string).collect::<Vec<_>>(),
                "deferred": report.deferred.iter().map(|row| json!({"path": row.path, "reason": row.reason.as_str()})).collect::<Vec<_>>(),
                "skipped": report.skipped,
                "would_ingest": report.would_ingest.iter().map(|row| json!({"path": row.path, "title": row.title, "title_source": row.title_source.as_str()})).collect::<Vec<_>>(),
                "diagnostics": diagnostics,
            }),
        ),
    }
}
