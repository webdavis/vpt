use super::{HistoryReceipt, HistoryWrite, NewIntent};
use vpt_domain::history::{EventId, RunId};
use vpt_domain::time::UtcInstant;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedRetention {
    pub intent_id: i64,
    pub effect_id: EventId,
    pub receipt: HistoryReceipt,
}

pub trait RetentionHistory {
    fn prepare_retention(&self, run: RunId, intent: &NewIntent) -> HistoryWrite<PreparedRetention>;

    fn complete_retention(
        &self,
        run: RunId,
        intent_id: i64,
        effect_id: EventId,
        at: UtcInstant,
    ) -> HistoryWrite<HistoryReceipt>;
}
