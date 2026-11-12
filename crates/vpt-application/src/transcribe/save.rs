use super::{EngineSlot, Transcription};
use crate::ports::{
    AcceptedTranscript, DirtyPublication, EngineArtifact, EngineOutputLedger, LedgerError,
    StoreError, Stores, TranscriptionChange, WorkDelta,
};
use std::path::{Path, PathBuf};
use vpt_domain::{identity::RecordingId, time::UtcInstant};

#[derive(Debug)]
pub enum SaveError {
    Ledger(LedgerError),
    Store(StoreError),
    Modified(PathBuf),
}

pub struct AcceptedInput<'a> {
    pub recording: &'a RecordingId,
    pub duration: f64,
    pub at: UtcInstant,
    pub root: &'a Path,
    pub uncertainty: vpt_domain::uncertainty::Classification,
}

pub fn prepare(
    result: Transcription,
    input: AcceptedInput<'_>,
    ledger: &impl EngineOutputLedger,
    stores: &impl Stores,
) -> Result<WorkDelta, SaveError> {
    let AcceptedInput {
        recording,
        duration,
        at,
        root,
        uncertainty,
    } = input;
    let accepted = AcceptedTranscript {
        checker_engine: result
            .checker
            .as_ref()
            .map(|output| output.transcript.engine.clone()),
        recording: recording.clone(),
        transcript: vpt_domain::transcript::label_speakers(
            &result.record.transcript,
            result.checker.as_ref().map(|output| &output.transcript),
        ),
        requested_language: result.language,
        duration,
        accepted_at: at,
        lexical_classification: result.lexical_classification,
        from_checker: result.record_slot == EngineSlot::Checker,
        uncertainty,
    };
    let mut delta = WorkDelta {
        transcription: Some(TranscriptionChange::Accept(Box::new(accepted))),
        ..Default::default()
    };
    for output in std::iter::once(result.record).chain(result.checker) {
        let target = root.join(format!(
            "{}.{}.json",
            recording, output.transcript.engine.name
        ));
        stores.validate_path(&target).map_err(SaveError::Store)?;
        let previous = ledger
            .engine_output(&target)
            .map_err(SaveError::Ledger)?
            .map(|saved| saved.digest);
        if stores.digest_of(&target).map_err(SaveError::Store)? != previous {
            return Err(SaveError::Modified(target));
        }
        let digest = stores.digest_bytes(&output.raw_document);
        delta.commit.publications.push(DirtyPublication {
            target: target.clone(),
            expected_previous: previous,
            intended: digest,
            recorded_at: at,
        });
        delta.engine_outputs.push(EngineArtifact {
            recording: recording.clone(),
            engine: output.transcript.engine.name,
            target,
            digest,
            duration,
            language_source: output.transcript.language_source,
            bytes: output.raw_document,
            created_at: at,
        });
    }
    Ok(delta)
}
