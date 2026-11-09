use vpt_application::ports::{LedgerError, RecordingObservation};

pub(in crate::ledger) const MAX_RECORDINGS: usize = 65_536;
const MAX_METADATA: usize = 16 * 1024 * 1024;

pub(in crate::ledger) fn capacity() -> LedgerError {
    LedgerError::Corrupt("recording observation capacity exceeded".into())
}

#[derive(Default)]
pub(in crate::ledger) struct Inventory {
    rows: Vec<RecordingObservation>,
    bytes: usize,
}

impl Inventory {
    pub(in crate::ledger) fn push(&mut self, row: RecordingObservation) -> Result<(), LedgerError> {
        let record = &row.record;
        let mut bytes = std::mem::size_of_val(&row)
            .saturating_add(record.id.as_str().len())
            .saturating_add(record.title.as_ref().map_or(0, String::len))
            .saturating_add(record.audio_path.as_os_str().len())
            .saturating_add(
                record
                    .source_path
                    .as_ref()
                    .map_or(0, |p| p.as_os_str().len()),
            );
        if let Some(transcript) = &row.transcription {
            bytes = bytes
                .saturating_add(transcript.language.len())
                .saturating_add(transcript.requested_language.len());
            for engine in &transcript.engines {
                for size in [
                    std::mem::size_of_val(engine),
                    engine.name.len(),
                    engine.tool.len(),
                    engine.version.len(),
                    engine.model.len(),
                    engine.family.len(),
                ] {
                    bytes = bytes.saturating_add(size);
                }
            }
        }
        for flag in row.flags.iter().chain(&row.diagnostics) {
            bytes = bytes.saturating_add(std::mem::size_of_val(flag));
        }
        for artifact in &row.artifacts {
            bytes = bytes
                .saturating_add(std::mem::size_of_val(artifact))
                .saturating_add(artifact.path.as_os_str().len())
                .saturating_add(artifact.owner.as_str().len());
        }
        if self.rows.len() == MAX_RECORDINGS || bytes > MAX_METADATA.saturating_sub(self.bytes) {
            return Err(capacity());
        }
        self.bytes += bytes;
        self.rows.push(row);
        Ok(())
    }

    pub(in crate::ledger) fn finish(self) -> Vec<RecordingObservation> {
        self.rows
    }
}
