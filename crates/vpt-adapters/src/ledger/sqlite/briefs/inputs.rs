use super::super::{SqliteLedger, map, recordings};
use vpt_application::ports::*;
use vpt_domain::identity::RecordingId;
impl BriefInputsLedger for SqliteLedger {
    fn visit_brief_inputs(
        &self,
        visitor: &mut dyn FnMut(BriefInput) -> Result<(), LedgerError>,
    ) -> Result<(), LedgerError> {
        self.snapshot(|connection| {
            let mut query = connection
                .prepare("SELECT id FROM recordings ORDER BY captured_at,id LIMIT 65537")
                .map_err(map)?;
            let mut rows = query.query([]).map_err(map)?;
            let mut count = 0;
            while let Some(row) = rows.next().map_err(map)? {
                count += 1;
                if count > 65_536 {
                    return Err(LedgerError::Corrupt("brief inputs exceed capacity".into()));
                }
                let id = RecordingId::parse(
                    row.get_ref(0)
                        .map_err(map)?
                        .as_str()
                        .map_err(|_| crate::ledger::brief::invalid())?,
                )
                .map_err(|_| crate::ledger::brief::invalid())?;
                let recording = recordings::one_record(connection, "id", id.as_str())?
                    .ok_or_else(crate::ledger::brief::invalid)?;
                let note = if recording.stages.note == StageState::Succeeded {
                    super::super::notes::read::recording(connection, &id, NoteStage::Transcript)?
                } else {
                    None
                };
                let review = if note.is_some() {
                    super::super::verification::state::review(connection, &id)?
                } else {
                    ReviewSnapshot::default()
                };
                let relations = super::super::relations::read(connection, &id)?;
                let tags = super::super::tags::read(connection, &id)?.unwrap_or_default();
                visitor(BriefInput {
                    recording,
                    note,
                    review,
                    tags,
                    relations,
                })?;
            }
            Ok(())
        })
    }
}
