use super::RetentionReport;
use crate::ports::LedgerError;
use vpt_domain::{
    history::{Event, EventCounts, EventDetail, EventKind, EventState},
    time::UtcInstant,
};

impl RetentionReport {
    pub fn event(&self, at: UtcInstant, failed: bool) -> Result<Option<Event>, LedgerError> {
        if self.moved.is_empty() {
            return Ok(None);
        }
        Ok(Some(Event {
            kind: EventKind::Retention,
            state: if failed {
                EventState::Failed
            } else {
                EventState::Done
            },
            recording: None,
            occasion: None,
            detail: EventDetail::Retention {
                completed: super::report::moved_count(self.moved.len())?,
                failed: u32::from(failed),
            },
            counts: EventCounts {
                retention: Some(self.store_counts()?),
                ..Default::default()
            },
            paths: vec![],
            occurred_at: at,
        }))
    }
}
