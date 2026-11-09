use super::{Kept, KeptReason, Retention, RetentionError, RetentionReport, history};
use crate::ports::{
    Clock, EventSink, RetentionHistory, RetentionJournal, RetentionStatus, RunHistory, StoreError,
    Stores, Trash,
};
use vpt_domain::{history::RunId, layout::StoreKey, retention::expired};

impl<L, S, T, C, E> Retention<'_, L, S, T, C, E>
where
    L: RetentionJournal + RetentionHistory + RunHistory + crate::ports::RetentionArtifacts,
    S: Stores + crate::ports::RetentionFiles,
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
            .retention_artifacts()
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
            report.kept.extend(
                owned
                    .iter()
                    .filter(|row| {
                        row.kind.store() == key
                            && row.status != RetentionStatus::Expired
                            && row.path.parent() != Some(root)
                    })
                    .map(|row| Kept {
                        path: row.path.clone(),
                        reason: KeptReason::PathEscape,
                    }),
            );
            let entries = self
                .stores
                .retention_entries(root)
                .map_err(|error| history::failure(RetentionError::Stores(error)))?;
            report
                .kept
                .extend(entries.redirected.into_iter().map(|path| Kept {
                    path,
                    reason: KeptReason::PathEscape,
                }));
            for entry in entries.files {
                if entry.path.parent() != Some(root) {
                    return Err(history::failure(RetentionError::Stores(
                        StoreError::Escape(entry.path),
                    )));
                }
                let artifact = owned
                    .binary_search_by(|row| row.path.cmp(&entry.path))
                    .ok()
                    .map(|index| &owned[index])
                    .filter(|row| row.kind.store() == key);
                let reason = match artifact.map(|row| row.status) {
                    None => Some(KeptReason::Untracked),
                    Some(RetentionStatus::PublicationPending) => {
                        Some(KeptReason::PublicationPending)
                    }
                    Some(RetentionStatus::Expired) => Some(KeptReason::Expired),
                    Some(RetentionStatus::Published) => None,
                };
                if let Some(reason) = reason {
                    report.kept.push(Kept {
                        path: entry.path,
                        reason,
                    });
                } else if let Some(artifact) = artifact
                    && expired(entry.mtime, hold, self.clock.now())
                {
                    self.expire(run, artifact, report)?;
                }
            }
        }
        Ok(())
    }
}
