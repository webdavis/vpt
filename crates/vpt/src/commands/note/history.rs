use vpt_application::ports::WorkDelta;
use vpt_domain::{
    identity::RecordingId,
    run::{HistoryFact, WorkOutcome},
    time::UtcInstant,
};

pub(crate) fn facts(
    delta: &WorkDelta,
    id: &RecordingId,
    at: UtcInstant,
) -> Vec<(UtcInstant, HistoryFact)> {
    let mut facts = vec![(
        at,
        HistoryFact::WorkStep {
            recording: Some(id.clone()),
            outcome: WorkOutcome::Completed,
        },
    )];
    for change in &delta.notes {
        if let Some(from) = &change.relocated_from {
            facts.push((
                at,
                HistoryFact::NoteMoved {
                    recording: id.clone(),
                    from: from.clone(),
                    to: change.artifact.target.clone(),
                },
            ));
        }
    }
    facts
}
