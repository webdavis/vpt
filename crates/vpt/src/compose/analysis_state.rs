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
    if record.stages.analysis == next {
        return Ok(());
    }
    let change = AnalysisStateChange {
        recording: record.id.clone(),
        expected: record.stages.analysis,
        next,
        at: SystemClock.now(),
    };
    let fact = change.fact();
    commit(
        ledger.commit_step(
            run,
            &WorkDelta {
                analysis_state: Some(change),
                ..Default::default()
            },
            &[fact],
        ),
        sink,
        warnings,
    )?;
    record.stages.analysis = next;
    Ok(())
}
