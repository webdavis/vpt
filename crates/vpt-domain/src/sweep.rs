use crate::time::{FileTime, UtcInstant};

pub const SF_DATALESS: u32 = 0x4000_0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeferralReason {
    Dataless,
    AudioTooLarge,
    InvalidContainer,
    NotAtRest,
    ChangedDuringRead,
}

impl DeferralReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dataless => "dataless",
            Self::AudioTooLarge => "audio_too_large",
            Self::InvalidContainer => "invalid_container",
            Self::NotAtRest => "not_at_rest",
            Self::ChangedDuringRead => "changed_during_read",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "dataless" => Some(Self::Dataless),
            "audio_too_large" => Some(Self::AudioTooLarge),
            "invalid_container" => Some(Self::InvalidContainer),
            "not_at_rest" => Some(Self::NotAtRest),
            "changed_during_read" => Some(Self::ChangedDuringRead),
            _ => None,
        }
    }
}

pub struct CandidateFacts {
    pub size: u64,
    pub mtime: FileTime,
    pub flags: u32,
}

pub struct SeenFacts {
    pub size: u64,
    pub mtime: FileTime,
    pub ingested: bool,
    pub deferred_size: Option<u64>,
}

pub struct SweepLimits {
    pub max_audio_bytes: u64,
    pub quiet_period_secs: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PreOpen {
    Dataless,
    Unchanged,
    Open,
}

pub fn pre_open(candidate: &CandidateFacts, seen: Option<&SeenFacts>) -> PreOpen {
    if candidate.flags & SF_DATALESS != 0 {
        return PreOpen::Dataless;
    }
    match seen {
        Some(seen)
            if seen.ingested && seen.size == candidate.size && seen.mtime == candidate.mtime =>
        {
            PreOpen::Unchanged
        }
        _ => PreOpen::Open,
    }
}

pub fn size_gate(size: u64, limits: &SweepLimits) -> Result<(), DeferralReason> {
    if size > limits.max_audio_bytes {
        Err(DeferralReason::AudioTooLarge)
    } else {
        Ok(())
    }
}

pub fn rest_gate(
    mtime: FileTime,
    size: u64,
    now: UtcInstant,
    seen: Option<&SeenFacts>,
    limits: &SweepLimits,
) -> Result<(), DeferralReason> {
    let age_nanos =
        (i128::from(now.secs) - i128::from(mtime.secs)) * 1_000_000_000 - i128::from(mtime.nanos);
    if age_nanos < i128::from(limits.quiet_period_secs) * 1_000_000_000 {
        return Err(DeferralReason::NotAtRest);
    }
    if seen
        .and_then(|seen| seen.deferred_size)
        .is_some_and(|deferred_size| deferred_size != size)
    {
        return Err(DeferralReason::NotAtRest);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
