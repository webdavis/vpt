use super::RuntimeLedger;
use vpt_application::ports::{AcceptedProposal, GroundingBasis, LedgerError, ProposalLedger};
use vpt_domain::identity::RecordingId;

impl ProposalLedger for RuntimeLedger {
    fn accepted_proposal(&self, id: &RecordingId) -> Result<Option<AcceptedProposal>, LedgerError> {
        match self {
            Self::Sqlite(ledger) => ledger.accepted_proposal(id),
            Self::Empty(ledger) => ledger.accepted_proposal(id),
        }
    }

    fn grounding_basis(&self, id: &RecordingId) -> Result<Option<GroundingBasis>, LedgerError> {
        match self {
            Self::Sqlite(ledger) => ledger.grounding_basis(id),
            Self::Empty(ledger) => ledger.grounding_basis(id),
        }
    }
}
