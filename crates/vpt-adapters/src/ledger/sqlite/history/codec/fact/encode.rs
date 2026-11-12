use super::*;
pub(in crate::ledger::sqlite::history::codec) fn encode(
    fact: &HistoryFact,
) -> Result<Value, LedgerError> {
    if !fact.referenced_ids_are_nonzero() {
        return Err(corrupt());
    }
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
        HistoryFact::AnalysisStateChanged {
            recording,
            from,
            to,
        } => json!({
            "tag":"analysis_state_changed", "recording":recording.as_str(),
            "from":from.as_str(), "to":to.as_str(),
        }),
        HistoryFact::TagRejected {
            recording,
            index,
            reason,
        } => {
            json!({"tag":"tag_rejected", "recording":recording.as_str(), "index":index, "reason":reason.as_str()})
        }
        HistoryFact::TagRenamed { recording, index } => {
            json!({"tag":"tag_renamed", "recording":recording.as_str(), "index":index})
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
            "tag":"note_moved", "recording":recording.as_str(),
            "from":from.to_str().ok_or_else(corrupt)?, "to":to.to_str().ok_or_else(corrupt)?,
        }),
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
        HistoryFact::MissingRetentionHistory { intent_id } => {
            json!({"tag":"missing_retention_history","intent_id":intent_id})
        }
        HistoryFact::HookFailed { event, failure } => {
            json!({"tag":"hook_failed","event":event.render(),"failure":encode_hook_failure(*failure)})
        }
        HistoryFact::Event(event) => json!({"tag":"event","event":event::encode(event)?}),
        HistoryFact::RecoveryProgress { evidence, baseline } => {
            if !baseline.is_well_formed() {
                return Err(corrupt());
            }
            json!({"tag":"recovery_progress","evidence":super::super::recovery::encode(evidence),
                "baseline":super::super::recovery::encode_baseline(*baseline)})
        }
        HistoryFact::RecoverySummary { evidence, window } => {
            json!({"tag":"recovery_summary","evidence":super::super::recovery::encode(evidence),"window":window.map(encode_window)})
        }
    })
}
