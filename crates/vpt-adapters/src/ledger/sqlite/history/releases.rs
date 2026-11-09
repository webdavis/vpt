use super::*;
use vpt_application::ports::{NewRelease, PreparedRelease, ReleaseHistory, ReleaseVersion};
mod complete;
mod prepare;
#[cfg(test)]
pub(in crate::ledger::sqlite) mod tests;
impl ReleaseHistory for SqliteLedger {
    fn prepare_release(&self, run: RunId, draft: &NewRelease) -> HistoryWrite<PreparedRelease> {
        self.history_write(|c| prepare::apply(c, run, draft))
    }
    fn complete_release(
        &self,
        run: RunId,
        sequence: i64,
        expected: ReleaseVersion,
        at: UtcInstant,
    ) -> HistoryWrite<HistoryReceipt> {
        self.history_write(|c| complete::apply(c, run, sequence, expected, at))
    }
}
