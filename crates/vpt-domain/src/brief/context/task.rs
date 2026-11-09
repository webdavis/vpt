use crate::occasion::OccasionSource;

#[derive(Clone, PartialEq, Eq)]
pub struct ContextTask {
    pub source: OccasionSource,
    pub project: String,
    pub id: String,
    pub content: String,
    pub due: Option<String>,
    pub completed: bool,
    pub priority: Option<u32>,
}

impl std::fmt::Debug for ContextTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContextTask")
            .field("source", &self.source)
            .finish_non_exhaustive()
    }
}
