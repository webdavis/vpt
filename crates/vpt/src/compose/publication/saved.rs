use std::path::Path;
use vpt_application::{
    RepairError,
    ports::{BriefArtifact, BriefLedger, EngineOutputLedger, NoteLedger},
};

pub(super) struct Saved {
    pub bytes: Option<Vec<u8>>,
    pub brief: Option<BriefArtifact>,
}

pub(super) fn read(
    ledger: &(impl EngineOutputLedger + NoteLedger + BriefLedger),
    target: &Path,
) -> Result<Saved, RepairError> {
    if let Some(output) = ledger.engine_output(target).map_err(RepairError::Ledger)? {
        return Ok(Saved {
            bytes: Some(output.bytes),
            brief: None,
        });
    }
    if let Some(note) = ledger.note_at(target).map_err(RepairError::Ledger)? {
        return Ok(Saved {
            bytes: Some(note.text.into_bytes()),
            brief: None,
        });
    }
    let brief = ledger.brief_at(target).map_err(RepairError::Ledger)?;
    Ok(Saved {
        bytes: brief.as_ref().map(|brief| brief.text.as_bytes().to_vec()),
        brief,
    })
}
