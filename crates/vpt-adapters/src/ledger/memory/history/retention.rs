use super::{MemoryLedger, state::conflict};
use vpt_application::ports::{
    HistoryReceipt, HistoryWrite, LedgerError, NewIntent, PreparedRetention, RetentionHistory,
    RetentionIntent,
};
use vpt_domain::history::{EventId, RunId};
use vpt_domain::run::{EffectKind, EffectOutcome, EffectReference, HistoryFact, WorkOutcome};
use vpt_domain::time::UtcInstant;

impl RetentionHistory for MemoryLedger {
    fn prepare_retention(&self, run: RunId, intent: &NewIntent) -> HistoryWrite<PreparedRetention> {
        self.history_write(|state| {
            if !state.history.has_run(run) {
                return Err(conflict());
            }
            if intent.path.to_str().is_none() {
                return Err(LedgerError::Corrupt("unreadable retention path".into()));
            }
            let id = state
                .next_intent_id
                .checked_add(1)
                .filter(|id| *id > 0)
                .ok_or_else(conflict)?;
            state.next_intent_id = id;
            state.intents.push((
                RetentionIntent {
                    id,
                    kind: intent.kind,
                    recording: intent.recording.clone(),
                    path: intent.path.clone(),
                    expected: intent.expected,
                    recorded_at: intent.recorded_at,
                },
                None,
            ));
            let saved = state.history.append(
                run,
                intent.recorded_at,
                HistoryFact::EffectIntent {
                    kind: EffectKind::RetentionTrash,
                    reference: EffectReference::RetentionIntent {
                        id: u64::try_from(id).map_err(|_| conflict())?,
                        path: intent.path.clone(),
                    },
                },
            )?;
            Ok(PreparedRetention {
                intent_id: id,
                effect_id: saved.id,
                receipt: HistoryReceipt {
                    run,
                    records: vec![saved],
                    completed: vec![],
                },
            })
        })
    }

    fn complete_retention(
        &self,
        run: RunId,
        intent_id: i64,
        effect_id: EventId,
        at: UtcInstant,
    ) -> HistoryWrite<HistoryReceipt> {
        self.history_write(|state| {
            if !state.history.has_run(run) {
                return Err(conflict());
            }
            let (intent, completed) = state
                .intents
                .iter_mut()
                .find(|(intent, _)| intent.id == intent_id)
                .ok_or_else(conflict)?;
            let expected = HistoryFact::EffectIntent {
                kind: EffectKind::RetentionTrash,
                reference: EffectReference::RetentionIntent {
                    id: u64::try_from(intent.id).map_err(|_| conflict())?,
                    path: intent.path.clone(),
                },
            };
            if !state
                .history
                .records
                .iter()
                .any(|saved| saved.id == effect_id && saved.fact == expected)
            {
                return Err(conflict());
            }
            let recording = state
                .recordings
                .iter_mut()
                .find(|recording| recording.id == intent.recording)
                .ok_or_else(conflict)?;
            if recording.audio_path != intent.path || recording.digest != intent.expected {
                return Err(conflict());
            }
            let mut receipt = HistoryReceipt {
                run,
                records: vec![],
                completed: vec![recording.id.clone()],
            };
            if completed.is_some() {
                if recording.audio_trashed_at.is_none() {
                    return Err(conflict());
                }
                return Ok(receipt);
            }
            recording.audio_trashed_at.get_or_insert(at);
            *completed = Some(at);
            for fact in [
                HistoryFact::EffectObserved {
                    intent: effect_id,
                    outcome: EffectOutcome::Completed,
                },
                HistoryFact::WorkStep {
                    recording: Some(recording.id.clone()),
                    outcome: WorkOutcome::Completed,
                },
            ] {
                receipt.records.push(state.history.append(run, at, fact)?);
            }
            Ok(receipt)
        })
    }
}

#[cfg(test)]
mod tests;
