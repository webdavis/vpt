mod render;
use crate::{
    digest::Sha256Digest,
    identity::RecordingId,
    occasion::{OccasionId, time::OccasionTime},
    time::{UtcInstant, UtcOffset},
};
pub use render::content;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Released,
    Transcript,
    Analysis,
    Brief,
}
impl Kind {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "released" => Some(Self::Released),
            "transcript" => Some(Self::Transcript),
            "analysis" => Some(Self::Analysis),
            "brief" => Some(Self::Brief),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Released => "released",
            Self::Transcript => "transcript",
            Self::Analysis => "analysis",
            Self::Brief => "brief",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid,
    CapacityExceeded,
}
pub enum Identity<'a> {
    Recording(&'a RecordingId),
    Occasion {
        id: &'a OccasionId,
        sources: &'a [RecordingId],
    },
}
impl Identity<'_> {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Recording(id) => id.as_str(),
            Self::Occasion { id, .. } => id.as_str(),
        }
    }
}
pub struct Provenance<'a> {
    pub captured: Option<(UtcInstant, UtcOffset)>,
    pub occasion_at: Option<&'a OccasionTime>,
    pub engines: &'a [String],
    pub open_flags: u32,
    pub reviewed: bool,
}
pub struct Document<'a> {
    pub kind: Kind,
    pub identity: Identity<'a>,
    pub note: &'a str,
    pub digest: Sha256Digest,
    pub stem: &'a str,
    pub body: &'a str,
    pub provenance: Provenance<'a>,
    pub generated_at: UtcInstant,
}
impl Document<'_> {
    pub fn title(&self) -> String {
        format!("vpt {} {}", self.kind.as_str(), self.stem)
    }
    pub fn validate(&self) -> Result<(), Error> {
        if self.body.len() > 16 * 1024 * 1024
            || self.note.len() > 65_536
            || self.stem.len() > 65_536
            || self.provenance.engines.len() > 65_536
        {
            return Err(Error::CapacityExceeded);
        }
        if self.provenance.reviewed && self.provenance.open_flags != 0 {
            return Err(Error::Invalid);
        }
        if self.stem.is_empty()
            || self.stem.chars().any(char::is_control)
            || self.stem.contains(['/', '\\'])
            || self.note.is_empty()
            || self.note.contains('\0')
        {
            return Err(Error::Invalid);
        }
        match (
            &self.identity,
            self.kind,
            self.provenance.captured,
            self.provenance.occasion_at,
        ) {
            (Identity::Occasion { sources, .. }, Kind::Brief, None, Some(_))
                if sources.len() <= 65_536 =>
            {
                let unique: std::collections::BTreeSet<_> =
                    sources.iter().map(|s| s.as_str()).collect();
                if unique.len() != sources.len() {
                    return Err(Error::Invalid);
                }
            }
            (
                Identity::Recording(_),
                Kind::Released | Kind::Transcript | Kind::Analysis,
                Some((at, offset)),
                None,
            ) => {
                if offset.secs.unsigned_abs() > 86_400
                    || offset.secs % 60 != 0
                    || !(0..=9999).contains(&at.civil(offset).year)
                {
                    return Err(Error::Invalid);
                }
            }
            _ => return Err(Error::Invalid),
        }
        if !(0..=9999).contains(&self.generated_at.civil(UtcOffset { secs: 0 }).year) {
            return Err(Error::Invalid);
        }
        let mut remaining = 1024 * 1024usize;
        for engine in self.provenance.engines {
            if engine.is_empty() || engine.chars().any(char::is_control) {
                return Err(Error::Invalid);
            }
            remaining = remaining
                .checked_sub(engine.len())
                .ok_or(Error::CapacityExceeded)?;
        }
        Ok(())
    }
}
