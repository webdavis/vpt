use super::*;
pub(super) fn value(fact: &HistoryFact) -> Result<Value, EventEncodeError> {
    Ok(match fact {
        HistoryFact::BriefSaved { occasion } => {
            json!({"tag":"brief_saved","occasion":occasion.as_str()})
        }
        HistoryFact::BriefPublished { occasion } => {
            json!({"tag":"brief_published","occasion":occasion.as_str()})
        }
        HistoryFact::OccasionSaved { occasion } => {
            json!({"tag":"occasion_saved", "occasion": occasion.as_str()})
        }
        HistoryFact::SynthesisStateChanged {
            recording,
            from,
            to,
        } => json!({
            "tag":"synthesis_state_changed", "recording":recording.as_str(),
            "from":from.as_str(), "to":to.as_str(),
        }),
        HistoryFact::TagRejected {
            recording,
            index,
            reason,
        } => {
            json!({"tag":"tag_rejected", "recording":recording.as_str(), "index":index, "reason":reason.as_str()})
        }
        HistoryFact::FlagResolved {
            recording,
            flag,
            resolution,
        } => json!({
            "tag":"flag_resolved", "recording":recording.as_str(), "flag":flag.hex(), "resolution":resolution.as_str(),
        }),
        HistoryFact::NoteMoved {
            recording,
            from,
            to,
        } => json!({
            "tag":"note_moved", "recording":recording.as_str(), "from":path(from)?, "to":path(to)?,
        }),
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
