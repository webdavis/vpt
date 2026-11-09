use crate::brief::{ContextSummary, ContextTask};
use crate::occasion::{KeyError, OccasionDetails, OccasionKey, OccasionSource};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ContextSnapshot {
    pub status: ContextSummary,
    pub occasions: Vec<ContextOccasion>,
    pub tasks: Vec<ContextTask>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ContextOccasion {
    pub source: OccasionSource,
    pub id: String,
    pub calendar: Option<String>,
    pub details: OccasionDetails,
}
impl ContextOccasion {
    pub fn key(&self) -> Result<OccasionKey, KeyError> {
        match self.source {
            OccasionSource::Dam => OccasionKey::dam(&self.id),
            OccasionSource::Google => {
                OccasionKey::google(self.calendar.as_deref().ok_or(KeyError::Invalid)?, &self.id)
            }
            OccasionSource::Manual => Err(KeyError::Invalid),
        }
    }
}
impl std::fmt::Debug for ContextOccasion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContextOccasion")
            .field("source", &self.source)
            .finish_non_exhaustive()
    }
}
