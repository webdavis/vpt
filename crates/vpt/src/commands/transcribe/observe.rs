use crate::cli::output::Outcome;
use vpt_adapters::{AudioError, Roots};
use vpt_application::{
    Settings,
    ports::{RecordingLedger, RecordingRecord},
};
use vpt_domain::identity::RecordingId;
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(crate) fn record(
    ledger: &impl RecordingLedger,
    id: &RecordingId,
) -> Result<RecordingRecord, Box<ErrorDocument>> {
    ledger
        .by_id(id)
        .map_err(|error| Box::new(crate::compose::errors::ledger_error(error)))?
        .ok_or_else(|| {
            Box::new(
                ErrorDocument::new(ErrorKind::Usage, "recording not found")
                    .ids(vec![id.to_string()]),
            )
        })
}

pub(crate) fn audio(
    record: &RecordingRecord,
    settings: &Settings,
    roots: &Roots,
) -> Result<f64, Box<ErrorDocument>> {
    if record.audio_trashed_at.is_some() {
        return Err(Box::new(
            ErrorDocument::new(ErrorKind::Refused, "archived audio has expired")
                .rule("audio_expired"),
        ));
    }
    vpt_adapters::inspect_archived_audio(
        &roots.stores.audio,
        &record.audio_path,
        record.digest,
        settings.source.max_audio_bytes,
    )
    .map_err(|error| {
        Box::new(match error {
            AudioError::Unavailable => {
                ErrorDocument::new(ErrorKind::Store, "archived audio is unavailable")
            }
            AudioError::UnsafePath => {
                ErrorDocument::new(ErrorKind::Refused, "archived audio path is unsafe")
                    .rule("path_escape")
            }
            AudioError::TooLarge => ErrorDocument::new(
                ErrorKind::Refused,
                "archived audio exceeds the size ceiling",
            )
            .rule("audio_too_large"),
            AudioError::Changed => ErrorDocument::new(
                ErrorKind::Refused,
                "archived audio differs from the saved recording",
            )
            .rule("audio_modified"),
            AudioError::InvalidContainer => ErrorDocument::new(
                ErrorKind::Refused,
                "archived audio is not a complete media container",
            )
            .rule("invalid_audio"),
        })
    })
}

pub(crate) fn plan(
    record: &RecordingRecord,
    settings: &Settings,
    duration: f64,
) -> Result<Outcome, Box<ErrorDocument>> {
    let selected = settings
        .engines
        .select(None)
        .map_err(super::errors::selection)?;
    let engines: Vec<_> = std::iter::once(selected.main).chain(selected.checker).map(|engine| serde_json::json!({"name":engine.name, "family":engine.family(), "local":engine.local()})).collect();
    Ok(Outcome::Success {
        completed: Vec::new(),
        diagnostics: Vec::new(),
        document: vpt_protocol::result::document(
            "transcribe",
            serde_json::json!({
                "id":record.id.as_str(), "dry_run":true, "language":selected.language, "duration":duration, "engines":engines,
                "checker_runs":match settings.engines.checker_runs { vpt_application::engines::CheckerRuns::Always => "always", vpt_application::engines::CheckerRuns::OnFailure => "on_failure" }, "timeout_secs":settings.engines.timeout.as_secs()
            }),
        ),
        human: format!(
            "Would transcribe {} in {} using {}\n",
            record.id, selected.language, selected.main.name
        ),
    })
}
