use crate::{
    proposal,
    spawn::{self, SpawnError, Status},
};
use sha2::{Digest, Sha256};
use vpt_application::ports::{AgentCommand, SynthesisFailure, SynthesisOutput, SynthesisRequest};
use vpt_domain::digest::Sha256Digest;
mod arguments;

const OUTPUT_LIMIT: usize = 1_048_576;

pub struct CommandAgent;

impl AgentCommand for CommandAgent {
    fn synthesize(
        &self,
        request: &SynthesisRequest<'_>,
    ) -> Result<SynthesisOutput, SynthesisFailure> {
        let argv = arguments::argv(request)?;
        if request.content.len() > vpt_domain::note::MAX_RENDERED {
            return Err(SynthesisFailure::InputLimit);
        }
        let outcome = spawn::run(
            &argv,
            request.content.as_bytes(),
            request.settings.timeout,
            OUTPUT_LIMIT + 1,
        )
        .map_err(|error| match error {
            SpawnError::NotFound(_) => SynthesisFailure::Unavailable,
            SpawnError::Io(_) => SynthesisFailure::Io,
        })?;
        match outcome.status {
            Status::Exited(0) => (),
            Status::Exited(code) => return Err(SynthesisFailure::Exited(code)),
            Status::Signaled(signal) => return Err(SynthesisFailure::Signaled(signal)),
            Status::DeadlineExceeded => return Err(SynthesisFailure::DeadlineExceeded),
            Status::Interrupted => return Err(SynthesisFailure::Interrupted),
        }
        if outcome.stdout.len() > OUTPUT_LIMIT {
            return Err(SynthesisFailure::OutputLimit);
        }
        let decoded = proposal::decode(&outcome.stdout).map_err(|error| match error {
            proposal::ProposalDecodeError::UnsupportedVersion(version) => {
                SynthesisFailure::UnsupportedVersion(version)
            }
            _ => SynthesisFailure::InvalidDocument,
        })?;
        Ok(SynthesisOutput {
            proposal: decoded.proposal,
            diagnostics: decoded.diagnostics,
            input_digest: Sha256Digest(Sha256::digest(request.content.as_bytes()).into()),
        })
    }
}

#[cfg(test)]
mod tests;
