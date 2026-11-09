mod model;
mod reconcile;
pub mod save;
use crate::engines::{CheckerRuns, EngineSettings};
use crate::ports::{Engine, EngineFailure, EngineOutput, EngineRequest};
pub use model::{
    AttemptFailure, EngineSlot, Transcription, TranscriptionError, TranscriptionInput,
};

pub fn transcribe(
    settings: &EngineSettings,
    engine: &impl Engine,
    input: &TranscriptionInput<'_>,
) -> Result<Transcription, TranscriptionError> {
    let selected = settings
        .select(input.language)
        .map_err(TranscriptionError::Selection)?;
    if !input.duration.is_finite() || input.duration < 0.0 {
        return Err(TranscriptionError::InvalidAudioDuration);
    }
    let request = |config| EngineRequest {
        engine: config,
        audio: input.audio,
        out_dir: input.out_dir,
        language: selected.language,
        duration: input.duration,
        timeout: settings.timeout,
    };
    let mut failures = Vec::new();
    let main = attempt(
        engine,
        &request(selected.main),
        EngineSlot::Main,
        &mut failures,
    )?;
    let checker = if settings.checker_runs == CheckerRuns::Always || main.is_none() {
        selected
            .checker
            .map(|config| attempt(engine, &request(config), EngineSlot::Checker, &mut failures))
            .transpose()?
            .flatten()
    } else {
        None
    };
    let (record_slot, record, checker, config) = match (main, checker) {
        (Some(record), checker) => (EngineSlot::Main, record, checker, selected.main),
        (None, Some(record)) => (
            EngineSlot::Checker,
            record,
            None,
            selected
                .checker
                .ok_or_else(|| TranscriptionError::Failed(failures.clone()))?,
        ),
        (None, None) => return Err(TranscriptionError::Failed(failures)),
    };
    Ok(Transcription {
        language: selected.language.into(),
        record_slot,
        record,
        checker,
        lexical_classification: config.lexical_classification(),
        checker_lexical_classification: selected
            .checker
            .is_some_and(|config| config.lexical_classification()),
        failures,
    })
}

fn attempt(
    engine: &impl Engine,
    request: &EngineRequest<'_>,
    slot: EngineSlot,
    failures: &mut Vec<AttemptFailure>,
) -> Result<Option<EngineOutput>, TranscriptionError> {
    match engine.transcribe(request) {
        Ok(output) => Ok(Some(output)),
        Err(cause) => {
            let interrupted = cause == EngineFailure::Interrupted;
            failures.push(AttemptFailure {
                engine: request.engine.name.clone(),
                slot,
                cause,
            });
            if interrupted {
                Err(TranscriptionError::Interrupted(std::mem::take(failures)))
            } else {
                Ok(None)
            }
        }
    }
}

#[cfg(test)]
mod tests;
