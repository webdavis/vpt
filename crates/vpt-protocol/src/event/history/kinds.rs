use super::*;

pub(super) fn run_kind(value: RunKind) -> &'static str {
    match value {
        RunKind::Confirm => "confirm",
        RunKind::Review => "review",
        RunKind::VerifyNote => "verify-note",
        RunKind::Synthesize => "synthesize",
        RunKind::Pipeline => "run",
        RunKind::Brief => "brief",
        RunKind::Redact => "redact",
        RunKind::Ingest => "ingest",
        RunKind::Recover => "recover",
        RunKind::Publish => "publish",
        RunKind::Retain => "retain",
        RunKind::Transcribe => "transcribe",
        RunKind::Note => "note",
    }
}

pub(super) fn run_outcome(value: RunOutcome) -> &'static str {
    match value {
        RunOutcome::Completed => "completed",
        RunOutcome::Failed => "failed",
        RunOutcome::Interrupted => "interrupted",
    }
}

pub(super) fn work_outcome(value: WorkOutcome) -> &'static str {
    match value {
        WorkOutcome::Completed => "completed",
        WorkOutcome::Failed => "failed",
        WorkOutcome::Unknown => "unknown",
        WorkOutcome::Deferred => "deferred",
    }
}

pub(super) fn effect_kind(value: EffectKind) -> &'static str {
    match value {
        EffectKind::Archive => "archive",
        EffectKind::Publication => "publication",
        EffectKind::AudioTrash => "audio_trash",
        EffectKind::RetentionTrash => "retention_trash",
    }
}

pub(super) fn effect_outcome(value: EffectOutcome) -> &'static str {
    match value {
        EffectOutcome::Completed => "completed",
        EffectOutcome::Failed => "failed",
        EffectOutcome::Unknown => "unknown",
    }
}

pub(super) fn hook_failure(value: HookFailureClass) -> Value {
    match value {
        HookFailureClass::Spawn => json!({"tag": "spawn"}),
        HookFailureClass::Io => json!({"tag": "io"}),
        HookFailureClass::Encoding => json!({"tag": "encoding"}),
        HookFailureClass::Exited(code) => json!({"tag": "exited", "code": code}),
        HookFailureClass::Signaled(signal) => json!({"tag": "signaled", "signal": signal}),
        HookFailureClass::DeadlineExceeded => json!({"tag": "deadline_exceeded"}),
        HookFailureClass::Interrupted => json!({"tag": "interrupted"}),
    }
}
