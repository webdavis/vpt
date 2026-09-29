use std::collections::{HashMap, HashSet};
use vpt_application::ports::LedgerError;
use vpt_domain::history::{EventId, RunId};
use vpt_domain::run::{EffectOutcome, HistoryFact, HistoryRecord, WorkOutcome};

#[derive(Default)]
pub(super) struct Counts {
    pub(super) completed_work: u32,
    pub(super) failed_work: u32,
    pub(super) unknown_work: u32,
    pub(super) deferred_work: u32,
    pub(super) completed_effects: u32,
    pub(super) failed_effects: u32,
    pub(super) unresolved_effects: u32,
    pub(super) missing_history: u32,
}

pub(super) fn from_records(
    records: &[HistoryRecord],
    affected: &[RunId],
) -> Result<Counts, LedgerError> {
    let mut outcomes: HashMap<EventId, EffectOutcome> = HashMap::new();
    let mut missing_ids = HashSet::new();
    let mut counts = Counts::default();
    for record in records {
        match &record.fact {
            HistoryFact::WorkStep { outcome, .. } if affected.contains(&record.run) => {
                let slot = match outcome {
                    WorkOutcome::Completed => &mut counts.completed_work,
                    WorkOutcome::Failed => &mut counts.failed_work,
                    WorkOutcome::Unknown => &mut counts.unknown_work,
                    WorkOutcome::Deferred => &mut counts.deferred_work,
                };
                *slot = slot.checked_add(1).ok_or_else(overflow)?;
            }
            HistoryFact::EffectIntent { .. } if affected.contains(&record.run) => {
                outcomes.insert(record.id, EffectOutcome::Unknown);
            }
            HistoryFact::EffectObserved { intent, outcome } => {
                if let Some(current) = outcomes.get_mut(intent) {
                    *current = *outcome;
                }
            }
            HistoryFact::MissingRetentionHistory { intent_id }
                if affected.contains(&record.run) =>
            {
                missing_ids.insert(*intent_id);
            }
            _ => {}
        }
    }
    for outcome in outcomes.values() {
        let slot = match outcome {
            EffectOutcome::Completed => &mut counts.completed_effects,
            EffectOutcome::Failed => &mut counts.failed_effects,
            EffectOutcome::Unknown => &mut counts.unresolved_effects,
        };
        *slot = slot.checked_add(1).ok_or_else(overflow)?;
    }
    counts.missing_history = u32::try_from(missing_ids.len()).map_err(|_| overflow())?;
    Ok(counts)
}

fn overflow() -> LedgerError {
    LedgerError::Corrupt("recovery count overflow".into())
}
