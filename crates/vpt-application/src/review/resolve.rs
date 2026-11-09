use crate::ports::{FlagResolutionChange, ReviewSnapshot};
use vpt_domain::{
    digest::Sha256Digest,
    identity::RecordingId,
    review::{ResolveError, ReviewAction},
    time::UtcInstant,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionError {
    UnknownFlag,
    InvalidSnapshot,
    InvalidTerm,
    Action(ResolveError),
}

pub struct PreparedResolution {
    pub change: Option<FlagResolutionChange>,
    pub known_term: Option<String>,
}
impl std::fmt::Debug for PreparedResolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedResolution")
            .field("change", &self.change)
            .field("adds_known_term", &self.known_term.is_some())
            .finish()
    }
}

pub fn prepare(
    snapshot: &mut ReviewSnapshot,
    recording: &RecordingId,
    identifier: Sha256Digest,
    action: &ReviewAction,
    at: UtcInstant,
) -> Result<PreparedResolution, ResolutionError> {
    let Some(accepted) = &snapshot.accepted else {
        return Err(if snapshot.flags.is_empty() {
            ResolutionError::UnknownFlag
        } else {
            ResolutionError::InvalidSnapshot
        });
    };
    if accepted.recording != *recording
        || snapshot.flags.len() > 65_539
        || snapshot
            .flags
            .iter()
            .filter(|flag| flag.identifier == identifier)
            .count()
            > 1
    {
        return Err(ResolutionError::InvalidSnapshot);
    }
    let flag = snapshot
        .flags
        .iter_mut()
        .find(|flag| flag.identifier == identifier)
        .ok_or(ResolutionError::UnknownFlag)?;
    if !flag.resolution.valid_text() {
        return Err(ResolutionError::InvalidSnapshot);
    }
    let resolution = flag.resolve(action, at).map_err(ResolutionError::Action)?;
    let known_term = term::addition(&flag.content, &resolution)?;
    let change = (flag.resolution != resolution).then(|| FlagResolutionChange {
        recording: recording.clone(),
        flag: identifier,
        expected: flag.resolution.clone(),
        resolution: resolution.clone(),
    });
    flag.resolution = resolution;
    if change.is_some() && matches!(flag.content, vpt_domain::review::FlagContent::Lexical(_)) {
        snapshot.invalidate_verification();
    }
    Ok(PreparedResolution { change, known_term })
}

mod term;
