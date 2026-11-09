use vpt_domain::{context::ContextSnapshot, time::UtcInstant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContextWindow {
    pub start: UtcInstant,
    pub end: UtcInstant,
}
pub struct ContextRequest<'a> {
    pub window: Option<ContextWindow>,
    pub tasks: bool,
    pub calendars: &'a [String],
    pub projects: &'a [String],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContextConfigError {
    pub key: &'static str,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextError {
    Config(ContextConfigError),
    Interrupted,
    InvalidDocument,
}
impl From<ContextConfigError> for ContextError {
    fn from(error: ContextConfigError) -> Self {
        Self::Config(error)
    }
}
pub trait ContextSource {
    fn collect(&self, request: &ContextRequest<'_>) -> Result<ContextSnapshot, ContextError>;
}
