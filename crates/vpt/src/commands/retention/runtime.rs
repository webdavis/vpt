use vpt_application::ports::{
    EventSink, RetentionHistory, RetentionJournal, RunHistory, Stores, Trash,
};
use vpt_application::{ReconcileProgress, Retention, RetentionFailure, RetentionReport};
use vpt_application::{RetentionSettings, StorePaths};

#[allow(
    clippy::too_many_arguments,
    reason = "command composition supplies independent ports"
)]
pub(crate) fn execute<L, S, T, E>(
    ledger: &L,
    stores: &S,
    trash: &T,
    sink: &mut E,
    settings: &RetentionSettings,
    paths: &StorePaths,
    dry_run: bool,
    initial: ReconcileProgress,
) -> Result<RetentionReport, Box<RetentionFailure>>
where
    L: RetentionHistory
        + RetentionJournal
        + RunHistory
        + vpt_application::ports::RetentionArtifacts,
    S: Stores + vpt_application::ports::RetentionFiles,
    T: Trash,
    E: EventSink,
{
    Retention {
        ledger,
        stores,
        trash,
        clock: &vpt_adapters::SystemClock,
        sink,
        settings,
        paths,
    }
    .run(dry_run, initial)
}
