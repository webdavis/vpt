use super::history::commit;
use std::collections::BTreeSet;
use vpt_adapters::SystemClock;
use vpt_application::ports::*;
use vpt_domain::history::RunId;
use vpt_protocol::error::ErrorDocument;

pub(crate) fn set(
    ledger: &impl RunHistory,
    sink: &mut impl EventSink,
    warnings: &mut BTreeSet<SinkWarning>,
    run: RunId,
    record: &mut RecordingRecord,
    next: StageState,
) -> Result<(), Box<ErrorDocument>> {
    if record.stages.synthesis == next {
        return Ok(());
    }
    let change = SynthesisStateChange {
        recording: record.id.clone(),
        expected: record.stages.synthesis,
        next,
        at: SystemClock.now(),
    };
    let fact = change.fact();
    commit(
        ledger.commit_step(
            run,
            &WorkDelta {
                synthesis_state: Some(change),
                ..Default::default()
            },
            &[fact],
        ),
        sink,
        warnings,
    )?;
    record.stages.synthesis = next;
    Ok(())
}
