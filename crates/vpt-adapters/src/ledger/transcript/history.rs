use vpt_application::ports::LedgerError;
use vpt_domain::time::UtcInstant;

pub(in crate::ledger) fn check_history(
    change: &vpt_application::ports::TranscriptionChange,
    facts: &[(UtcInstant, vpt_domain::run::HistoryFact)],
) -> Result<(), LedgerError> {
    use vpt_application::ports::TranscriptionChange;
    use vpt_domain::run::{HistoryFact, WorkOutcome};
    let expected = match change {
        TranscriptionChange::Accept(_) => WorkOutcome::Completed,
        TranscriptionChange::Fail(_) => WorkOutcome::Failed,
    };
    let mut outcomes = facts.iter().filter_map(|(_, fact)| match fact {
        HistoryFact::WorkStep {
            recording: Some(id),
            outcome,
        } if id == change.recording() => Some(*outcome),
        _ => None,
    });
    if outcomes.next() != Some(expected) || outcomes.next().is_some() {
        return Err(LedgerError::Conflict(
            "transcription requires matching history".into(),
        ));
    }
    Ok(())
}
