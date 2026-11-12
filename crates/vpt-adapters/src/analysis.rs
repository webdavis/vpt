use crate::{
    proposal,
    spawn::{self, SpawnError, Status},
};
use sha2::{Digest, Sha256};
use vpt_application::ports::{AgentCommand, AnalysisFailure, AnalysisOutput, AnalysisRequest};
use vpt_domain::digest::Sha256Digest;
mod arguments;

const OUTPUT_LIMIT: usize = 1_048_576;

pub struct CommandAgent;

impl AgentCommand for CommandAgent {
    fn analyze(&self, request: &AnalysisRequest<'_>) -> Result<AnalysisOutput, AnalysisFailure> {
        let argv = arguments::argv(request)?;
        if request.content.len() > vpt_domain::note::MAX_RENDERED {
            return Err(AnalysisFailure::InputLimit);
        }
        let outcome = spawn::run(
            &argv,
            request.content.as_bytes(),
            request.settings.timeout,
            OUTPUT_LIMIT + 1,
        )
        .map_err(|error| match error {
            SpawnError::NotFound(_) => AnalysisFailure::Unavailable,
            SpawnError::Io(_) => AnalysisFailure::Io,
        })?;
        match outcome.status {
            Status::Exited(0) => (),
            Status::Exited(code) => return Err(AnalysisFailure::Exited(code)),
            Status::Signaled(signal) => return Err(AnalysisFailure::Signaled(signal)),
            Status::DeadlineExceeded => return Err(AnalysisFailure::DeadlineExceeded),
            Status::Interrupted => return Err(AnalysisFailure::Interrupted),
        }
        if outcome.stdout.len() > OUTPUT_LIMIT {
            return Err(AnalysisFailure::OutputLimit);
        }
        let decoded = proposal::decode(&outcome.stdout).map_err(|error| match error {
            proposal::ProposalDecodeError::UnsupportedVersion(version) => {
                AnalysisFailure::UnsupportedVersion(version)
            }
            _ => AnalysisFailure::InvalidDocument,
        })?;
        Ok(AnalysisOutput {
            proposal: decoded.proposal,
            diagnostics: decoded.diagnostics,
            input_digest: Sha256Digest(Sha256::digest(request.content.as_bytes()).into()),
        })
    }
}

#[cfg(test)]
mod tests;
