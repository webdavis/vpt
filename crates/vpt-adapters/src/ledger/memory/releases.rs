use super::*;
use vpt_application::ports::*;
pub(super) fn owns(state: &State, path: &std::path::Path) -> bool {
    state
        .releases
        .iter()
        .any(|r| r.destination == path || r.report_path == path)
}
impl ReleaseLedger for MemoryLedger {
    fn release(&self, sequence: i64) -> Result<Option<ReleaseRecord>, LedgerError> {
        with(self, |s| {
            s.releases.iter().find(|r| r.sequence == sequence).cloned()
        })
    }
    fn release_payload(&self, sequence: i64) -> Result<Option<ReleasePayload>, LedgerError> {
        with(self, |s| {
            s.release_payloads
                .iter()
                .find(|r| r.release.sequence == sequence)
                .cloned()
        })
    }
    fn releases(&self, recording: &RecordingId) -> Result<Vec<i64>, LedgerError> {
        with(self, |s| {
            s.releases
                .iter()
                .rev()
                .filter(|r| r.recording == *recording && r.finalized_at.is_some())
                .map(|r| r.sequence)
                .collect()
        })
    }
    fn pending_releases(&self) -> Result<Vec<i64>, LedgerError> {
        with(self, |s| {
            s.release_payloads
                .iter()
                .map(|p| p.release.sequence)
                .collect()
        })
    }
}
#[cfg(test)]
mod tests {
    macro_rules! case {
        ($name:ident) => {
            #[test]
            fn $name() {
                crate::ledger::contract::releases::$name(&super::MemoryLedger::new());
            }
        };
    }
    crate::ledger::contract::releases::cases!(case);
}
