use super::{IngestFailure, candidate::recorder_failure};
use crate::SourceSettings;
use crate::ports::{Candidate, RecorderStore, SeenRow, SourceMetadata};
use vpt_domain::container::{Container, inspect};
use vpt_domain::sweep::{
    CandidateFacts, DeferralReason, PreOpen, SeenFacts, SweepLimits, pre_open, rest_gate, size_gate,
};
use vpt_domain::time::UtcInstant;

pub(super) fn previous(row: &SeenRow) -> SeenFacts {
    SeenFacts {
        size: row.size,
        mtime: row.mtime,
        ingested: row.recording.is_some() && row.deferral_reason.is_none(),
        deferred_size: row.deferred_size,
    }
}

pub(super) fn selection(candidate: &Candidate, seen: Option<&SeenRow>) -> PreOpen {
    pre_open(
        &CandidateFacts {
            size: candidate.size,
            mtime: candidate.mtime,
            flags: candidate.flags,
        },
        seen.map(previous).as_ref(),
    )
}

pub(super) struct Eligible<H> {
    pub handle: H,
    pub metadata: SourceMetadata,
    pub container: Container,
}

pub(super) fn source<R: RecorderStore>(
    recorder: &R,
    candidate: &Candidate,
    seen: Option<&SeenRow>,
    now: UtcInstant,
    settings: &SourceSettings,
) -> Result<Result<Eligible<R::Handle>, DeferralReason>, IngestFailure> {
    let handle = recorder.open(&candidate.path).map_err(recorder_failure)?;
    let metadata = recorder.metadata(&handle).map_err(recorder_failure)?;
    let limits = SweepLimits {
        max_audio_bytes: settings.max_audio_bytes,
        quiet_period_secs: settings.quiet_period_secs,
    };
    if let Err(reason) = size_gate(metadata.size, &limits) {
        return Ok(Err(reason));
    }
    let Ok(container) = inspect(metadata.size, |offset, buffer| {
        recorder.read_at(&handle, offset, buffer)
    }) else {
        return Ok(Err(DeferralReason::InvalidContainer));
    };
    if let Err(reason) = rest_gate(
        metadata.mtime,
        metadata.size,
        now,
        seen.map(previous).as_ref(),
        &limits,
    ) {
        return Ok(Err(reason));
    }
    Ok(Ok(Eligible {
        handle,
        metadata,
        container,
    }))
}
