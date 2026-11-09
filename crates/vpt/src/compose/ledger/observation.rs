use super::RuntimeLedger;
use vpt_application::ports::{LedgerError, ObservationLedger, RecordingObservation};
use vpt_domain::identity::RecordingId;

impl ObservationLedger for RuntimeLedger {
    fn observe_recordings(
        &self,
        id: Option<&RecordingId>,
    ) -> Result<Vec<RecordingObservation>, LedgerError> {
        match self {
            Self::Sqlite(ledger) => ledger.observe_recordings(id),
            Self::Empty(ledger) => ledger.observe_recordings(id),
        }
    }
}
