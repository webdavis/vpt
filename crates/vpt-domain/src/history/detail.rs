use super::{Event, SafeFailureClass};
use crate::sweep::DeferralReason;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventDetail {
    None,
    Brief {
        notes: u32,
        spans: u32,
        tasks: u32,
        unresolved: u32,
    },
    ReviewAggregate {
        recordings: u32,
    },
    Review {
        spans: u32,
        agreed_unverified: u32,
    },
    Failure(SafeFailureClass),
    Deferred {
        reason: DeferralReason,
        count: u32,
    },
    Retention {
        completed: u32,
        failed: u32,
    },
    Actions {
        sent: u32,
        failed: u32,
        refused: u32,
        withdrawn: Option<u32>,
    },
}

impl Event {
    pub fn detail_text(&self) -> String {
        match self.detail {
            EventDetail::None => String::new(),
            EventDetail::Brief {
                notes,
                spans,
                tasks,
                unresolved,
            } => format!(
                "{notes} notes, {spans} spans, {tasks} tasks, {unresolved} unresolved flags"
            ),
            EventDetail::ReviewAggregate { recordings } => {
                format!("{recordings} recordings need review")
            }
            EventDetail::Review {
                spans,
                agreed_unverified,
            } => {
                format!("{spans} spans to review, {agreed_unverified} agreed spans unverified")
            }
            EventDetail::Failure(class) => class.as_str().to_owned(),
            EventDetail::Deferred { reason, count } => {
                format!("deferred {count} times: {}", reason.as_str())
            }
            EventDetail::Retention { completed, failed } => {
                format!("{completed} retention items completed, {failed} failed")
            }
            EventDetail::Actions {
                sent,
                failed,
                refused,
                withdrawn,
            } => {
                let withdrawn = withdrawn.map_or_else(String::new, |n| format!(", {n} withdrawn"));
                format!("{sent} sent, {failed} failed, {refused} refused{withdrawn}")
            }
        }
    }
}
