use super::*;

pub(super) fn encode_hook_failure(failure: HookFailureClass) -> Value {
    match failure {
        HookFailureClass::Spawn => json!({"kind":"spawn"}),
        HookFailureClass::Io => json!({"kind":"io"}),
        HookFailureClass::Encoding => json!({"kind":"encoding"}),
        HookFailureClass::Exited(code) => json!({"kind":"exited","code":code}),
        HookFailureClass::Signaled(signal) => json!({"kind":"signaled","code":signal}),
        HookFailureClass::DeadlineExceeded => json!({"kind":"deadline_exceeded"}),
        HookFailureClass::Interrupted => json!({"kind":"interrupted"}),
    }
}

pub(super) fn decode_hook_failure(value: &Value) -> Result<HookFailureClass, LedgerError> {
    Ok(match string(value, "kind")? {
        "spawn" => HookFailureClass::Spawn,
        "io" => HookFailureClass::Io,
        "encoding" => HookFailureClass::Encoding,
        "exited" => {
            HookFailureClass::Exited(i32::try_from(signed(value, "code")?).map_err(|_| corrupt())?)
        }
        "signaled" => HookFailureClass::Signaled(
            i32::try_from(signed(value, "code")?).map_err(|_| corrupt())?,
        ),
        "deadline_exceeded" => HookFailureClass::DeadlineExceeded,
        "interrupted" => HookFailureClass::Interrupted,
        _ => return Err(corrupt()),
    })
}

pub(super) fn encode_window(window: FailureWindow) -> Value {
    json!({"after":window.after.map(|id| id.render()),"through":window.through.render()})
}

pub(super) fn decode_window(value: &Value) -> Result<FailureWindow, LedgerError> {
    let after = optional(value, "after")?
        .map(|value| failure_id(&json!({"id":value}), "id"))
        .transpose()?;
    Ok(FailureWindow {
        after,
        through: failure_id(value, "through")?,
    })
}

pub(in crate::ledger::sqlite::history) fn run_kind(kind: RunKind) -> &'static str {
    match kind {
        RunKind::Brief => "brief",
        RunKind::Redact => "redact",
        RunKind::Ingest => "ingest",
        RunKind::Recover => "recover",
        RunKind::Publish => "publish",
        RunKind::Retain => "retain",
        RunKind::Transcribe => "transcribe",
        RunKind::Note => "note",
        RunKind::Confirm => "confirm",
        RunKind::Review => "review",
        RunKind::VerifyNote => "verify-note",
        RunKind::Analyze => "analyze",
        RunKind::Pipeline => "run",
    }
}

pub(super) fn run_outcome(outcome: RunOutcome) -> &'static str {
    match outcome {
        RunOutcome::Completed => "completed",
        RunOutcome::Failed => "failed",
        RunOutcome::Interrupted => "interrupted",
    }
}

pub(super) fn work_outcome(outcome: WorkOutcome) -> &'static str {
    match outcome {
        WorkOutcome::Completed => "completed",
        WorkOutcome::Failed => "failed",
        WorkOutcome::Unknown => "unknown",
        WorkOutcome::Deferred => "deferred",
    }
}

pub(super) fn effect_kind(kind: EffectKind) -> &'static str {
    match kind {
        EffectKind::Archive => "archive",
        EffectKind::Publication => "publication",
        EffectKind::AudioTrash => "audio_trash",
        EffectKind::RetentionTrash => "retention_trash",
    }
}

pub(super) fn effect_outcome(outcome: EffectOutcome) -> &'static str {
    match outcome {
        EffectOutcome::Completed => "completed",
        EffectOutcome::Failed => "failed",
        EffectOutcome::Unknown => "unknown",
    }
}

pub(super) fn encode_reference(reference: &EffectReference) -> Result<Value, LedgerError> {
    Ok(match reference {
        EffectReference::ArchiveTarget(path) => {
            json!({"tag":"archive_target","path":path.to_str().ok_or_else(corrupt)?})
        }
        EffectReference::PublicationTarget(path) => {
            json!({"tag":"publication_target","path":path.to_str().ok_or_else(corrupt)?})
        }
        EffectReference::AudioTrash { recording, path } => {
            json!({"tag":"audio_trash","recording":recording.as_str(),"path":path.to_str().ok_or_else(corrupt)?})
        }
        EffectReference::RetentionIntent { id, path } => {
            json!({"tag":"retention_intent","id":id,"path":path.to_str().ok_or_else(corrupt)?})
        }
    })
}

pub(super) fn decode_reference(value: &Value) -> Result<EffectReference, LedgerError> {
    let path = PathBuf::from(string(value, "path")?);
    match string(value, "tag")? {
        "archive_target" => Ok(EffectReference::ArchiveTarget(path)),
        "publication_target" => Ok(EffectReference::PublicationTarget(path)),
        "audio_trash" => Ok(EffectReference::AudioTrash {
            recording: recording(value, "recording")?.ok_or_else(corrupt)?,
            path,
        }),
        "retention_intent" => Ok(EffectReference::RetentionIntent {
            id: unsigned(value, "id")?,
            path,
        }),
        _ => Err(corrupt()),
    }
}

pub(super) fn parse_run_kind(value: &str) -> Result<RunKind, LedgerError> {
    match value {
        "brief" => Ok(RunKind::Brief),
        "redact" => Ok(RunKind::Redact),
        "ingest" => Ok(RunKind::Ingest),
        "recover" => Ok(RunKind::Recover),
        "publish" => Ok(RunKind::Publish),
        "retain" => Ok(RunKind::Retain),
        "transcribe" => Ok(RunKind::Transcribe),
        "note" => Ok(RunKind::Note),
        "confirm" => Ok(RunKind::Confirm),
        "review" => Ok(RunKind::Review),
        "verify-note" => Ok(RunKind::VerifyNote),
        "analyze" => Ok(RunKind::Analyze),
        "run" => Ok(RunKind::Pipeline),
        _ => Err(corrupt()),
    }
}
