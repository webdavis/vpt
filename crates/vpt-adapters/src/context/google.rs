use super::Failure;
mod collect;
mod http;
mod page;
mod token;
use vpt_application::{
    GoogleCredentials,
    ports::{ContextConfigError, ContextError, ContextRequest, ContextWindow},
};
use vpt_domain::context::ContextSnapshot;

enum Request<'a> {
    Refresh(&'a GoogleCredentials),
    Events {
        token: &'a str,
        calendar: &'a str,
        window: ContextWindow,
        page: Option<&'a str>,
    },
}
pub struct GoogleContext<'a> {
    pub credentials: &'a GoogleCredentials,
}
impl GoogleContext<'_> {
    pub fn validate(&self, request: &ContextRequest<'_>) -> Result<(), ContextConfigError> {
        collect::validate(self.credentials, request)
    }
}
impl vpt_application::ports::ContextSource for GoogleContext<'_> {
    fn collect(&self, request: &ContextRequest<'_>) -> Result<ContextSnapshot, ContextError> {
        let mut native = http::Native::new();
        collect::run(
            self.credentials,
            request,
            |request, timeout| native.send(request, timeout),
            std::time::Instant::now,
            crate::spawn::interrupted,
        )
    }
}
