use super::Writer;
use crate::compose::history::commit;
use vpt_adapters::SystemClock;
use vpt_application::ports::*;
use vpt_domain::{
    history::RunId,
    identity::RecordingId,
    run::{HistoryFact, WorkOutcome},
};
use vpt_protocol::error::ErrorDocument;

pub(super) fn failure<E: EventSink>(
    writer: &mut Writer<'_, E>,
    run: RunId,
    id: &RecordingId,
    attempted: bool,
) -> Result<(), Box<ErrorDocument>> {
    let at = SystemClock.now();
    let delta = WorkDelta {
        proposal: attempted.then(|| ProposalChange::Fail {
            recording: id.clone(),
            at,
        }),
        ..Default::default()
    };
    commit(
        writer.ledger.commit_step(
            run,
            &delta,
            &[(
                at,
                HistoryFact::WorkStep {
                    recording: Some(id.clone()),
                    outcome: WorkOutcome::Failed,
                },
            )],
        ),
        writer.sink,
        writer.warnings,
    )?;
    Ok(())
}

pub(super) fn pending<E: EventSink>(
    writer: &mut Writer<'_, E>,
    run: RunId,
    input: &mut NoteInputs,
) -> Result<(), Box<ErrorDocument>> {
    crate::compose::synthesis_state::set(
        writer.ledger,
        writer.sink,
        writer.warnings,
        run,
        &mut input.record,
        StageState::Pending,
    )
}
