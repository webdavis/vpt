use super::*;
use vpt_domain::{identity::RecordingId, run::WorkOutcome};

pub(super) fn finish<E: EventSink>(
    context: &mut Context<'_, E>,
    run: RunId,
    id: &RecordingId,
    committed: bool,
    record_failure: bool,
    result: Result<(), Box<ErrorDocument>>,
) -> Result<(), Box<ErrorDocument>> {
    let stopped = result.as_ref().is_err_and(|error| {
        error.kind == ErrorKind::Ledger || error.rule.as_deref() == Some("unsafe_state")
    });
    let result = if stopped {
        result
    } else {
        let at = SystemClock.now();
        let mut facts = Vec::new();
        if record_failure && result.is_err() && !committed {
            facts.push((
                at,
                HistoryFact::WorkStep {
                    recording: Some(id.clone()),
                    outcome: WorkOutcome::Failed,
                },
            ));
        }
        facts.push((
            at,
            HistoryFact::RunFinished(crate::compose::run_outcome::from_result(&result)),
        ));
        commit(
            context
                .ledger
                .commit_step(run, &WorkDelta::default(), &facts),
            context.sink,
            context.warnings,
        )
        .map(|_| ())
        .and(result)
    };
    result.map_err(|mut error| {
        if committed && !error.completed.contains(&id.to_string()) {
            error.completed.push(id.to_string());
        }
        error
    })
}
