use super::Report;
use crate::cli::output::Outcome;
use vpt_protocol::error::ErrorDocument;

pub(super) fn success(report: &Report, dry_run: bool) -> Result<Outcome, Box<ErrorDocument>> {
    let diagnostics = report.diagnostics();
    let ingested = report
        .ingestion
        .ingested
        .iter()
        .map(crate::documents::record_json)
        .collect::<Result<Vec<_>, _>>()
        .map_err(Box::new)?;
    let mut human = format!(
        "{}: {} ingested, {} transcribed, {} notes, {} synthesized, {} released\n",
        if dry_run { "Run plan" } else { "Run complete" },
        if dry_run {
            report.ingestion.would_ingest.len()
        } else {
            ingested.len()
        },
        report.transcribed.len(),
        report.notes.len(),
        report.synthesized.len(),
        report.redacted.len()
    );
    for written in &report.redacted {
        human.push_str(&written.human());
    }
    for (id, stage) in &report.plan {
        human.push_str(&format!("Would {} {id}\n", stage.as_str()));
    }
    for path in &report.ingestion.would_ingest {
        human.push_str(&format!("Would ingest {}\n", path.path.display()));
    }
    for diagnostic in &diagnostics {
        human.push_str(&format!("Warning: {diagnostic}\n"));
    }
    let outcome = Outcome::Success {
        completed: if dry_run {
            Vec::new()
        } else {
            report.completed()
        },
        diagnostics: diagnostics.clone(),
        human,
        document: vpt_protocol::result::document(
            "run",
            serde_json::json!({
                "dry_run":dry_run, "ingested":ingested,
                "transcribed":report.transcribed.iter().map(|id| id.as_str()).collect::<Vec<_>>(),
                "notes":report.notes.iter().map(|(id,path)| serde_json::json!({"id":id.as_str(),"path":path})).collect::<Vec<_>>(),
                "synthesized":report.synthesized.iter().map(|id| id.as_str()).collect::<Vec<_>>(), "redacted":report.redacted.iter().map(|written| written.fields()).collect::<Vec<_>>(),
                "would_ingest":report.ingestion.would_ingest.iter().map(|row| serde_json::json!({"path":row.path,"title":row.title,"title_source":row.title_source.as_str()})).collect::<Vec<_>>(),
                "plan":report.plan.iter().map(|(id,stage)| serde_json::json!({"id":id.as_str(),"stage":stage.as_str()})).collect::<Vec<_>>(),
                "deferred":report.ingestion.deferred.iter().map(|row|serde_json::json!({"path":row.path,"reason":row.reason.as_str()})).collect::<Vec<_>>(),
                "skipped":report.ingestion.skipped, "diagnostics":diagnostics,
            }),
        ),
    };
    Ok(super::retention::output::attach(
        outcome,
        &report.retention,
        dry_run,
    ))
}
