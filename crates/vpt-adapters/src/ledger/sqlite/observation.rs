use super::{SqliteLedger, recordings};
use crate::ledger::observation::inventory::{Inventory, MAX_RECORDINGS, capacity};
use crate::ledger::observation::{artifacts_by_recording, assemble};
use vpt_application::ports::{LedgerError, ObservationLedger, RecordingObservation};
use vpt_domain::identity::RecordingId;

impl ObservationLedger for SqliteLedger {
    fn observe_recordings(
        &self,
        id: Option<&RecordingId>,
    ) -> Result<Vec<RecordingObservation>, LedgerError> {
        self.snapshot(|connection| {
            let mut statement = connection.prepare(
                "SELECT CASE WHEN length(CAST(id AS BLOB))=30 THEN id END FROM recordings WHERE (?1 IS NULL OR id=?1) ORDER BY captured_at, id LIMIT ?2"
            ).map_err(super::map)?;
            let ids = statement.query_map(rusqlite::params![id.map(RecordingId::as_str), (MAX_RECORDINGS + 1) as i64], |row| row.get::<_, String>(0)).map_err(super::map)?;
            let ids = ids.collect::<Result<Vec<_>, _>>().map_err(super::map)?;
            if ids.len() > MAX_RECORDINGS {
                return Err(capacity());
            }
            let records = ids;
            if records.is_empty() {
                return Ok(Vec::new());
            }
            let mut artifacts =
                artifacts_by_recording(super::retention_artifacts::read(connection)?);
            let mut observed = Inventory::default();
            for id in records {
                let Some(record) = recordings::one_record(connection, "id", &id)? else {
                    continue;
                };
                let review = super::verification::state::review(connection, &record.id)?;
                let owned = artifacts.remove(record.id.as_str()).unwrap_or_default();
                observed.push(assemble(record, review, owned))?;
            }
            Ok(observed.finish())
        })
    }
}
