use super::{corrupt, count, field, optional, run_id};
use serde_json::{Value, json};
use vpt_application::ports::LedgerError;
use vpt_domain::run::RecoveryEvidence;
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
        "completed":evidence.completed,
        "failed":evidence.failed,
        "unresolved":evidence.unresolved,
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
        completed: count(value, "completed")?,
        failed: count(value, "failed")?,
        unresolved: count(value, "unresolved")?,
        missing_history: count(value, "missing_history")?,
    })
}
