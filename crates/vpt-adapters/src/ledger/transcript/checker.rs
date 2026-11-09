use super::corrupt;
use vpt_application::ports::{AcceptedTranscript, LedgerError};
use vpt_protocol::engine::EngineMetadata;

pub(super) fn check_accepted(accepted: &AcceptedTranscript) -> Result<(), LedgerError> {
    if let Some(checker) = &accepted.checker_engine {
        fields(
            [
                &checker.name,
                &checker.tool,
                &checker.version,
                &checker.model,
                &checker.family,
            ],
            &accepted.transcript.engine.name,
            accepted.from_checker,
        )?;
    }
    Ok(())
}

pub(super) fn validate(
    checker: Option<&EngineMetadata>,
    record: &str,
    from_checker: bool,
) -> Result<(), LedgerError> {
    if let Some(checker) = checker {
        fields(
            [
                &checker.name,
                &checker.tool,
                &checker.version,
                &checker.model,
                &checker.family,
            ],
            record,
            from_checker,
        )?;
    }
    Ok(())
}

fn fields(values: [&str; 5], record: &str, from_checker: bool) -> Result<(), LedgerError> {
    if from_checker || values[0] == record {
        return Err(corrupt());
    }
    for value in values {
        if value.trim().is_empty() || value.len() > 262_144 || value.chars().count() > 65_536 {
            return Err(corrupt());
        }
    }
    Ok(())
}
