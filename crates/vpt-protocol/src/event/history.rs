use super::{EventEncodeError, event_value, path, timestamp};
use serde_json::{Value, json};
use vpt_domain::run::{
    EffectKind, EffectOutcome, EffectReference, FailureWindow, HistoryFact, HistoryRecord,
    HookFailureClass, RecoveryBaseline, RecoveryEvidence, RunKind, RunOutcome, WorkOutcome,
};
use vpt_domain::time::UtcInstant;

pub(super) fn encode(
    record: &HistoryRecord,
    appended_at: UtcInstant,
) -> Result<String, EventEncodeError> {
    let fact = fact_value(&record.fact)?;
    Ok(json!({
        "schema": "vpt.log/1",
        "event_id": record.id.render(),
        "run_id": record.run.render(),
        "sequence": record.id.sequence(),
        "occurred_at": timestamp(record.occurred_at)?,
        "appended_at": timestamp(appended_at)?,
        "fact": fact,
    })
    .to_string())
}

fn fact_value(fact: &HistoryFact) -> Result<Value, EventEncodeError> {
    Ok(match fact {
        HistoryFact::RunStarted(kind) => json!({
            "tag": "run_started", "kind": run_kind(*kind),
        }),
        HistoryFact::RunFinished(outcome) => json!({
            "tag": "run_finished", "outcome": run_outcome(*outcome),
        }),
        HistoryFact::WorkStep { recording, outcome } => json!({
            "tag": "work_step",
            "recording": recording.as_ref().map(|id| id.as_str()),
            "outcome": work_outcome(*outcome),
        }),
        HistoryFact::EffectIntent { kind, reference } => json!({
            "tag": "effect_intent",
            "kind": effect_kind(*kind),
            "reference": reference_value(reference)?,
        }),
        HistoryFact::EffectObserved { intent, outcome } => json!({
            "tag": "effect_observed",
            "intent": intent.render(),
            "outcome": effect_outcome(*outcome),
        }),
        HistoryFact::MissingRetentionHistory { intent_id } => json!({
            "tag": "missing_retention_history", "intent_id": intent_id,
        }),
        HistoryFact::HookFailed { event, failure } => json!({
            "tag": "hook_failed",
            "event": event.render(),
            "failure": hook_failure(*failure),
        }),
        HistoryFact::Event(event) => json!({
            "tag": "event", "event": event_value(event)?,
        }),
        HistoryFact::RecoveryProgress { evidence, baseline } => json!({
            "tag": "recovery_progress", "evidence": evidence_value(evidence)?,
            "baseline": baseline_value(*baseline),
        }),
        HistoryFact::RecoverySummary { evidence, window } => json!({
            "tag": "recovery_summary",
            "evidence": evidence_value(evidence)?,
            "window": window.map(window_value),
        }),
    })
}

fn baseline_value(baseline: RecoveryBaseline) -> Value {
    json!({
        "generation": baseline.generation,
        "pending_from": baseline.pending_from.map(|id| id.render()),
        "through": baseline.through.map(|id| id.render()),
    })
}

fn reference_value(reference: &EffectReference) -> Result<Value, EventEncodeError> {
    Ok(match reference {
        EffectReference::ArchiveTarget(target) => json!({
            "tag": "archive_target", "path": path(target)?,
        }),
        EffectReference::PublicationTarget(target) => json!({
            "tag": "publication_target", "path": path(target)?,
        }),
        EffectReference::AudioTrash {
            recording,
            path: target,
        } => json!({
            "tag": "audio_trash", "recording": recording.as_str(), "path": path(target)?,
        }),
        EffectReference::RetentionIntent { id, path: target } => json!({
            "tag": "retention_intent", "id": id, "path": path(target)?,
        }),
    })
}

fn evidence_value(evidence: &RecoveryEvidence) -> Result<Value, EventEncodeError> {
    Ok(json!({
        "affected_runs": evidence.affected_runs.iter().map(|run| run.render()).collect::<Vec<_>>(),
        "first_failure_observed_at": evidence.first_failure_observed_at.map(timestamp).transpose()?,
        "logging_restored_observed_at": evidence.logging_restored_observed_at.map(timestamp).transpose()?,
        "failures": evidence.failures,
        "recovered_records": evidence.recovered_records,
        "completed_work": evidence.completed_work,
        "failed_work": evidence.failed_work,
        "unknown_work": evidence.unknown_work,
        "deferred_work": evidence.deferred_work,
        "completed_effects": evidence.completed_effects,
        "failed_effects": evidence.failed_effects,
        "unresolved_effects": evidence.unresolved_effects,
        "missing_history": evidence.missing_history,
    }))
}

fn window_value(window: FailureWindow) -> Value {
    json!({
        "after": window.after.map(|id| id.render()),
        "through": window.through.render(),
    })
}

fn run_kind(value: RunKind) -> &'static str {
    match value {
        RunKind::Ingest => "ingest",
        RunKind::Recover => "recover",
        RunKind::Publish => "publish",
        RunKind::Retain => "retain",
    }
}

fn run_outcome(value: RunOutcome) -> &'static str {
    match value {
        RunOutcome::Completed => "completed",
        RunOutcome::Failed => "failed",
        RunOutcome::Interrupted => "interrupted",
    }
}

fn work_outcome(value: WorkOutcome) -> &'static str {
    match value {
        WorkOutcome::Completed => "completed",
        WorkOutcome::Failed => "failed",
        WorkOutcome::Unknown => "unknown",
        WorkOutcome::Deferred => "deferred",
    }
}

fn effect_kind(value: EffectKind) -> &'static str {
    match value {
        EffectKind::Archive => "archive",
        EffectKind::Publication => "publication",
        EffectKind::AudioTrash => "audio_trash",
        EffectKind::RetentionTrash => "retention_trash",
    }
}

fn effect_outcome(value: EffectOutcome) -> &'static str {
    match value {
        EffectOutcome::Completed => "completed",
        EffectOutcome::Failed => "failed",
        EffectOutcome::Unknown => "unknown",
    }
}

fn hook_failure(value: HookFailureClass) -> Value {
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
