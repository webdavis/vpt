mod validate;
use super::{ContextSummary, EvidencePack};
use crate::{
    occasion::{OccasionDetails, OccasionId, OccasionSource},
    time::UtcInstant,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocumentError {
    Invalid,
    CapacityExceeded,
}

pub struct Document<'a> {
    pub id: &'a OccasionId,
    pub source: OccasionSource,
    pub occasion: &'a OccasionDetails,
    pub pack: &'a EvidencePack,
    pub context: &'a ContextSummary,
    pub tasks: &'a [super::ContextTask],
    pub generated_at: UtcInstant,
}

#[derive(Clone, PartialEq)]
pub struct Snapshot {
    pub id: OccasionId,
    pub source: OccasionSource,
    pub occasion: OccasionDetails,
    pub pack: EvidencePack,
    pub context: ContextSummary,
    pub tasks: Vec<super::ContextTask>,
    pub generated_at: UtcInstant,
}
impl Snapshot {
    pub fn document(&self) -> Document<'_> {
        Document {
            id: &self.id,
            source: self.source,
            occasion: &self.occasion,
            pack: &self.pack,
            context: &self.context,
            tasks: &self.tasks,
            generated_at: self.generated_at,
        }
    }
}
impl std::fmt::Debug for Snapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BriefSnapshot")
            .field("id", &self.id)
            .field("source", &self.source)
            .finish_non_exhaustive()
    }
}

impl Document<'_> {
    pub fn validate(&self) -> Result<(), DocumentError> {
        validate::validate(self)?;
        let at = self.generated_at.civil(crate::time::UtcOffset { secs: 0 });
        if !(0..=9999).contains(&at.year) {
            return Err(DocumentError::Invalid);
        }
        for item in &self.pack.items {
            if item.text.trim().is_empty()
                || !item.source.at_secs.is_finite()
                || item.source.at_secs < 0.0
                || item.source.at_secs > (1u64 << 53) as f64
            {
                return Err(DocumentError::Invalid);
            }
        }
        Ok(())
    }
}

pub fn source_path(path: &std::path::Path) -> Option<&str> {
    let value = path.to_str()?;
    (path.is_absolute()
        && value.len() <= 65_536
        && !value.contains('\0')
        && !value
            .split('/')
            .skip(1)
            .any(|part| matches!(part, "" | "." | "..")))
    .then_some(value)
}
