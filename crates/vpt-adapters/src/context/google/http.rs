use super::*;
use std::time::Duration;
mod request;
pub(super) struct Native {
    agent: ureq::Agent,
}
impl Native {
    pub fn new() -> Self {
        let agent = ureq::Agent::config_builder()
            .https_only(true)
            .max_redirects(0)
            .proxy(None)
            .http_status_as_error(false)
            .max_response_header_size(65_536)
            .timeout_global(Some(Duration::from_secs(30)))
            .build()
            .into();
        Self { agent }
    }
    pub fn send(&mut self, input: Request<'_>, timeout: Duration) -> Result<Vec<u8>, Failure> {
        if timeout.is_zero() {
            return Err(Failure::Deadline);
        }
        let request = self
            .agent
            .configure_request(request::build(input)?)
            .timeout_global(Some(timeout))
            .build();
        read(self.agent.run(request).map_err(error)?)
    }
}
fn read(response: ureq::http::Response<ureq::Body>) -> Result<Vec<u8>, Failure> {
    if !response.status().is_success() {
        return Err(Failure::HttpStatus);
    }
    let bytes = response
        .into_body()
        .into_with_config()
        .limit(65_537)
        .read_to_vec()
        .map_err(error)?;
    if bytes.len() > 65_536 {
        return Err(Failure::Capacity);
    }
    Ok(bytes)
}
fn error(error: ureq::Error) -> Failure {
    match error {
        ureq::Error::Timeout(_) => Failure::Deadline,
        ureq::Error::BodyExceedsLimit(_) => Failure::Capacity,
        _ => Failure::Network,
    }
}
#[cfg(test)]
mod tests;
