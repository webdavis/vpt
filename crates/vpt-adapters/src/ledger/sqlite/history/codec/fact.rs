use super::{
    corrupt, event, event_id, failure_id, field, optional, recording, signed, string, unsigned,
};
use serde_json::{Value, json};
use std::path::PathBuf;
use vpt_application::ports::LedgerError;
use vpt_domain::run::{
    EffectKind, EffectOutcome, EffectReference, FailureWindow, HistoryFact, HookFailureClass,
    RunKind, RunOutcome, WorkOutcome,
};

pub(super) fn encode(fact: &HistoryFact) -> Result<Value, LedgerError> {
    if !fact.recovery_run_ids_are_nonzero() {
        return Err(corrupt());
    }
    Ok(match fact {
        HistoryFact::RunStarted(kind) => json!({"tag":"run_started","kind":run_kind(*kind)}),
        HistoryFact::RunFinished(outcome) => {
            json!({"tag":"run_finished","outcome":run_outcome(*outcome)})
        }
        HistoryFact::WorkStep { recording, outcome } => json!({
            "tag":"work","recording":recording.as_ref().map(|id| id.as_str()),"outcome":work_outcome(*outcome)
        }),
        HistoryFact::EffectIntent { kind, reference } => {
            if !reference.matches_kind(*kind) {
                return Err(corrupt());
            }
            json!({"tag":"effect_intent","kind":effect_kind(*kind),"reference":encode_reference(reference)?})
        }
        HistoryFact::EffectObserved { intent, outcome } => {
            json!({"tag":"effect_observed","intent":intent.render(),"outcome":effect_outcome(*outcome)})
        }
        HistoryFact::HookFailed { event, failure } => {
            json!({"tag":"hook_failed","event":event.render(),"failure":encode_hook_failure(*failure)})
        }
        HistoryFact::Event(event) => json!({"tag":"event","event":event::encode(event)?}),
        HistoryFact::RecoveryProgress(evidence) => {
            json!({"tag":"recovery_progress","evidence":super::recovery::encode(evidence)})
        }
        HistoryFact::RecoverySummary { evidence, window } => {
            json!({"tag":"recovery_summary","evidence":super::recovery::encode(evidence),"window":window.map(encode_window)})
        }
    })
}

pub(super) fn decode(value: &Value) -> Result<HistoryFact, LedgerError> {
    match string(value, "tag")? {
        "run_started" => Ok(HistoryFact::RunStarted(match string(value, "kind")? {
            "ingest" => RunKind::Ingest,
            "recover" => RunKind::Recover,
            "publish" => RunKind::Publish,
            "retain" => RunKind::Retain,
            _ => return Err(corrupt()),
        })),
        "run_finished" => Ok(HistoryFact::RunFinished(match string(value, "outcome")? {
            "completed" => RunOutcome::Completed,
            "failed" => RunOutcome::Failed,
            "interrupted" => RunOutcome::Interrupted,
            _ => return Err(corrupt()),
        })),
        "work" => Ok(HistoryFact::WorkStep {
            recording: recording(value, "recording")?,
            outcome: match string(value, "outcome")? {
                "completed" => WorkOutcome::Completed,
                "failed" => WorkOutcome::Failed,
                "unknown" => WorkOutcome::Unknown,
                _ => return Err(corrupt()),
            },
        }),
        "effect_intent" => {
            let kind = match string(value, "kind")? {
                "archive" => EffectKind::Archive,
                "publication" => EffectKind::Publication,
                "audio_trash" => EffectKind::AudioTrash,
                "retention_trash" => EffectKind::RetentionTrash,
                _ => return Err(corrupt()),
            };
            let reference = decode_reference(field(value, "reference")?)?;
            if !reference.matches_kind(kind) {
                return Err(corrupt());
            }
            Ok(HistoryFact::EffectIntent { kind, reference })
        }
        "effect_observed" => Ok(HistoryFact::EffectObserved {
            intent: event_id(value, "intent")?,
            outcome: match string(value, "outcome")? {
                "completed" => EffectOutcome::Completed,
                "failed" => EffectOutcome::Failed,
                "unknown" => EffectOutcome::Unknown,
                _ => return Err(corrupt()),
            },
        }),
        "hook_failed" => Ok(HistoryFact::HookFailed {
            event: event_id(value, "event")?,
            failure: decode_hook_failure(field(value, "failure")?)?,
        }),
        "event" => Ok(HistoryFact::Event(event::decode(field(value, "event")?)?)),
        "recovery_progress" => Ok(HistoryFact::RecoveryProgress(super::recovery::decode(
            field(value, "evidence")?,
        )?)),
        "recovery_summary" => Ok(HistoryFact::RecoverySummary {
            evidence: super::recovery::decode(field(value, "evidence")?)?,
            window: optional(value, "window")?.map(decode_window).transpose()?,
        }),
        _ => Err(corrupt()),
    }
}

fn encode_hook_failure(failure: HookFailureClass) -> Value {
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

fn decode_hook_failure(value: &Value) -> Result<HookFailureClass, LedgerError> {
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

fn encode_window(window: FailureWindow) -> Value {
    json!({"after":window.after.map(|id| id.render()),"through":window.through.render()})
}

fn decode_window(value: &Value) -> Result<FailureWindow, LedgerError> {
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
        RunKind::Ingest => "ingest",
        RunKind::Recover => "recover",
        RunKind::Publish => "publish",
        RunKind::Retain => "retain",
    }
}

fn run_outcome(outcome: RunOutcome) -> &'static str {
    match outcome {
        RunOutcome::Completed => "completed",
        RunOutcome::Failed => "failed",
        RunOutcome::Interrupted => "interrupted",
    }
}

fn work_outcome(outcome: WorkOutcome) -> &'static str {
    match outcome {
        WorkOutcome::Completed => "completed",
        WorkOutcome::Failed => "failed",
        WorkOutcome::Unknown => "unknown",
    }
}

fn effect_kind(kind: EffectKind) -> &'static str {
    match kind {
        EffectKind::Archive => "archive",
        EffectKind::Publication => "publication",
        EffectKind::AudioTrash => "audio_trash",
        EffectKind::RetentionTrash => "retention_trash",
    }
}

fn effect_outcome(outcome: EffectOutcome) -> &'static str {
    match outcome {
        EffectOutcome::Completed => "completed",
        EffectOutcome::Failed => "failed",
        EffectOutcome::Unknown => "unknown",
    }
}

fn encode_reference(reference: &EffectReference) -> Result<Value, LedgerError> {
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

fn decode_reference(value: &Value) -> Result<EffectReference, LedgerError> {
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
