use vpt_application::ports::{LedgerError, StageState, SynthesisStateChange, WorkDelta};
use vpt_domain::{run::HistoryFact, time::UtcInstant};

pub(super) fn check<'a>(
    delta: &'a WorkDelta,
    facts: &[(UtcInstant, HistoryFact)],
) -> Result<Option<&'a SynthesisStateChange>, LedgerError> {
    let mut transitions = facts
        .iter()
        .filter(|(_, fact)| matches!(fact, HistoryFact::SynthesisStateChanged { .. }));
    let Some(change) = &delta.synthesis_state else {
        return if transitions.next().is_none() {
            Ok(None)
        } else {
            Err(conflict())
        };
    };
    let valid = matches!(
        (change.expected, change.next),
        (
            StageState::Pending | StageState::Failed,
            StageState::Disabled
        ) | (
            StageState::Disabled | StageState::Failed | StageState::Succeeded,
            StageState::Pending
        )
    );
    let only_state = WorkDelta {
        synthesis_state: Some(change.clone()),
        ..Default::default()
    };
    if !valid
        || transitions.next() != Some(&change.fact())
        || transitions.next().is_some()
        || delta != &only_state
    {
        return Err(conflict());
    }
    Ok(Some(change))
}

pub(super) fn conflict() -> LedgerError {
    LedgerError::Conflict("synthesis state transition is inconsistent".into())
}
