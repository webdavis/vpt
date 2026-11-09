mod mutation;
mod observe;
mod output;
mod retention;
mod selection;
mod work;
use crate::{
    cli::output::Outcome,
    compose::{Environment, settings_from},
};
use std::path::Path;
use vpt_application::{IngestReport, pipeline::NextStage};
use vpt_domain::identity::RecordingId;
use vpt_protocol::error::ErrorDocument;

#[derive(Default)]
struct Report {
    ingestion: IngestReport,
    transcribed: Vec<RecordingId>,
    synthesized: Vec<RecordingId>,
    redacted: Vec<super::redact::output::Written>,
    notes: Vec<(RecordingId, std::path::PathBuf)>,
    plan: Vec<(RecordingId, NextStage)>,
    retention: vpt_application::RetentionReport,
    retention_failed: bool,
    stage_diagnostics: Vec<String>,
}
impl Report {
    fn record_diagnostics(&mut self, id: &RecordingId, outcome: Outcome) {
        if let Outcome::Success { diagnostics, .. } = outcome {
            self.stage_diagnostics.extend(
                diagnostics
                    .into_iter()
                    .map(|detail| format!("{id}: {detail}")),
            );
        }
    }

    fn diagnostics(&self) -> Vec<String> {
        self.ingestion
            .log
            .iter()
            .chain(&self.stage_diagnostics)
            .cloned()
            .collect()
    }

    fn completed(&self) -> Vec<String> {
        let mut ids = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for id in self
            .ingestion
            .ingested
            .iter()
            .map(|row| &row.id)
            .chain(&self.ingestion.recovered)
            .chain(&self.transcribed)
            .chain(&self.synthesized)
            .chain(
                self.redacted
                    .iter()
                    .map(|written| &written.release.recording),
            )
            .chain(self.notes.iter().map(|(id, _)| id))
            .map(|id| id.as_str())
            .chain(self.retention.completed.iter().map(|id| id.as_str()))
        {
            if seen.insert(id) {
                ids.push(id.to_string());
            }
        }
        ids
    }
}

pub(crate) fn run(environment: &Environment, config: Option<&Path>, dry_run: bool) -> Outcome {
    execute(environment, config, dry_run).unwrap_or_else(|error| Outcome::Failure(*error))
}
fn execute(
    environment: &Environment,
    config: Option<&Path>,
    dry_run: bool,
) -> Result<Outcome, Box<ErrorDocument>> {
    let settings = settings_from(environment, config)?;
    settings
        .engines
        .select(None)
        .map_err(super::transcribe::errors::selection)?;
    let config = config.map(Path::to_path_buf).map_or_else(
        || {
            environment
                .config_path()
                .map_err(crate::compose::config_error)
        },
        Ok,
    )?;
    let roots = vpt_adapters::config::resolve(&settings, config.parent().unwrap_or(Path::new(".")))
        .map_err(|error| Box::new(crate::compose::errors::root_error(&error)))?;
    if dry_run {
        observe::run(&settings, &roots)
    } else {
        mutation::run(&settings, &roots)
    }
}
