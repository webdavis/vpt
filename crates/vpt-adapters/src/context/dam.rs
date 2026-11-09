use super::Failure;
use vpt_domain::context::ContextSnapshot;
mod collect;
mod decode;
mod plan;
use vpt_application::ports::{ContextConfigError, ContextError, ContextRequest, ContextSource};

pub struct DamContext<'a> {
    pub command: &'a [String],
}
impl DamContext<'_> {
    pub fn validate(&self, request: &ContextRequest<'_>) -> Result<(), ContextConfigError> {
        plan::make(self.command, request).map(|_| ())
    }
}
impl ContextSource for DamContext<'_> {
    fn collect(&self, request: &ContextRequest<'_>) -> Result<ContextSnapshot, ContextError> {
        collect::run(
            self.command,
            request,
            |argv, timeout| crate::spawn::run(argv, &[], timeout, 65_537),
            std::time::Instant::now,
        )
    }
}

#[cfg(test)]
mod boundary_tests;
#[cfg(test)]
mod collection_tests;

#[cfg(test)]
mod tests;
