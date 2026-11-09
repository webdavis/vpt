mod fact;
mod kinds;
use super::{EventEncodeError, event_value, path, timestamp};
use kinds::*;
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
    let fact = fact::value(&record.fact)?;
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
