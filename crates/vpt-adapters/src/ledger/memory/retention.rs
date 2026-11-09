use super::*;

impl RetentionJournal for MemoryLedger {
    fn record_intent(&self, intent: &NewIntent) -> Result<i64, LedgerError> {
        with(self, |state| {
            if !intent.owner.matches_kind(intent.kind) {
                return Err(LedgerError::Conflict(
                    "retention owner kind mismatch".into(),
                ));
            }
            let id = state
                .next_intent_id
                .checked_add(1)
                .ok_or_else(|| LedgerError::Conflict("intent identifier exhausted".into()))?;
            state.intents.push((
                RetentionIntent {
                    id,
                    kind: intent.kind,
                    owner: intent.owner.clone(),
                    path: intent.path.clone(),
                    expected: intent.expected,
                    recorded_at: intent.recorded_at,
                },
                None,
            ));
            state.next_intent_id = id;
            Ok(id)
        })?
    }

    fn pending_intents(&self) -> Result<Vec<RetentionIntent>, LedgerError> {
        with(self, |state| {
            state
                .intents
                .iter()
                .filter(|(_, done)| done.is_none())
                .map(|(intent, _)| intent.clone())
                .collect()
        })
    }

    fn complete_intent(&self, id: i64, at: UtcInstant) -> Result<(), LedgerError> {
        with(self, |state| {
            if let Some((_, done)) = state.intents.iter_mut().find(|(intent, _)| intent.id == id)
                && done.is_none()
            {
                *done = Some(at);
            }
        })
    }
}
