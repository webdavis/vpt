use vpt_application::ports::{AcceptedProposal, AcceptedTranscript, LedgerError, NoteArtifact};
use vpt_domain::review::ReviewFlag;

pub(in crate::ledger) fn check(
    proposal: Option<&AcceptedProposal>,
    transcript: Option<&AcceptedTranscript>,
    source: Option<&NoteArtifact>,
    flags: &[ReviewFlag],
) -> Result<bool, LedgerError> {
    let (Some(proposal), Some(transcript), Some(source)) = (proposal, transcript, source) else {
        return Ok(false);
    };
    Ok(proposal.input_digest == source.content_digest
        && proposal.basis == super::basis::compute(transcript, flags)?)
}
