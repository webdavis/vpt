use super::{Kept, KeptReason, Retention, RetentionError, RetentionReport, history};
use crate::ports::{
    Clock, EventSink, RecordingLedger, RetentionHistory, RetentionJournal, RunHistory, StoreError,
    Stores, Trash,
};
use vpt_domain::history::RunId;
use vpt_domain::layout::StoreKey;
use vpt_domain::retention::expired;

impl<L, S, T, C, E> Retention<'_, L, S, T, C, E>
where
    L: RecordingLedger + RetentionJournal + RetentionHistory + RunHistory,
    S: Stores,
    T: Trash,
    C: Clock,
    E: EventSink,
{
    pub(super) fn sweep(
        &mut self,
        run: Option<RunId>,
        report: &mut RetentionReport,
    ) -> Result<(), Box<history::Failure>> {
        let owned = self
            .ledger
            .recordings()
            .map_err(|error| history::failure(RetentionError::Ledger(error)))?;
        for key in StoreKey::all() {
            if key == StoreKey::Audio && !self.settings.include_audio {
                continue;
            }
            let hold = self.settings.hold(key);
            if hold.seconds() == 0 {
                continue;
            }
            let root = self.paths.get(key);
            let entries = self
                .stores
                .entries(root)
                .map_err(|error| history::failure(RetentionError::Stores(error)))?;
            for entry in entries {
                if entry.path.parent() != Some(root) {
                    return Err(history::failure(RetentionError::Stores(
                        StoreError::Escape(entry.path),
                    )));
                }
                let record = owned.iter().find(|record| {
                    key == StoreKey::Audio
                        && record.audio_path == entry.path
                        && record.audio_trashed_at.is_none()
                });
                let Some(record) = record else {
                    report.kept.push(Kept {
                        path: entry.path,
                        reason: KeptReason::Untracked,
                    });
                    continue;
                };
                if expired(entry.mtime, hold, self.clock.now()) {
                    self.expire(run, record, report)?;
                }
            }
        }
        Ok(())
    }
}
