use super::{corrupt, count, event_id, field, optional, run_id};
use serde_json::{Value, json};
use vpt_application::ports::LedgerError;
use vpt_domain::run::{RecoveryBaseline, RecoveryEvidence};
use vpt_domain::time::UtcInstant;

pub(super) fn encode(evidence: &RecoveryEvidence) -> Value {
    let affected_runs: Vec<_> = evidence
        .affected_runs
        .iter()
        .map(|run| run.render())
        .collect();
    json!({
        "affected_runs":affected_runs,
        "first_failure_observed_at":evidence.first_failure_observed_at.map(|at| at.secs),
        "logging_restored_observed_at":evidence.logging_restored_observed_at.map(|at| at.secs),
        "failures":evidence.failures,
        "recovered_records":evidence.recovered_records,
        "completed_work":evidence.completed_work,
        "failed_work":evidence.failed_work,
        "unknown_work":evidence.unknown_work,
        "deferred_work":evidence.deferred_work,
        "completed_effects":evidence.completed_effects,
        "failed_effects":evidence.failed_effects,
        "unresolved_effects":evidence.unresolved_effects,
        "missing_history":evidence.missing_history,
    })
}

pub(super) fn decode(value: &Value) -> Result<RecoveryEvidence, LedgerError> {
    let affected_runs = field(value, "affected_runs")?
        .as_array()
        .ok_or_else(corrupt)?
        .iter()
        .map(|run| {
            let wrapped = json!({"id":run});
            run_id(&wrapped, "id")
        })
        .collect::<Result<Vec<_>, _>>()?;
    let restored = optional(value, "logging_restored_observed_at")?
        .map(|value| {
            value
                .as_i64()
                .map(|secs| UtcInstant { secs })
                .ok_or_else(corrupt)
        })
        .transpose()?;
    Ok(RecoveryEvidence {
        affected_runs,
        first_failure_observed_at: optional(value, "first_failure_observed_at")?
            .map(|value| {
                value
                    .as_i64()
                    .map(|secs| UtcInstant { secs })
                    .ok_or_else(corrupt)
            })
            .transpose()?,
        logging_restored_observed_at: restored,
        failures: count(value, "failures")?,
        recovered_records: count(value, "recovered_records")?,
        completed_work: count(value, "completed_work")?,
        failed_work: count(value, "failed_work")?,
        unknown_work: count(value, "unknown_work")?,
        deferred_work: count(value, "deferred_work")?,
        completed_effects: count(value, "completed_effects")?,
        failed_effects: count(value, "failed_effects")?,
        unresolved_effects: count(value, "unresolved_effects")?,
        missing_history: count(value, "missing_history")?,
    })
}

pub(super) fn encode_baseline(baseline: RecoveryBaseline) -> Value {
    json!({
        "generation":baseline.generation,
        "pending_from":baseline.pending_from.map(|id| id.render()),
        "through":baseline.through.map(|id| id.render()),
    })
}

pub(super) fn decode_baseline(value: &Value) -> Result<RecoveryBaseline, LedgerError> {
    let baseline = RecoveryBaseline {
        generation: optional(value, "generation")?
            .map(|value| value.as_u64().ok_or_else(corrupt))
            .transpose()?,
        pending_from: optional(value, "pending_from")?
            .map(|value| event_id(&json!({"id":value}), "id"))
            .transpose()?,
        through: optional(value, "through")?
            .map(|value| event_id(&json!({"id":value}), "id"))
            .transpose()?,
    };
    baseline
        .is_well_formed()
        .then_some(baseline)
        .ok_or_else(corrupt)
}
