use super::{Ingest, IngestReport};
use crate::ports::{
    Archive, Clock, EventSink, RecorderStore, RecordingLedger, RunHistory, Trash, TrashError,
};

impl<R, A, L, C, T, S> Ingest<'_, R, A, L, C, T, S>
where
    R: RecorderStore,
    A: Archive,
    L: RecordingLedger + RunHistory,
    C: Clock,
    T: Trash + ?Sized,
    S: EventSink + ?Sized,
{
    pub(super) fn discard_owned(
        &self,
        staged: &std::path::Path,
        report: &mut IngestReport,
    ) -> Option<TrashError> {
        if self.archive.validate_trash_path(staged).is_err() {
            report
                .log
                .push("staged cleanup pending: ownership validation failed".into());
            return None;
        }
        let outcome = self.trash.trash(staged);
        for diagnostic in self.trash.take_diagnostics().into_iter().take(16) {
            let field = diagnostic.strip_prefix("unknown additive helper field: ");
            if diagnostic.len() <= 400
                && field.is_some_and(|field| {
                    field
                        .bytes()
                        .all(|byte| byte.is_ascii() && !byte.is_ascii_control())
                })
            {
                report.log.push(diagnostic);
            } else {
                report.log.push("trash reported a diagnostic".into());
            }
        }
        let error = outcome.err();
        match &error {
            None => {}
            Some(TrashError::HelperAbsent) => report
                .log
                .push("staged cleanup pending: helper unavailable".into()),
            Some(TrashError::HelperVersion { found }) => report.log.push(format!(
                "staged cleanup pending: incompatible helper major {found}"
            )),
            Some(TrashError::Failed(_)) => report
                .log
                .push("staged cleanup pending: helper failed".into()),
            Some(TrashError::Unknown(_)) => report
                .log
                .push("staged cleanup pending: helper reply unknown".into()),
        }
        error
    }
}
